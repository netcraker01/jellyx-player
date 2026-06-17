/**
 * Typed Tauri event subscriptions.
 *
 * These are thin wrappers around subscribeEvent that add type safety.
 * Event names use lowercase-hyphen format matching Rust constants.
 */

import { subscribeEvent } from './tauri';
import type { Track, FrequencyData } from '@shared/types/models';

type UnlistenFn = () => void;

/** Progress tick payload emitted periodically during playback. */
export interface ProgressTick {
  position: number;
  duration: number;
}

export function onTrackChanged(cb: (track: Track) => void): Promise<UnlistenFn> {
  return subscribeEvent<Track>('track-changed', cb);
}

export function onStateChanged(cb: (state: string) => void): Promise<UnlistenFn> {
  return subscribeEvent<string>('state-changed', cb);
}

export function onQueueUpdated(cb: (queue: Track[]) => void): Promise<UnlistenFn> {
  return subscribeEvent<Track[]>('queue-updated', cb);
}

export function onProgressTick(cb: (progress: ProgressTick) => void): Promise<UnlistenFn> {
  return subscribeEvent<ProgressTick>('progress-tick', cb);
}

/** Subscribe to frequency data events from the Rust FFT engine. */
export function onFrequencyData(cb: (data: FrequencyData) => void): Promise<UnlistenFn> {
  return subscribeEvent<FrequencyData>('frequency-data', cb);
}