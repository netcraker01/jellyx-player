/**
 * Remote player store — manages frontend browser-native audio playback
 * for remote tracks (YouTube, SoundCloud, etc.) via HTMLAudioElement.
 *
 * This is the browser-side companion to the Rust proxy server.
 * When Rust emits `stream-resolved`, the frontend loads the proxied URL
 * into an HTMLAudio element and drives play/pause/seek natively.
 *
 * For YouTube tracks, the frontend calls `cache_remote_stream` to download
 * the stream to a local file for instant seeking. The local file is routed
 * back through the proxy (`file://` via the proxy) instead of
 * `convertFileSrc` (asset://) so the Web Audio AnalyserNode stays
 * CORS-un-tainted and keeps producing real frequency data for Modo Cine.
 * SoundCloud stays on the remote proxy path (its seek already works fine
 * over HTTP Range requests).
 *
 * A Web Audio AnalyserNode is bound to the audio element once and kept
 * alive across tracks; a rAF loop publishes FrequencyData to the same
 * `frequencyData` store the local Rust FFT path uses, so the visualizer
 * works uniformly regardless of source.
 *
 * Local tracks still use the Rust Symphonia/cpal pipeline (and the Rust
 * FFT engine), which is untouched here.
 */

import { writable, get } from 'svelte/store';
import { progress, isPlaying, currentTrack, volume, nextTrack, normalizeAudio, frequencyData } from './player';
import { skipToNext } from './player';
import { notifications } from '@shared/stores/notifications';
import { t } from '@i18n';
import { cacheRemoteStream } from '@services/commands';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { Track, FrequencyData } from '@shared/types/models';
import { Source } from '@shared/types/models';

/** The underlying HTMLAudio element for remote playback. */
let audioEl: HTMLAudioElement | null = null;

// ── Remote Web Audio FFT (AnalyserNode) ─────────────────────────────
// Remote tracks (YouTube, SoundCloud) bypass the Rust Symphonia/cpal
// pipeline and therefore have no Rust FFT. We attach a Web Audio API
// AnalyserNode to the HTMLAudioElement and run a requestAnimationFrame
// loop that publishes FrequencyData to the same `frequencyData` store
// the local path uses, so Modo Cine / the visualizer works uniformly.
//
// CORS note (the lesson from the earlier rollback):
// `createMediaElementSource` permanently "taints" the element if the
// source is not CORS-compliant, after which getFloatFrequencyData
// returns all zeros. The local proxy already responds with
// `Access-Control-Allow-Origin: *` on both the remote-forward path and
// the `file://` local-cache path. To keep analysis working across the
// YouTube local-cache swap, we set `audioEl.crossOrigin = 'anonymous'`
// up front and route cached local files through the proxy (`file://`
// via the proxy) instead of `convertFileSrc` (asset://), which is not
// CORS-compliant. This is a minimal, opt-in change: local playback
// (Symphonia/cpal) is untouched.

/** Web Audio context kept alive across tracks (never closed on stop). */
let audioCtx: AudioContext | null = null;
/** Media element source bound to audioEl. Created ONCE per element lifetime. */
let mediaSource: MediaElementAudioSourceNode | null = null;
/** Gain node for remote playback volume/mute when routed through Web Audio. */
let gainNode: GainNode | null = null;
/** Analyser node used for frequency-bin extraction. */
let analyser: AnalyserNode | null = null;
/** rAF id for the remote FFT loop (null when not running). */
let fftRafId: number | null = null;
/** Reusable byte buffer for analyser.getByteFrequencyData (Uint8Array bins).
 *  Typed as `Uint8Array<ArrayBuffer>` to match the narrower lib.dom signature. */
let fftByteBins: Uint8Array<ArrayBuffer> | null = null;
/** Reusable Float32Array for publishing to the frequencyData store. */
let fftFloatBins: Float32Array | null = null;

/** FFT size for the remote AnalyserNode. Must be a power of two; 1024
 *  matches the Rust FftEngine size used for local playback so the
 *  visualizer sees the same bin count regardless of source. */
const REMOTE_FFT_SIZE = 1024;

