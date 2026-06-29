/**
 * Typed Tauri event subscriptions and binary FFT channel.
 *
 * Event subscriptions are thin wrappers around subscribeEvent that add type safety.
 * Binary FFT uses Tauri v2's Channel API for zero-JSON-overhead streaming.
 * Event names use lowercase-hyphen format matching Rust constants.
 */

import { subscribeEvent } from './tauri';
import { invokeCommand } from './tauri';
import type { Track, FrequencyData, QueueState } from '@shared/types/models';

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
  streamUrl: string;
  /** The raw remote stream URL (before proxying). Present for remote tracks
   * so the frontend can call `cache_remote_stream` to download a local copy
   * for instant seeking. Undefined for local-file proxy URLs. */
  remoteUrl?: string;
}

export function onStreamResolved(cb: (payload: StreamResolvedEvent) => void): Promise<UnlistenFn> {
  return subscribeEvent<StreamResolvedEvent>('stream-resolved', cb);
}

/**
 * Decode a binary FFT frame into FrequencyData.
 *
 * Binary frame layout (all little-endian):
 * - Bytes 0-3: sample_rate (u32 LE)
 * - Bytes 4-7: peak (f32 LE)
 * - Bytes 8+: bins (N * f32 LE)
 */
function decodeFftFrame(buffer: ArrayBuffer): FrequencyData {
  const view = new DataView(buffer);
  const sampleRate = view.getUint32(0, true);  // little-endian
  const peak = view.getFloat32(4, true);         // little-endian
  const bins = new Float32Array(buffer, 8);     // view from byte offset 8
  return { bins, sampleRate, peak };
}

/**
 * Create a Tauri Channel for binary FFT streaming and start the stream.
 *
 * The Channel receives Uint8Array frames from the Rust FFT engine at ~60fps.
 * Each frame is decoded into a FrequencyData object with a Float32Array for bins.
 * Returns an unlisten function that stops the stream.
 */
export async function createFftChannel(cb: (data: FrequencyData) => void): Promise<UnlistenFn> {
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  if (!isTauri) {
    return () => {};
  }

  const { Channel } = await import('@tauri-apps/api/core');

  const channel = new Channel<ArrayBuffer>();
  channel.onmessage = (message: ArrayBuffer) => {
    const data = decodeFftFrame(message);
    cb(data);
  };

  await invokeCommand('start_fft_stream', { channel });

  // Return a no-op unlisten for now — the Channel lifecycle is tied to playback
  // When playback stops, the Rust side clears the channel
  return () => {};
}