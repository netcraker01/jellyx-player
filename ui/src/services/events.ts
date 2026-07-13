/**
 * Typed Tauri event subscriptions for playback and FFT streaming.
 *
 * Event subscriptions are thin wrappers around subscribeEvent that add type safety.
 * FFT streaming uses Tauri events (emit/listen) instead of Channel IPC to avoid
 * the strict-index ordering that could permanently stall after a single delivery failure.
 * Event names use lowercase-hyphen format matching Rust constants.
 */

import { subscribeEvent } from './tauri';
import type { Track, FrequencyData, QueueState, UpdateInfo } from '@shared/types/models';

type UnlistenFn = () => void;

/** Progress tick payload emitted periodically during playback. */
export interface ProgressTick {
  position: number;
  duration: number;
}

/** Buffering progress payload for remote track buffering. */
export interface BufferingProgressEvent {
  progress: number;
  trackId: string;
}

export function onTrackChanged(cb: (track: Track) => void): Promise<UnlistenFn> {
  return subscribeEvent<Track>('track-changed', cb);
}

export function onStateChanged(cb: (state: string) => void): Promise<UnlistenFn> {
  return subscribeEvent<string>('state-changed', cb);
}

export function onQueueUpdated(cb: (state: QueueState) => void): Promise<UnlistenFn> {
  return subscribeEvent<QueueState>('queue-updated', cb);
}

export function onProgressTick(cb: (progress: ProgressTick) => void): Promise<UnlistenFn> {
  return subscribeEvent<ProgressTick>('progress-tick', cb);
}

export function onBufferingProgress(cb: (payload: BufferingProgressEvent) => void): Promise<UnlistenFn> {
  return subscribeEvent<BufferingProgressEvent>('buffering-progress', cb);
}

/** Stream resolved payload emitted when a remote track's stream URL is ready. */
export interface StreamResolvedEvent {
  trackId: string;
  /** Echoes the frontend playback request that caused this resolution. */
  streamRequestId: number;
  streamUrl: string;
  /** The raw remote stream URL (before proxying). Present for remote tracks
   * so the frontend can call `cache_remote_stream` to download a local copy
   * for instant seeking. Undefined for local-file proxy URLs. */
  remoteUrl?: string;
  /** Per-process capability for the local stream proxy. Scoped to this event. */
  proxyCapability?: string;
}

export function onStreamResolved(cb: (payload: StreamResolvedEvent) => void): Promise<UnlistenFn> {
  return subscribeEvent<StreamResolvedEvent>('stream-resolved', cb);
}

/** Emitted by the backend periodic update check when a newer version is found. */
export function onUpdateAvailable(cb: (info: UpdateInfo) => void): Promise<UnlistenFn> {
  return subscribeEvent<UpdateInfo>('update-available', cb);
}

/**
 * Tauri event payload for FFT frames emitted by the Rust engine.
 *
 * The Rust side serializes `FrequencyData` as JSON via `webview.emit()`.
 * `bins` arrives as a plain `number[]` which we convert to `Float32Array`.
 */
interface FftFramePayload {
  bins: number[];
  sampleRate: number;
  peak: number;
}

export function frequencyDataFromFftPayload(payload: FftFramePayload): FrequencyData {
  return {
    bins: new Float32Array(payload.bins),
    sampleRate: payload.sampleRate,
    peak: payload.peak,
  };
}

/**
 * Subscribe to FFT frames from the Rust engine via Tauri events.
 *
 * The Rust FFT engine emits `"fft-frame"` events at ~60fps with JSON-serialized
 * `FrequencyData`. Each frame is decoded into a `FrequencyData` object with a
 * `Float32Array` for bins. Returns an unlisten function that stops the stream.
 *
 * Unlike the previous Channel-based approach, events have no ordering guarantees.
 * If a frame is lost, the next one arrives normally — no permanent stall.
 */
export async function onFftFrame(cb: (data: FrequencyData) => void): Promise<UnlistenFn> {
  return subscribeEvent<FftFramePayload>('fft-frame', (payload) => {
    cb(frequencyDataFromFftPayload(payload));
  });
}
