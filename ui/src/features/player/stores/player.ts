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
  resumeRemoteAudioCtx,
  seekRemote,
  stopRemote,
  remoteActive,
} from './remotePlayer';
import {
  DEFAULT_VISUALIZER_MODE,
  type VisualizerModeId,
} from '../visualizers/registry';

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

import { getMigratedItem, setMigratedItem } from '@shared/utils/storage';
import { extractErrorMessage } from '@shared/utils/errors';

/** localStorage suffix for persisted volume (0-100, the user-facing unit). */
const VOLUME_SUFFIX = 'volume';

/** Default volume (0-100). Used when no persisted value exists. */
const VOLUME_DEFAULT = 80;

/** Read the persisted volume (0-100), falling back to the default. */
function readPersistedVolume(): number {
  const raw = getMigratedItem(VOLUME_SUFFIX);
  if (raw == null) return VOLUME_DEFAULT;
  const n = Number(raw);
  if (!Number.isFinite(n)) return VOLUME_DEFAULT;
  return Math.min(100, Math.max(0, Math.round(n)));
}

/** Current volume level (0-100, the user-facing unit). Persisted to localStorage. */
export const volume = writable<number>(readPersistedVolume());

// Persist volume to localStorage whenever it changes.
volume.subscribe((v) => {
  setMigratedItem(VOLUME_SUFFIX, String(v));
});

/** Whether audio normalization is enabled. Persisted in DB. */
export const normalizeAudio = writable(true);

/** Latest frequency data from the Rust FFT engine (null until first event). */
export const frequencyData = writable<FrequencyData | null>(null);

/** The playback pipeline currently allowed to publish FFT frames. */
export type FftSource = 'local' | 'remote';

// Local playback is the default because its Rust channel is established during
// bootstrap. Switching sources clears the old frame and prevents in-flight
// callbacks from a previous pipeline from repainting the visualizer.
let activeFftSource: FftSource = 'local';

/** Select the only playback pipeline permitted to update frequencyData. */
export function selectFftSource(source: FftSource): void {
  if (activeFftSource === source) return;
  activeFftSource = source;
  frequencyData.set(null);
}

/** Publish a frame only when it belongs to the active playback pipeline. */
export function publishFftFrame(source: FftSource, data: FrequencyData): void {
  if (activeFftSource === source) {
    frequencyData.set(data);
  }
}

/** Clear the shared frame only when the stopped pipeline is still active. */
export function clearFftSource(source: FftSource): void {
  if (activeFftSource === source) {
    frequencyData.set(null);
  }
}

/** Whether the Winamp-style fullscreen visualizer overlay is active.
 *
 *  This is the VISUALIZER toggle — independent from `cinematicMode` (the
 *  ambient background controlled by Settings). It is driven by the bottom-bar
 *  button next to the volume slider (see `toggleModoCine`) and consumed by
 *  `Visualizer.svelte` to expand its canvas to a fullscreen overlay. It is
 *  NOT persisted: the visualizer is a transient, per-session view. */
export const modoCineActive = writable<boolean>(false);

/** localStorage suffix for the persisted visualizer mode. */
const VISUALIZER_MODE_SUFFIX = 'visualizer-mode';

/** Read a persisted visualizer mode id, validating it is a known mode.
 *  Unknown/missing values fall back to the default (bars) so the store
 *  never carries a stale id that the registry can't resolve. */
function readPersistedVisualizerMode(): VisualizerModeId {
  const raw = getMigratedItem(VISUALIZER_MODE_SUFFIX);
  if (raw == null) return DEFAULT_VISUALIZER_MODE;
  // Validate against the registry's known ids; ignore anything else.
  // (We import the mode set lazily to avoid a circular import with the
  //  registry importing renderers that import types only.)
  const known: VisualizerModeId[] = ['bars', 'wave', 'mirror'];
  return (known as readonly string[]).includes(raw) ? (raw as VisualizerModeId) : DEFAULT_VISUALIZER_MODE;
}

/** Currently selected visualizer mode id (persisted to localStorage).
 *
 *  Driven by the `VisualizerSelector` inside the fullscreen overlay. The host
 *  (`Visualizer.svelte`) resolves this id to a renderer via the registry on
 *  every frame, so switching modes is a pure dispatch with no rAF churn.
 *  `modoCineActive` toggles the overlay itself; this store only picks which
 *  renderer runs while the overlay is open. */
export const visualizerMode = writable<VisualizerModeId>(readPersistedVisualizerMode());

visualizerMode.subscribe((v) => {
  setMigratedItem(VISUALIZER_MODE_SUFFIX, String(v));
});

