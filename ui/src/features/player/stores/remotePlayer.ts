/**
 * Remote player store — manages frontend browser-native audio playback
 * for remote tracks (YouTube, SoundCloud, etc.) via HTMLAudioElement.
 *
 * This is the browser-side companion to the Rust proxy server.
 * When Rust emits `stream-resolved`, the frontend loads the proxied URL
 * into an HTMLAudio element and drives play/pause/seek natively.
 *
 * For YouTube tracks, the frontend calls `cache_remote_stream` to download
 * the stream to a local file for instant seeking. The local file is loaded
 * via Tauri's `convertFileSrc` (asset:// protocol) which provides direct local
 * file access — no proxy round-trip, instant byte-range seeks. SoundCloud
 * stays on the remote proxy path (its seek already works fine over HTTP
 * Range requests).
 *
 * Local tracks still use the Rust Symphonia/cpal pipeline.
 */

import { writable, get } from 'svelte/store';
import { progress, isPlaying, currentTrack, volume, nextTrack, normalizeAudio } from './player';
import { skipToNext } from './player';
import { notifications } from '@shared/stores/notifications';
import { t } from '@i18n';
import { cacheRemoteStream } from '@services/commands';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { Track } from '@shared/types/models';
import { Source } from '@shared/types/models';

/** The underlying HTMLAudio element for remote playback. */
let audioEl: HTMLAudioElement | null = null;

/** Whether normalization is currently active (read-only flag for UI). */
let normalizationActive = false;

/** The current stream URL - stored for diagnostics. Survives HMR. */
let currentStreamUrl = '';

/** The original proxied stream URL. Kept as fallback if local-cache download fails. */
let baseStreamUrl = '';

/** The source of the current track (YouTube, SoundCloud, Local). */
let currentSource = '';

/** Known duration of the current track (from yt-dlp metadata), used as fallback
 *  when HTMLAudioElement.duration returns Infinity (common with YouTube m4a). */
let trackDuration = 0;

/** Absolute offset represented by the start of the currently loaded stream.
 *  Always 0 now — YouTube seek uses native `currentTime` again, so the element's
 *  currentTime is always absolute. Kept for the timeupdate handler so a future
 *  partial-stream approach can be reintroduced without touching that handler. */
let streamOffset = 0;

/** Whether a seek is in progress — suppresses error handling for aborts. */
let seeking = false;

/** Whether audio was playing before the seek started — used to resume after seek. */
let wasPlayingBeforeSeek = false;

/** Whether a source swap (cache → local file) is in progress — suppresses errors. */
let swappingSource = false;

/** Monotonic id for seek attempts — prevents stale callbacks from older seeks
 *  from mutating state during a newer seek. */
let seekToken = 0;

/** Whether a remote track is currently loaded. */
export const remoteActive = writable(false);

/** Whether a YouTube local-cache download is in progress. */
export const cachingStream = writable(false);

/** Add a local-proxy query parameter without mutating the encoded upstream URL. */
function appendProxyParam(url: string, key: string, value: string): string {
  const separator = url.includes('?') ? '&' : '?';
  return `${url}${separator}${encodeURIComponent(key)}=${encodeURIComponent(value)}`;
}

/** Build a loadable URL for a local file path via Tauri's asset protocol.
 *
 * Using `convertFileSrc` produces a `http://asset.localhost/` URL that the
 * Tauri WebView can access directly — no proxy round-trip, no HTTP/2 chunked
 * read issues, and instant byte-range seeks because the file is local. */
function localFileUrl(localPath: string): string {
  return convertFileSrc(localPath);
}

