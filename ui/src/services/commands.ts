/**
 * Typed Tauri command wrappers.
 *
 * These are thin wrappers around invokeCommand that add type safety.
 * All command names match the Rust #[tauri::command] function names.
 * Parameters use camelCase to match Tauri's IPC serialization.
 */

import { invokeCommand } from './tauri';
import type { Track, FavoriteEntry, HistoryEntry } from '@shared/types/models';

// ── Playback commands ──────────────────────────────────────────────

export function play(url: string): Promise<void> {
  return invokeCommand<void>('play', { url });
}

export function pause(): Promise<void> {
  return invokeCommand<void>('pause');
}

export function resume(): Promise<void> {
  return invokeCommand<void>('resume');
}

export function next(): Promise<void> {
  return invokeCommand<void>('next');
}

export function previous(): Promise<void> {
  return invokeCommand<void>('previous');
}

export function seek(position: number): Promise<void> {
  return invokeCommand<void>('seek', { position });
}

export function setVolume(volume: number): Promise<void> {
  return invokeCommand<void>('set_volume', { volume });
}

export function search(query: string): Promise<Track[]> {
  return invokeCommand<Track[]>('search', { query });
}

export function addToQueue(trackId: string): Promise<void> {
  return invokeCommand<void>('add_to_queue', { trackId });
}

export function getQueue(): Promise<Track[]> {
  return invokeCommand<Track[]>('get_queue');
}

export function getVersion(): Promise<string> {
  return invokeCommand<string>('get_version');
}

// ── Library commands ────────────────────────────────────────────────

/** Get all favorited tracks, ordered by most recently added first. */
export function getFavorites(): Promise<FavoriteEntry[]> {
  return invokeCommand<FavoriteEntry[]>('get_favorites');
}

/** Add a track to favorites. */
export function addFavorite(track: Track): Promise<void> {
  return invokeCommand<void>('add_favorite', { track });
}

/** Remove a track from favorites by its Helix track ID. */
export function removeFavorite(trackId: string): Promise<void> {
  return invokeCommand<void>('remove_favorite', { trackId });
}

/** Get play history, ordered by most recent first (max 50). */
export function getHistory(): Promise<HistoryEntry[]> {
  return invokeCommand<HistoryEntry[]>('get_history');
}

/** Clear all play history. */
export function clearHistory(): Promise<void> {
  return invokeCommand<void>('clear_history');
}