/** Parse the proxy port from a proxied stream URL like
 *  `http://127.0.0.1:8765/proxy?url=...`. Returns null if the URL is
 *  not a local proxy URL. */
function parseProxyPort(url: string): number | null {
  const m = url.match(/^https?:\/\/127\.0\.0\.1:(\d+)\/proxy\?/);
  return m ? Number(m[1]) : null;
}

/** Build a proxy-routed URL for a local file path so the cached YouTube
 *  m4a is served with CORS headers (the proxy injects
 *  `Access-Control-Allow-Origin: *`). Falls back to `convertFileSrc`
 *  (asset://) when the proxy port cannot be derived — in that case the
 *  AnalyserNode simply won't produce real data for the cached file, but
 *  playback still works (mirrors pre-FFT behavior). */
function proxyLocalUrl(port: number | null, path: string): string {
  if (port == null) return convertFileSrc(path);
  // Encode the file:// URL the same way the Rust proxy expects it.
  const fileUrl = `file://${path}`;
  return `http://127.0.0.1:${port}/proxy?url=${encodeURIComponent(fileUrl)}`;
}

/** Lazily create the AudioContext + MediaElementSource + AnalyserNode.
 *  `createMediaElementSource` can only be called ONCE per element —
 *  after that the source is permanently bound — so we create the chain
 *  on the first track and keep it alive for the element's lifetime. */
function ensureWebAudioChain(el: HTMLAudioElement): void {
  if (mediaSource) return; // already bound to this element
  const Ctx = window.AudioContext || (window as any).webkitAudioContext;
  if (!Ctx) return; // WebKitGTK without Web Audio — gracefully no-op
  audioCtx = new Ctx();
  try {
    mediaSource = audioCtx.createMediaElementSource(el);
  } catch {
    // Source may already be bound (HMR edge case) — bail out safely.
    mediaSource = null;
    audioCtx = null;
    return;
  }
  gainNode = audioCtx.createGain();
  gainNode.gain.value = Math.max(0, Math.min(1, get(volume) / 100));
  analyser = audioCtx.createAnalyser();
  analyser.fftSize = REMOTE_FFT_SIZE;
  analyser.smoothingTimeConstant = 0.8;
  mediaSource.connect(gainNode);
  gainNode.connect(analyser);
  analyser.connect(audioCtx.destination);
  // Pre-allocate reusable buffers (half the FFT size = Nyquist bins).
  fftByteBins = new Uint8Array(analyser.frequencyBinCount);
  fftFloatBins = new Float32Array(analyser.frequencyBinCount);
}

/** Start the rAF loop that reads the analyser and publishes FrequencyData.
 *  No-op if the analyser isn't ready. */
function startRemoteFftLoop(): void {
  if (!analyser || !fftByteBins || !fftFloatBins) return;
  if (fftRafId !== null) return; // already running

  const sampleRate = audioCtx?.sampleRate ?? 44100;

  const tick = (): void => {
    if (!analyser || !fftByteBins || !fftFloatBins) {
      fftRafId = null;
      return;
    }
    // getByteFrequencyData returns 0..255 magnitudes. Convert to 0..1
    // floats so the visualizer's existing peak-based normalization works.
    analyser.getByteFrequencyData(fftByteBins);
    let peak = 0;
    for (let i = 0; i < fftByteBins.length; i++) {
      const v = fftByteBins[i] / 255;
      fftFloatBins[i] = v;
      if (v > peak) peak = v;
    }
    frequencyData.set({
      bins: fftFloatBins,
      sampleRate,
      peak,
    });
    fftRafId = requestAnimationFrame(tick);
  };
  fftRafId = requestAnimationFrame(tick);
}

/** Stop the rAF loop and clear the frequencyData store. Called on
 *  stopRemote and when local playback takes over. */
function stopRemoteFftLoop(): void {
  if (fftRafId !== null) {
    cancelAnimationFrame(fftRafId);
    fftRafId = null;
  }
  frequencyData.set(null);
}

