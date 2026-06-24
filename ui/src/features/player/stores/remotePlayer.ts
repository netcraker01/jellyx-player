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

/** Whether a remote track is currently loaded. */
export const remoteActive = writable(false);

/** Create or reuse an HTMLAudio element. */
function getAudio(): HTMLAudioElement {
  if (!audioEl) {
    audioEl = new Audio();
    audioEl.crossOrigin = 'anonymous';

    // Sync timeupdate → progress store
    audioEl.addEventListener('timeupdate', () => {
      if (audioEl) {
        progress.set({ position: audioEl.currentTime, duration: audioEl.duration || 0 });
      }
    });

    // Sync play → isPlaying store
    audioEl.addEventListener('play', () => {
      isPlaying.set(true);
    });

    // Sync pause → isPlaying store
    audioEl.addEventListener('pause', () => {
      isPlaying.set(false);
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
 */
export async function loadRemoteStream(track: Track, streamUrl: string): Promise<void> {
  const audio = getAudio();

  // Stop any current playback
  audio.pause();
  audio.src = '';

  // Set volume (0-1)
  audio.volume = get(volume) / 100;

  // Load new stream
  audio.src = streamUrl;
  audio.load();

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
  if (audioEl) {
    audioEl.currentTime = position;
  }
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
