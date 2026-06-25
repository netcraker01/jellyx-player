/**
 * Remote player store — manages frontend browser-native audio playback
 * for remote tracks (YouTube, SoundCloud, etc.) via HTMLAudioElement.
 *
 * This is the browser-side companion to the Rust proxy server.
 * When Rust emits `stream-resolved`, the frontend loads the proxied URL
 * into an HTMLAudio element and drives play/pause/seek natively.
 *
 * Local tracks still use the Rust Symphonia/cpal pipeline.
 *
 * Architecture:
 * - Rust resolves stream URLs and proxies them for CORS/Range support.
 * - Frontend receives `stream-resolved` event with the proxied URL.
 * - HTMLAudio handles actual playback, buffering, and progress reporting.
 * - The Svelte store mirrors playback state back to the UI.
 */

import { writable, get } from 'svelte/store';
import { progress, isPlaying, currentTrack, volume, nextTrack } from './player';
import { skipToNext } from './player';
import { notifications } from '@shared/stores/notifications';
import { t } from '@i18n';
import type { Track } from '@shared/types/models';

/** The underlying HTMLAudio element for remote playback. */
let audioEl: HTMLAudioElement | null = null;

/** The current stream URL - stored for seek reload. Survives HMR. */
let currentStreamUrl = '';

/** The source of the current track (YouTube, SoundCloud, Local). */
let currentSource = '';

/** Known duration of the current track (from yt-dlp metadata), used as fallback
 *  when HTMLAudioElement.duration returns Infinity (common with YouTube m4a). */
let trackDuration = 0;

/** Whether a seek is in progress — suppresses error handling for aborts. */
let seeking = false;

/** Whether audio was playing before the seek started — used to resume after seek. */
let wasPlayingBeforeSeek = false;

/** Whether a remote track is currently loaded. */
export const remoteActive = writable(false);

/** Create or reuse an HTMLAudio element. */
function getAudio(): HTMLAudioElement {
  if (!audioEl) {
    audioEl = new Audio();
    audioEl.crossOrigin = 'anonymous';
    audioEl.preload = 'auto';

    // Sync timeupdate → progress store, and drive MSE segment prefetching
    audioEl.addEventListener('timeupdate', () => {
      const el = audioEl;
      if (!el) return;

      const dur = el.duration;
      // YouTube m4a streams report Infinity for duration. Use the track's
      // known duration from yt-dlp metadata as a reliable fallback.
      const safeDuration = Number.isFinite(dur) && dur > 0 ? dur : trackDuration;
      const safePosition = Number.isFinite(el.currentTime) ? el.currentTime : 0;
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

    // Sync seeking complete — reanudar reproducción si estaba sonando antes
    audioEl.addEventListener('seeked', () => {
      const el = audioEl;
      if (!el) return;
      seeking = false;
      // After a seek, the browser may pause and not auto-resume.
      // If we were playing before the seek, resume playback now.
      if (wasPlayingBeforeSeek && el.paused) {
        el.play().catch((e) => {
          const msg = e instanceof Error ? e.message : String(e);
          notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
        });
      }
      wasPlayingBeforeSeek = false;
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

      // Suppress ALL errors during seek — the browser fires error events
      // when we reload audio.src for seek, and calling skipToNext() would
      // cause an infinite loop of errors.
      if (seeking) {
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
 * YouTube m4a streams go through MSE (Media Source Extensions) because the
 * browser reports Infinity for duration and native `currentTime` seek breaks.
 * SoundCloud MP3 streams work fine with direct `audio.src` + `currentTime`.
 */
export async function loadRemoteStream(track: Track, streamUrl: string): Promise<void> {
  const audio = getAudio();

  // Store the track's known duration from yt-dlp metadata as fallback.
  // YouTube m4a streams report Infinity for audioEl.duration.
  trackDuration = track.duration ?? 0;
  currentSource = track.source;

  // Stop any current playback, including any prior MSE session.
  audio.pause();
  audio.src = '';

  // Set volume (0-1)
  audio.volume = get(volume) / 100;
  currentStreamUrl = streamUrl;

  // All remote tracks use direct audio.src via the proxy.
  // SoundCloud seek works natively. YouTube seek is handled by
  // reloading the source with a seekto param in seekRemote().
  audio.src = streamUrl;

  try {
    await audio.play();
    remoteActive.set(true);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
    remoteActive.set(false);
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

/** Seek to a position in seconds. */
export function seekRemote(position: number): void {
  const el = audioEl;
  if (!el) return;

  wasPlayingBeforeSeek = !el.paused;
  seeking = true;

  // YouTube: try native currentTime seek. The browser will make Range
  // requests to the proxy. If duration is Infinity, the browser can't
  // seek but we still need to resume playback.
  wasPlayingBeforeSeek = !el.paused;
  el.currentTime = position;

  // After setting currentTime, the browser fires 'seeking' then 'seeked'.
  // If it was playing before, resume playback after the seek completes.
  const resumeAfterSeek = () => {
    el.removeEventListener('seeked', resumeAfterSeek);
    if (wasPlayingBeforeSeek && el.paused) {
      el.play().catch(() => {});
    }
    seeking = false;
  };
  el.addEventListener('seeked', resumeAfterSeek);

  // Fallback: if 'seeked' never fires (Infinity duration), resume after 1s
  setTimeout(() => {
    el.removeEventListener('seeked', resumeAfterSeek);
    if (wasPlayingBeforeSeek && el.paused) {
      el.play().catch(() => {});
    }
    seeking = false;
  }, 1000);
}


/** Set volume for remote playback (0-100). */
export function setRemoteVolume(value: number): void {
  if (audioEl) {
    audioEl.volume = Math.max(0, Math.min(1, value / 100));
  }
}

/** Stop and cleanup remote playback. */
export function stopRemote(): void {
  if (audioEl) {
    audioEl.pause();
    audioEl.src = '';
    audioEl.load();
  }
  remoteActive.set(false);
}

/** Get the current HTMLAudio element (for advanced use). */
export function getAudioElement(): HTMLAudioElement | null {
  return audioEl;
}