/** Create or reuse an HTMLAudio element. */
function getAudio(): HTMLAudioElement {
  if (!audioEl) {
    audioEl = new Audio();
    audioEl.preload = 'auto';
    // Do NOT set crossOrigin — it forces CORS on ALL sources including local
    // asset:// files via convertFileSrc. When combined with Web Audio API's
    // createMediaElementSource, non-CORS-compliant sources produce silent output.

    // Sync timeupdate → progress store, and drive MSE segment prefetching
    audioEl.addEventListener('timeupdate', () => {
      const el = audioEl;
      if (!el) return;

      const dur = el.duration;
      // YouTube m4a streams report Infinity for duration. Use the track's
      // known duration from yt-dlp metadata as a reliable fallback.
      const safeDuration = Number.isFinite(dur) && dur > 0 ? dur : trackDuration;
      const safePosition = streamOffset + (Number.isFinite(el.currentTime) ? el.currentTime : 0);
      progress.set({ position: safePosition, duration: safeDuration });
    });

    // Sync play → isPlaying store
    audioEl.addEventListener('play', () => {
      isPlaying.set(true);
    });

    // Sync pause → isPlaying store (but not during seek — browser pauses
    // briefly while fetching the new Range, then resumes automatically)
    audioEl.addEventListener('pause', () => {
      if (!seeking) {
        isPlaying.set(false);
      }
    });

    // Sync ended → advance to next track in queue
    audioEl.addEventListener('ended', () => {
      isPlaying.set(false);
      remoteActive.set(false);
      // Remote tracks don't have a Rust decoder thread to detect EOF,
      // so we must advance the queue from the frontend.
      nextTrack();
    });

    // Error handling
    audioEl.addEventListener('error', (e) => {
      const target = e.target as HTMLAudioElement;
      const errorCode = target.error?.code;

      // Suppress ALL errors during seek or source swap — the browser fires
      // error events when we change audio.src, and calling skipToNext() would
      // cause an infinite loop of errors.
      if (seeking || swappingSource) {
        return;
      }

      const translate = get(t);
      let message = translate('playback.error_title', { default: 'Remote playback failed' });
      switch (errorCode) {
        case MediaError.MEDIA_ERR_ABORTED:
          message = translate('playback.aborted', { default: 'Playback aborted' });
          break;
        case MediaError.MEDIA_ERR_NETWORK:
          message = translate('playback.network_error', { default: 'Network error during playback' });
          break;
        case MediaError.MEDIA_ERR_DECODE:
          message = translate('playback.decode_error', { default: 'Audio decoding error' });
          break;
        case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
          message = translate('playback.format_not_supported', { default: 'Audio format not supported' });
          break;
      }
      notifications.push({ type: 'error', title: translate('playback.error_title', { default: 'Playback Error' }), message, dismissible: true });
      isPlaying.set(false);
      remoteActive.set(false);
      // Auto-advance to the next track in the queue instead of stopping
      skipToNext();
    });
  }
  return audioEl;
}

/**
 * Load and play a remote stream URL.
 *
 * Called when the `stream-resolved` event arrives from Rust.
 * Stops any existing remote playback first.
 *
 * For YouTube tracks, this calls `cache_remote_stream` to download the
 * stream to a local file for instant seeking. While the download is in
 * progress, playback starts from the remote proxy URL (so the user hears
 * audio immediately). Once the download completes, the audio source is
 * swapped to the local file URL for instant seek. If the download fails,
 * playback continues on the remote proxy URL.
 *
 * SoundCloud tracks use the remote proxy URL directly (their seek works).
 */
