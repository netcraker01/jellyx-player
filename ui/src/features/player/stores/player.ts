/**
 * Player store — IPC-backed Svelte store for playback state.
 *
 * Subscribes to Tauri events (track-changed, state-changed, queue-updated, progress-tick)
 * and provides action methods that call Tauri commands.
 * Rust is the Source of Truth — Svelte is a dumb client.
 */
import { writable, derived, get } from 'svelte/store';
import * as events from '@services/events';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import { t } from '@i18n';
import type { Track, QueueState, FrequencyData } from '@shared/types/models';
import {
  loadRemoteStream,
  pauseRemote,
  resumeRemote,
  seekRemote,
  stopRemote,
  remoteActive,
} from './remotePlayer';

// ── Stores ────────────────────────────────────────────────────────

/** Currently playing track (null when idle). */
export const currentTrack = writable<Track | null>(null);

/** Whether audio is currently playing. */
export const isPlaying = writable(false);

/** Whether the player is in a buffering state (e.g., resolving/streaming a remote track). */
export const isBuffering = writable(false);

/** Buffering progress percentage (0 to 1). Null when not buffering. */
export const bufferingProgress = writable<number | null>(null);

/** Current playback progress: { position: seconds, duration: seconds }. */
export const progress = writable<{ position: number; duration: number }>({ position: 0, duration: 0 });

/** Full queue snapshot from the Rust backend. */
export const queueState = writable<QueueState>({
  tracks: [],
  currentIndex: null,
  shuffle: false,
  repeatMode: 'Off',
  playedIndices: [],
});

/** Current playback queue tracks (kept in original order). */
export const queue = derived(queueState, ($state) => $state.tracks);

/** Index of the current track within the queue. */
export const currentIndex = derived(queueState, ($state) => $state.currentIndex);

/** Whether shuffle mode is enabled. */
export const shuffle = writable(false);

/** Current repeat mode: Off, All, or One. */
export const repeatMode = writable<QueueState['repeatMode']>('Off');

/** Current volume level (0-100). */
export const volume = writable(80);

/** Whether audio normalization is enabled. Persisted in DB. */
export const normalizeAudio = writable(true);

/** Latest frequency data from the Rust FFT engine (null until first event). */
export const frequencyData = writable<FrequencyData | null>(null);

/** Whether Modo Cine (immersive fullscreen visualizer) is active. */
export const modoCineActive = writable<boolean>(false);

// ── Event Initialization ──────────────────────────────────────────

let initialized = false;

/**
 * Initialize player event subscriptions.
 * Call once from main.ts at app bootstrap.
 * Registers listeners for all playback events from Rust.
 */
export async function initPlayerEvents(): Promise<void> {
  if (initialized) return;
  initialized = true;

  // Track changed — update current track
  await events.onTrackChanged((track: Track) => {
    currentTrack.set(track);
  });

  // State changed — update isPlaying and isBuffering
  await events.onStateChanged((state: string) => {
    isPlaying.set(state === 'Playing');
    if (state === 'Playing') {
      isBuffering.set(false);
      bufferingProgress.set(null);
    } else if (state.startsWith('Buffering')) {
      isBuffering.set(true);
      bufferingProgress.set(0.0);
    } else if (state === 'Stopped' || state === 'Paused') {
      isBuffering.set(false);
      bufferingProgress.set(null);
    }

    // Stop remote playback when Rust signals stopped
    if (state === 'Stopped') {
      stopRemote();
    }
  });

  // Queue updated — update full queue snapshot and derived mode state
  await events.onQueueUpdated((state: QueueState) => {
    queueState.set(state);
    shuffle.set(state.shuffle);
    repeatMode.set(state.repeatMode);
  });

  // Progress tick — update position and duration (skip if remote active,
  // since remotePlayer updates progress directly from HTMLAudio)
  await events.onProgressTick((tick: events.ProgressTick) => {
    if (!get(remoteActive)) {
      progress.set({ position: tick.position, duration: tick.duration });
    }
  });

  // Buffering progress — update buffering percentage for remote tracks
  await events.onBufferingProgress((payload: events.BufferingProgressEvent) => {
    isBuffering.set(true);
    bufferingProgress.set(payload.progress);
  });

  // Stream resolved — remote playback URL ready; load into HTMLAudio
  await events.onStreamResolved((payload: events.StreamResolvedEvent) => {
    const track = get(currentTrack);
    if (track && track.id === payload.trackId) {
      loadRemoteStream(track, payload.streamUrl, payload.remoteUrl).catch((e) => {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Remote Playback Error', message: msg, dismissible: true });
      });
    }
  });

  // Load persisted audio normalization setting.
  // Normalization is applied in the backend during cache download (ffmpeg
  // loudnorm), so we only need to persist the setting here. The next track
  // load will cache the normalized variant automatically.
  try {
    const settings = await commands.getAudioSettings();
    normalizeAudio.set(settings.normalizeAudio);
    // Apply to local (Rust) audio backend immediately
    await commands.setPlaybackNormalizeAudio(settings.normalizeAudio);
  } catch {
    // Defaults to enabled — leave the store's default (true)
  }
}