/** Resume the AudioContext if it was suspended (autoplay policy / WebKit
 *  requiring a user gesture). Safe to call repeatedly.
 *
 *  WebKitGTK starts a fresh AudioContext in the `suspended` state and will
 *  only move it to `running` from within a user-gesture call stack. When
 *  `loadRemoteStream()` runs outside a gesture (e.g. an auto-advanced queue
 *  or a programmatic `nextTrack()`), the initial `resumeAudioCtx()` call
 *  silently no-ops and the AnalyserNode keeps producing all-zero bins.
 *
 *  Any genuine user gesture that should light up Modo Cine — the Modo Cine
 *  button click — MUST call this so the suspended context finally resumes
 *  and `frequencyData` starts carrying real magnitudes. */
export function resumeRemoteAudioCtx(): void {
  if (audioCtx && audioCtx.state === 'suspended') {
    audioCtx.resume().catch(() => {
      // Autoplay gesture not yet received — loop will start producing
      // data once the context is resumed by a later user interaction.
    });
  }
}

/** Internal alias kept for the existing `loadRemoteStream` / `resumeRemote`
 *  call sites. Delegates to the public function. */
function resumeAudioCtx(): void {
  resumeRemoteAudioCtx();
}

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

/** Create or reuse an HTMLAudio element. */
function getAudio(): HTMLAudioElement {
  if (!audioEl) {
    audioEl = new Audio();
    audioEl.preload = 'auto';
    // crossOrigin='anonymous' is REQUIRED for the AnalyserNode to see
    // real data. The proxy already responds with
    // `Access-Control-Allow-Origin: *` on both the remote-forward path
    // and the file:// local-cache path, so all remote sources are
    // CORS-compliant when routed through the proxy. This MUST be set
    // before any src is assigned, otherwise the element is tainted and
    // getByteFrequencyData returns all zeros.
    audioEl.crossOrigin = 'anonymous';

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

  // Ensure the Web Audio AnalyserNode chain is bound to this audio
  // element BEFORE we assign a src. createMediaElementSource can only be
  // called once per element lifetime; subsequent tracks reuse it.
  ensureWebAudioChain(audio);

  // Set playback volume. When the remote track is routed through Web Audio,
  // control loudness via GainNode (audioEl.volume can become ineffective once
  // playback is flowing through createMediaElementSource on some WebKit builds).
  if (gainNode) {
    gainNode.gain.value = Math.max(0, Math.min(1, get(volume) / 100));
    audio.volume = 1;
  } else {
    audio.volume = get(volume) / 100;
  }
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

  // Start the remote FFT rAF loop so Modo Cine has frequency data. The
  // AudioContext may start suspended (autoplay policy) — resume it on
  // play, and the loop will begin producing real data once it's running.
  resumeAudioCtx();
  startRemoteFftLoop();

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
        // Route the cached file through the proxy so it is served with
        // CORS headers (Access-Control-Allow-Origin: *). The audio
        // element has crossOrigin='anonymous' set, so a non-CORS source
        // (like asset:// from convertFileSrc) would silently taint the
        // AnalyserNode and zero out frequency data. The proxy serves
        // file:// URLs with the same CORS + Range headers as remote.
        const port = parseProxyPort(playableUrl);
        const localUrl = proxyLocalUrl(port, localPath);
        // Preserve current playback position across the source swap.
        // The browser must load the new source's metadata before currentTime
        // can be set — setting it immediately after src change is a no-op.
        const currentPosition = audio.currentTime;
        const wasPlaying = !audio.paused;

        swappingSource = true;

        // Revert to the proxy URL if the local file fails to load.
        // This handles: corrupt file, unsupported codec, proxy/asset error.
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
    // The AudioContext may have been suspended by the autoplay policy;
    // a user gesture (clicking play) is the right moment to resume it.
    resumeAudioCtx();
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
    const normalized = Math.max(0, Math.min(1, value / 100));
    if (gainNode) {
      gainNode.gain.value = normalized;
      audioEl.volume = 1;
    } else {
      audioEl.volume = normalized;
    }
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
  // Stop the remote FFT rAF loop first so we don't keep publishing
  // stale frequency data after the element is torn down.
  stopRemoteFftLoop();
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