export async function loadRemoteStream(track: Track, streamUrl: string, remoteUrl?: string): Promise<void> {
  const audio = getAudio();

  // Store the track's known duration from yt-dlp metadata as fallback.
  // YouTube m4a streams report Infinity for audioEl.duration.
  trackDuration = track.duration ?? 0;
  currentSource = track.source;
  streamOffset = 0;

  // Stop any current playback, including any prior MSE session.
  audio.pause();
  audio.src = '';

  // Set volume (0-1)
  audio.volume = get(volume) / 100;
  // Give the proxy the known metadata duration so it can expose
  // X-Content-Duration / Content-Duration on the initial media response. This
  // is especially important for YouTube m4a streams, where the WebView often
  // reports `duration = Infinity`; without a finite duration, native seek can
  // stall or scan slowly.
  const playableUrl = trackDuration > 0
    ? appendProxyParam(streamUrl, 'duration', String(trackDuration))
    : streamUrl;

  baseStreamUrl = playableUrl;
  currentStreamUrl = playableUrl;

  // All remote tracks use direct audio.src via the proxy. Seeking is native
  // (currentTime) for both YouTube and SoundCloud; the proxy exposes byte-range
  // metadata so the browser's media engine can seek accurately.
  audio.src = playableUrl;

  // Start playback from the remote proxy URL immediately.
  // For YouTube, we'll swap to a local file once the cache download completes.
  try {
    await audio.play();
    remoteActive.set(true);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
    remoteActive.set(false);
    return; // Don't attempt cache download if playback failed
  }

  // YouTube local-cache: download the stream to a local file for instant seeking.
  // SoundCloud stays on the remote proxy (its seek works fine over HTTP Range).
  // Only cache tracks shorter than 15 minutes — longer tracks produce files
  // >15MB that take too long to download and can fail with body read errors.
  //
  // Audio normalization is now handled in the backend: when the setting is ON,
  // cache_remote_stream runs ffmpeg loudnorm (EBU R128, -14 LUFS) on the
  // downloaded file before caching. The frontend always swaps to the local
  // cache file regardless of the normalization setting — the cached file is
  // already normalized (or raw) as appropriate.
  const MAX_CACHE_DURATION_SEC = 15 * 60;
  if (track.source === Source.YouTube && remoteUrl && trackDuration > 0 && trackDuration <= MAX_CACHE_DURATION_SEC) {
    cachingStream.set(true);
    try {
      const localPath = await cacheRemoteStream(track.sourceId, remoteUrl);
      // Swap the audio source to the local file for instant seeking.
      // Only swap if the track hasn't changed while we were downloading.
      if (currentStreamUrl === playableUrl && currentSource === Source.YouTube) {
        const localUrl = localFileUrl(localPath);
        // Preserve current playback position across the source swap.
        // The browser must load the new source's metadata before currentTime
        // can be set — setting it immediately after src change is a no-op.
        const currentPosition = audio.currentTime;
        const wasPlaying = !audio.paused;

        swappingSource = true;

        // Revert to the proxy URL if the local file fails to load.
        // This handles: corrupt file, unsupported codec, asset protocol error.
        const revertToProxy = () => {
          if (!swappingSource) return; // already handled
          swappingSource = false;
          audio.removeEventListener('loadedmetadata', onLoadedMetadata);
          audio.removeEventListener('error', onSwapError);
          // Restore the proxy URL and playback position.
          audio.src = playableUrl;
          currentStreamUrl = playableUrl;
          const restorePos = () => {
            audio.currentTime = currentPosition;
            if (wasPlaying) audio.play().catch(() => {});
          };
          audio.addEventListener('loadedmetadata', restorePos, { once: true });
          notifications.push({
            type: 'warning',
            title: 'Usando transmisión remota',
            message: 'No se pudo usar la caché local; el seek puede ser más lento.',
            dismissible: true,
          });
        };

        const onLoadedMetadata = () => {
          audio.currentTime = currentPosition;
          swappingSource = false;
          if (wasPlaying) {
            audio.play().catch(() => {
              // Play failed after swap — revert to proxy.
              revertToProxy();
            });
          }
        };

        const onSwapError = () => {
          // Audio element error loading local file — revert to proxy.
          revertToProxy();
        };

        audio.addEventListener('loadedmetadata', onLoadedMetadata, { once: true });
        audio.addEventListener('error', onSwapError, { once: true });
        // Fallback: if loadedmetadata doesn't fire within 3s, revert to proxy.
        setTimeout(() => {
          if (swappingSource) {
            revertToProxy();
          }
        }, 3000);
        audio.src = localUrl;
        currentStreamUrl = localUrl;
      }
    } catch (e) {
      // Cache download failed (backend validation rejected the file, network
      // error, etc.) — continue playing from the remote proxy URL.
      // This is not a playback error; seek just won't be as fast.
      swappingSource = false;
    } finally {
      cachingStream.set(false);
    }
  }
}

/** Pause remote playback. */
export function pauseRemote(): void {
  if (audioEl) {
    audioEl.pause();
  }
}

/** Resume remote playback. */
export function resumeRemote(): void {
  if (audioEl) {
    audioEl.play().catch((e) => {
      const msg = e instanceof Error ? e.message : String(e);
      notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
    });
  }
}

