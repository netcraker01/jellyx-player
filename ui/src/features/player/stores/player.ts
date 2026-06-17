/**
 * Player store — IPC-backed Svelte store for playback state.
 *
 * Subscribes to Tauri events (track-changed, state-changed, queue-updated, progress-tick)
 * and provides action methods that call Tauri commands.
 * Rust is the Source of Truth — Svelte is a dumb client.
 */
import { writable, derived } from 'svelte/store';
import * as events from '@services/events';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import { favorites } from '@features/favorites/stores/favorites';
import type { Track, QueueState, FrequencyData } from '@shared/types/models';

// ── Stores ────────────────────────────────────────────────────────

/** Currently playing track (null when idle). */
export const currentTrack = writable<Track | null>(null);

/** Whether audio is currently playing. */
export const isPlaying = writable(false);

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

/** Latest frequency data from the Rust FFT engine (null until first event). */
export const frequencyData = writable<FrequencyData | null>(null);

/** Whether Modo Cine (immersive fullscreen visualizer) is active. */
export const modoCineActive = writable<boolean>(false);

/** Whether the current track is favorited. */
export const isCurrentTrackFavorited = derived(
  [currentTrack, favorites],
  ([$currentTrack, $favorites]) => {
    if (!$currentTrack) return false;
    return $favorites.some((entry) => entry.track.id === $currentTrack.id);
  },
);

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

  // State changed — update isPlaying
  await events.onStateChanged((state: string) => {
    isPlaying.set(state === 'Playing');
  });

  // Queue updated — update full queue snapshot and derived mode state
  await events.onQueueUpdated((state: QueueState) => {
    queueState.set(state);
    shuffle.set(state.shuffle);
    repeatMode.set(state.repeatMode);
  });

  // Progress tick — update position and duration
  await events.onProgressTick((tick: events.ProgressTick) => {
    progress.set({ position: tick.position, duration: tick.duration });
  });
}

// ── Actions ────────────────────────────────────────────────────────

/** Play a track by stream URL. */
export async function playTrack(url: string): Promise<void> {
  try {
    await commands.play(url);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Pause current playback. */
export async function pauseTrack(): Promise<void> {
  try {
    await commands.pause();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Resume paused playback. */
export async function resumeTrack(): Promise<void> {
  try {
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
    await commands.seek(position);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Set volume (0-100). */
export async function setVolume(value: number): Promise<void> {
  volume.set(value);
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