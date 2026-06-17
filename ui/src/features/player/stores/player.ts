/**
 * Player store — IPC-backed Svelte store for playback state.
 *
 * Subscribes to Tauri events (track-changed, state-changed, queue-updated, progress-tick)
 * and provides action methods that call Tauri commands.
 * Rust is the Source of Truth — Svelte is a dumb client.
 */
import { writable } from 'svelte/store';
import * as events from '@services/events';
import * as commands from '@services/commands';
import type { Track, FrequencyData } from '@shared/types/models';

// ── Stores ────────────────────────────────────────────────────────

/** Currently playing track (null when idle). */
export const currentTrack = writable<Track | null>(null);

/** Whether audio is currently playing. */
export const isPlaying = writable(false);

/** Current playback progress: { position: seconds, duration: seconds }. */
export const progress = writable<{ position: number; duration: number }>({ position: 0, duration: 0 });

/** Current playback queue (ordered list of tracks). */
export const queue = writable<Track[]>([]);

/** Current volume level (0-100). */
export const volume = writable(80);

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

  // State changed — update isPlaying
  await events.onStateChanged((state: string) => {
    isPlaying.set(state === 'Playing');
  });

  // Queue updated — update full queue
  await events.onQueueUpdated((newQueue: Track[]) => {
    queue.set(newQueue);
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
    console.error('Failed to play track:', e);
  }
}

/** Pause current playback. */
export async function pauseTrack(): Promise<void> {
  try {
    await commands.pause();
  } catch (e) {
    console.error('Failed to pause:', e);
  }
}

/** Resume paused playback. */
export async function resumeTrack(): Promise<void> {
  try {
    await commands.resume();
  } catch (e) {
    console.error('Failed to resume:', e);
  }
}

/** Skip to next track. */
export async function nextTrack(): Promise<void> {
  try {
    await commands.next();
  } catch (e) {
    console.error('Failed to skip next:', e);
  }
}

/** Skip to previous track. */
export async function previousTrack(): Promise<void> {
  try {
    await commands.previous();
  } catch (e) {
    console.error('Failed to skip previous:', e);
  }
}

/** Seek to a position (in seconds). */
export async function seekTo(position: number): Promise<void> {
  try {
    await commands.seek(position);
  } catch (e) {
    console.error('Failed to seek:', e);
  }
}

/** Set volume (0-100). */
export async function setVolume(value: number): Promise<void> {
  volume.set(value);
  try {
    await commands.setVolume(value);
  } catch (e) {
    console.error('Failed to set volume:', e);
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