// ── Actions ────────────────────────────────────────────────────────

/** Play a track, dispatching to the correct backend command by source. */
export async function playTrack(track: Track): Promise<void> {
  try {
    if (track.localPath) {
      await commands.playLocal(track.localPath);
    } else {
      // Remote track (YouTube, SoundCloud) — use play_stream for HTTP streaming
      await commands.playStream(track);
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    // Detect DRM-protected tracks for a user-friendly message
    const translate = get(t);
    const drmMessage = msg.includes('DRM')
      ? translate('playback.drm_protected', { default: 'Cannot play: DRM-protected track' })
      : msg;
    notifications.push({ type: 'error', title: translate('playback.error_title', { default: 'Playback Error' }), message: drmMessage, dismissible: true });
    // Auto-advance to the next track in the queue instead of stopping
    skipToNext();
  }
}

/**
 * Skip to next track, auto-advancing past tracks that fail to resolve (e.g. DRM).
 * Tries up to 10 consecutive tracks before giving up to prevent infinite loops.
 */
export async function skipToNext(): Promise<void> {
  const translate = get(t);
  for (let i = 0; i < 10; i++) {
    try {
      await commands.next();
      return; // Successfully started next track
    } catch (e) {
      // Track failed (DRM, network, etc.) — show error and try the next one
      const msg = e instanceof Error ? e.message : String(e);
      const drmMessage = msg.includes('DRM')
        ? translate('playback.drm_protected', { default: 'Cannot play: DRM-protected track' })
        : msg;
      notifications.push({ type: 'error', title: translate('playback.error_title', { default: 'Playback Error' }), message: drmMessage, dismissible: true });
      // Continue loop to try the next track
    }
  }
  // All tracks failed — stop
  notifications.push({ type: 'error', title: translate('playback.error_title', { default: 'Playback Error' }), message: translate('playback.all_failed', { default: 'All tracks in queue failed to play' }), dismissible: true });
}

/** Pause current playback. */
export async function pauseTrack(): Promise<void> {
  try {
    if (get(remoteActive)) {
      pauseRemote();
    }
    await commands.pause();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Resume paused playback. */
export async function resumeTrack(): Promise<void> {
  try {
    if (get(remoteActive)) {
      resumeRemote();
    }
    await commands.resume();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Skip to next track. */
export async function nextTrack(): Promise<void> {
  try {
    await commands.next();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Skip to previous track. */
export async function previousTrack(): Promise<void> {
  try {
    await commands.previous();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Seek to a position (in seconds). */
export async function seekTo(position: number): Promise<void> {
  try {
    // Always try remote seek first — if there's an audio element playing,
    // it's a remote track. Local tracks use the Symphonia backend.
    if (get(remoteActive) || get(currentTrack)?.source) {
      seekRemote(position);
    }
    if (!get(remoteActive)) {
      // Local tracks use the Symphonia/cpal pipeline — seek via backend.
      await commands.seek(position);
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Set volume (0-100). */
export async function setVolume(value: number): Promise<void> {
  volume.set(value);
  // Sync remote playback volume if active
  try {
    const { setRemoteVolume } = await import('./remotePlayer');
    setRemoteVolume(value);
  } catch {
    // remotePlayer may not be available in test environments
  }
  try {
    await commands.setVolume(value);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Toggle play/pause based on current state. */
export async function togglePlayPause(): Promise<void> {
  let playing = false;
  isPlaying.subscribe((v) => (playing = v))();
  if (playing) {
    await pauseTrack();
  } else {
    await resumeTrack();
  }
}

/** Toggle shuffle mode. */
export async function toggleShuffle(): Promise<void> {
  let enabled = false;
  shuffle.subscribe((v) => (enabled = v))();
  try {
    await commands.setShuffle(!enabled);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Cycle repeat mode: Off -> All -> One -> Off. */
export async function cycleRepeat(): Promise<void> {
  try {
    await commands.cycleRepeat();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Remove a track from the queue by its Helix track ID. */
export async function removeTrack(trackId: string): Promise<void> {
  try {
    await commands.removeFromQueue(trackId);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Clear the entire queue and stop playback. */
export async function clearQueue(): Promise<void> {
  try {
    await commands.clearQueue();
    notifications.push({ type: 'info', title: 'Queue', message: get(t)('toasts.queue_cleared'), dismissible: true });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Toggle audio normalization on/off. Persists to DB.
 *  Normalization is applied during cache download in the backend (ffmpeg
 *  loudnorm). The setting takes effect on the next track load; tracks
 *  already cached with the previous setting are not re-normalized. */
export async function toggleNormalizeAudio(enabled: boolean): Promise<void> {
  try {
    await commands.setNormalizeAudio(enabled);
    await commands.setPlaybackNormalizeAudio(enabled);
    normalizeAudio.set(enabled);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Settings Error', message: msg, dismissible: true });
  }
}

/** Insert a selected track immediately after the current track. */
export async function playNext(trackId: string): Promise<void> {
  try {
    await commands.playNext(trackId);
    notifications.push({ type: 'info', title: 'Queue', message: get(t)('toasts.play_next_set'), dismissible: true });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}