/** Seek to a position in seconds.
 *
 * Both YouTube and SoundCloud use native `audio.currentTime = position`. The
 * local proxy advertises `Accept-Ranges: bytes` and forwards upstream
 * `Content-Range`/`Content-Length`/206 so the browser's media engine can issue
 * accurate byte-range requests on its own. SoundCloud MP3 seeking already
 * worked; YouTube m4a seeks instantly once the local cache file is loaded
 * (see `cache_remote_stream`). */
export function seekRemote(position: number): void {
  const el = audioEl;
  if (!el) return;

  const token = ++seekToken;
  const shouldResume = !el.paused || get(isPlaying);

  wasPlayingBeforeSeek = shouldResume;
  seeking = true;

  streamOffset = 0;
  el.currentTime = position;

  const resumeAfterSeek = async () => {
    if (token !== seekToken) return;

    if (shouldResume) {
      try {
        await el.play();
      } catch {
        // Ignore transient play failures while the browser is still fetching
        // the target Range. The fallback below will retry once buffering catches up.
      }
    }

    seeking = false;
    wasPlayingBeforeSeek = false;
  };

  // After setting currentTime, the browser may fire `seeked`, `canplay`, or
  // `playing` depending on how quickly the Range request is satisfied. Listen
  // to all three and guard with seekToken so old seeks cannot stop new ones.
  const onReady = () => {
    el.removeEventListener('seeked', onReady);
    el.removeEventListener('canplay', onReady);
    el.removeEventListener('playing', onReady);
    void resumeAfterSeek();
  };

  el.addEventListener('seeked', onReady);
  el.addEventListener('canplay', onReady);
  el.addEventListener('playing', onReady);

  // Kick playback immediately. For YouTube m4a Range seeks the element can
  // remain paused until play() is requested again, even if the target is valid.
  if (shouldResume) {
    el.play().catch(() => {});
  }

  // Fallback: if no readiness event fires (Infinity duration edge cases), retry
  // once after the browser has had a moment to issue the Range request.
  setTimeout(() => {
    if (token !== seekToken) return;

    el.removeEventListener('seeked', onReady);
    el.removeEventListener('canplay', onReady);
    el.removeEventListener('playing', onReady);

    if (shouldResume) {
      el.play().catch(() => {});
    }
    seeking = false;
    wasPlayingBeforeSeek = false;
  }, 1000);
}


/** Set volume for remote playback (0-100). */
export function setRemoteVolume(value: number): void {
  if (audioEl) {
    audioEl.volume = Math.max(0, Math.min(1, value / 100));
  }
}

/** Initialize the Web Audio API chain for normalization.
 *
 *  DEPRECATED: Web Audio API is no longer used for normalization.
 *  createMediaElementSource silences non-CORS-compliant sources (asset://
 *  URLs from convertFileSrc), which broke the cache swap for instant seeking.
 *  Normalization is now handled in the backend via ffmpeg loudnorm during
 *  cache download. These functions remain as no-ops for API compatibility.
 */
function initWebAudioChain(): void {
  // No-op: kept for API compatibility.
}

/** Enable audio normalization for remote playback.
 *
 *  No-op: normalization is applied during cache download in the backend.
 */
export function enableRemoteNormalization(): void {
  // No-op: normalization is applied during cache download in the backend.
}

/** Disable audio normalization for remote playback.
 *
 *  No-op: normalization is applied during cache download in the backend.
 */
export function disableRemoteNormalization(): void {
  // No-op: normalization is applied during cache download in the backend.
}

/** Set normalization state for remote playback.
 *  No-op now: the setting is persisted and applied on next cache download. */
export function setRemoteNormalization(_enabled: boolean): void {
  // No-op: normalization is applied during cache download in the backend.
}

/** Stop and cleanup remote playback. */
export function stopRemote(): void {
  if (audioEl) {
    audioEl.pause();
    audioEl.src = '';
    audioEl.load();
  }
  normalizationActive = false;
  remoteActive.set(false);
}

/** Get the current HTMLAudio element (for advanced use). */
export function getAudioElement(): HTMLAudioElement | null {
  return audioEl;
}