/** Toggle the fullscreen visualizer overlay from the bottom-bar button.
 *
 *  This toggles the VISUALIZER (`modoCineActive`), which is independent from
 *  the cinematic background (`cinematicMode`, controlled by Settings). It
 *  MUST run inside the user-gesture call stack so it can also call
 *  `resumeRemoteAudioCtx()`. On WebKitGTK a remote-track AudioContext starts
 *  suspended and only resumes from within a gesture; if playback began
 *  outside a gesture (auto-advanced queue, programmatic nextTrack), the
 *  initial `resumeRemoteAudioCtx()` in `loadRemoteStream` silently no-ops
 *  and the AnalyserNode keeps emitting all-zero bins, so the visualizer
 *  renders black. Calling `resumeRemoteAudioCtx()` here guarantees the
 *  context finally moves to `running` the instant the user asks for the
 *  visualizer, and `frequencyData` starts carrying real magnitudes. */
export function toggleModoCine(): void {
  modoCineActive.update((v) => !v);
  resumeRemoteAudioCtx();
}

// ── Cinematic ambient mode ─────────────────────────────────────────
// Opt-in reactive background that paints layered gradients/glows behind the
// app content, pulsing on frequencyData. Persisted to localStorage only — no
// backend round-trip.

/** localStorage suffix for the cinematic-mode on/off preference. */
const CINEMATIC_MODE_SUFFIX = 'cinematic-mode';

/** localStorage suffix for the cinematic intensity (0..1) preference. */
const CINEMATIC_INTENSITY_SUFFIX = 'cinematic-intensity';

/** Default intensity when no persisted value exists (0..1). */
const CINEMATIC_INTENSITY_DEFAULT = 0.5;

/** Read a boolean preference from localStorage, defaulting to false. */
function readPersistedFlag(suffix: string): boolean {
  return getMigratedItem(suffix) === 'true';
}

/** Read a clamped float preference from localStorage. */
function readPersistedFloat(suffix: string, fallback: number, min = 0, max = 1): number {
  const raw = getMigratedItem(suffix);
  if (raw == null) return fallback;
  const n = Number(raw);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, n));
}

/** Whether the cinematic ambient background is enabled (persisted, default off). */
export const cinematicMode = writable<boolean>(readPersistedFlag(CINEMATIC_MODE_SUFFIX));

cinematicMode.subscribe((v) => {
  setMigratedItem(CINEMATIC_MODE_SUFFIX, String(v));
});

/** Cinematic background intensity (0..1, persisted, default 0.5). */
export const cinematicIntensity = writable<number>(
  readPersistedFloat(CINEMATIC_INTENSITY_SUFFIX, CINEMATIC_INTENSITY_DEFAULT)
);

cinematicIntensity.subscribe((v) => {
  setMigratedItem(CINEMATIC_INTENSITY_SUFFIX, String(v));
});

/** Toggle the cinematic ambient mode on/off. */
export function toggleCinematicMode(): void {
  cinematicMode.update((v) => !v);
}

/** Set the cinematic intensity, clamped to 0..1. */
export function setCinematicIntensity(value: number): void {
  cinematicIntensity.set(Math.min(1, Math.max(0, value)));
}

// ── Event Initialization ──────────────────────────────────────────

let initialized = false;
let playerEventsStarting: Promise<void> | null = null;
let fftListenerUnlisten: (() => void) | null = null;
let fftListenerStarting: Promise<void> | null = null;

/**
 * Subscribe to Rust FFT events. The Rust engine emits "fft-frame" events
 * at ~60fps via `webview.emit()`. Events have no ordering guarantees —
 * if a frame is lost, the next one arrives normally (no Channel stall).
 */
async function ensureLocalFftListener(): Promise<void> {
  if (fftListenerUnlisten) return;
  if (fftListenerStarting) return fftListenerStarting;

  fftListenerStarting = events.onFftFrame((data: FrequencyData) => {
    publishFftFrame('local', data);
  }).then((unlisten) => {
    fftListenerUnlisten = unlisten;
  }).finally(() => {
    fftListenerStarting = null;
  });

  return fftListenerStarting;
}

/** Prepare the local FFT event listener for a new playback. */
export async function prepareLocalFft(): Promise<void> {
  selectFftSource('local');
  await ensureLocalFftListener();
}

/**
 * Initialize player event subscriptions.
 * Call once from main.ts at app bootstrap.
 * Registers listeners for all playback events from Rust.
 */
