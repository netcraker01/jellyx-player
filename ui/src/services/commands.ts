/**
 * Typed Tauri command wrappers.
 *
 * These are thin wrappers around invokeCommand that add type safety.
 * Actual Rust implementations will be connected during feature development.
 */

import { invokeCommand } from './tauri';
import type { Track } from '@shared/types/models';

export function search(query: string): Promise<Track[]> {
  return invokeCommand<Track[]>('search', { query });
}

export function play(trackId: string): Promise<void> {
  return invokeCommand<void>('play', { trackId });
}

export function pause(): Promise<void> {
  return invokeCommand<void>('pause');
}

export function next(): Promise<void> {
  return invokeCommand<void>('next');
}

export function previous(): Promise<void> {
  return invokeCommand<void>('previous');
}

export function setVolume(volume: number): Promise<void> {
  return invokeCommand<void>('set_volume', { volume });
}

export function toggleFavorite(trackId: string): Promise<void> {
  return invokeCommand<void>('toggle_favorite', { trackId });
}