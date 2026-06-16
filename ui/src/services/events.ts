/**
 * Typed Tauri event subscriptions.
 *
 * These are thin wrappers around subscribeEvent that add type safety.
 * Actual Rust events will be connected during feature development.
 */

import { subscribeEvent } from './tauri';
import type { Track } from '@shared/types/models';

type UnlistenFn = () => void;

export function onTrackChanged(cb: (track: Track) => void): Promise<UnlistenFn> {
  return subscribeEvent<Track>('track_changed', cb);
}

export function onStateChanged(cb: (state: string) => void): Promise<UnlistenFn> {
  return subscribeEvent<string>('state_changed', cb);
}

export function onQueueUpdated(cb: (queue: Track[]) => void): Promise<UnlistenFn> {
  return subscribeEvent<Track[]>('queue_updated', cb);
}

export function onProgressTick(cb: (progress: number) => void): Promise<UnlistenFn> {
  return subscribeEvent<number>('progress_tick', cb);
}