async function initializePlayerEvents(): Promise<void> {
  const unlisten = [] as Array<() => void>;

  try {

    // The FFT stream belongs to playback, not to either visualizer view. Start
    // it before any component mounts so local files update the shared store even
    // while the player is on another route or in the mini-player window.
    await ensureLocalFftListener();

    // Track changed — update current track
    unlisten.push(await events.onTrackChanged((track: Track) => {
    if (track.localPath) {
      // Handles backend-driven queue advancement as well as explicit UI play.
      prepareLocalFft().catch(() => {});
    } else {
      selectFftSource('remote');
    }
    currentTrack.set(track);
    }));

  // State changed — update isPlaying and isBuffering
    unlisten.push(await events.onStateChanged((state: string) => {
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
      clearFftSource('local');
    }
    }));

  // Queue updated — update full queue snapshot and derived mode state
    unlisten.push(await events.onQueueUpdated((state: QueueState) => {
    queueState.set(state);
    shuffle.set(state.shuffle);
    repeatMode.set(state.repeatMode);
    }));

  // Progress tick — update position and duration (skip if remote active,
  // since remotePlayer updates progress directly from HTMLAudio)
    unlisten.push(await events.onProgressTick((tick: events.ProgressTick) => {
    if (!get(remoteActive)) {
      progress.set({ position: tick.position, duration: tick.duration });
    }
    }));

  // Buffering progress — update buffering percentage for remote tracks
    unlisten.push(await events.onBufferingProgress((payload: events.BufferingProgressEvent) => {
    isBuffering.set(true);
    bufferingProgress.set(payload.progress);
    }));

  // Stream resolved — remote playback URL ready; load into HTMLAudio
    unlisten.push(await events.onStreamResolved((payload: events.StreamResolvedEvent) => {
    const track = get(currentTrack);
    if (track && shouldAcceptStreamResolution(track, payload)) {
      loadRemoteStream(track, payload.streamUrl, payload.remoteUrl, payload.proxyCapability).catch((e) => {
        const msg = extractErrorMessage(e, get(t));
        notifications.push({ type: 'error', title: 'Remote Playback Error', message: msg, dismissible: true });
      });
    }
    }));

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

  // Sync the persisted volume to the Rust backend's InternalState so the first
  // local track plays at the user's chosen level instead of the backend's 1.0
  // default. The command clamps to 0.0-1.0; divide the 0-100 UI value here.
    try {
      await commands.setVolume(get(volume) / 100);
    } catch {
      // Backend unavailable — volume applies on next track start via the store.
    }

    initialized = true;
  } catch (error) {
    for (const stopListening of unlisten) stopListening();
    if (fftListenerUnlisten) {
      fftListenerUnlisten();
      fftListenerUnlisten = null;
    }
    throw error;
  }
}

/** Reject late resolver events, including a replay of the same track id. */
export function shouldAcceptStreamResolution(
  track: Track | null,
  payload: events.StreamResolvedEvent,
): boolean {
  return !!track
    && track.id === payload.trackId
    && commands.isLatestStreamRequest(payload.streamRequestId);
}

export async function initPlayerEvents(): Promise<void> {
  if (initialized) return;
  if (!playerEventsStarting) {
    playerEventsStarting = initializePlayerEvents().finally(() => {
      playerEventsStarting = null;
    });
  }
  return playerEventsStarting;
}

// ── Actions ────────────────────────────────────────────────────────

/** Play a track, dispatching to the correct backend command by source. */
export async function playTrack(track: Track): Promise<void> {
  try {
    if (track.localPath) {
      commands.invalidateStreamRequests();
      stopRemote();
      await prepareLocalFft();
      await commands.playLocal(track.localPath);
    } else {
      // Remote track (YouTube, SoundCloud) — use play_stream for HTTP streaming
      selectFftSource('remote');
      await commands.playStream(track);
    }
  } catch (e) {
    const translate = get(t);
    const msg = extractErrorMessage(e, translate);
    // Detect DRM-protected tracks for a user-friendly message
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
      const msg = extractErrorMessage(e, translate);
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
    const msg = extractErrorMessage(e, get(t));
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
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Skip to next track. */
export async function nextTrack(): Promise<void> {
  try {
    await commands.next();
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Skip to previous track. */
export async function previousTrack(): Promise<void> {
  try {
    await commands.previous();
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
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
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Set volume (0-100, the user-facing unit). Scales to 0.0-1.0 for the Rust
 *  backend and forwards to the remote HTMLAudio path (which also expects 0-100
 *  and divides internally). Persists to localStorage via the volume store subscriber. */
export async function setVolume(value: number): Promise<void> {
  const clamped = Math.min(100, Math.max(0, Math.round(value)));
  volume.set(clamped);
  // Sync remote playback volume if active (remotePlayer divides by 100 itself)
  try {
    const { setRemoteVolume } = await import('./remotePlayer');
    setRemoteVolume(clamped);
  } catch {
    // remotePlayer may not be available in test environments
  }
  try {
    // Backend expects 0.0-1.0 — scale the 0-100 UI value here at the IPC boundary.
    await commands.setVolume(clamped / 100);
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
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
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Cycle repeat mode: Off -> All -> One -> Off. */
export async function cycleRepeat(): Promise<void> {
  try {
    await commands.cycleRepeat();
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Remove a track from the queue by its internal track ID. */
export async function removeTrack(trackId: string): Promise<void> {
  try {
    await commands.removeFromQueue(trackId);
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Clear the entire queue and stop playback. */
export async function clearQueue(): Promise<void> {
  try {
    await commands.clearQueue();
    notifications.push({ type: 'info', title: 'Queue', message: get(t)('toasts.queue_cleared'), dismissible: true });
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
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
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Settings Error', message: msg, dismissible: true });
  }
}

/** Insert a selected track immediately after the current track. */
export async function playNext(trackId: string): Promise<void> {
  try {
    await commands.playNext(trackId);
    notifications.push({ type: 'info', title: 'Queue', message: get(t)('toasts.play_next_set'), dismissible: true });
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}
