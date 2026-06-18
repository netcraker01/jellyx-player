/**
 * Typed Tauri command wrappers.
 *
 * These are thin wrappers around invokeCommand that add type safety.
 * All command names match the Rust #[tauri::command] function names.
 * Parameters use camelCase to match Tauri's IPC serialization.
 */

import { invokeCommand } from './tauri';
import type {
  Track,
  QueueState,
  FavoriteEntry,
  HistoryEntry,
  WatchedFolder,
  LocalTrackEntry,
  ScanResult,
  GroupedSearchResult,
  ArtistDetail,
  AlbumDetail,
} from '@shared/types/models';

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

/** Search with grouped results (songs, artists, albums). Optional filter: songs, artists, albums, or none for all. */
export function searchGrouped(query: string, filter?: string): Promise<GroupedSearchResult> {
  return invokeCommand<GroupedSearchResult>('search_grouped', { query, filter: filter ?? null });
}

/** Get full artist detail by artist ID. */
export function getArtistDetail(id: string): Promise<ArtistDetail> {
  return invokeCommand<ArtistDetail>('get_artist_detail', { id });
}

/** Get full album detail by album ID. */
export function getAlbumDetail(id: string): Promise<AlbumDetail> {
  return invokeCommand<AlbumDetail>('get_album_detail', { id });
}

/** Play all tracks in an album, replacing the current queue. */
export function playAlbum(albumId: string): Promise<void> {
  return invokeCommand<void>('play_album', { albumId });
}

export function addToQueue(trackId: string): Promise<void> {
  return invokeCommand<void>('add_to_queue', { trackId });
}

/** Remove a track from the queue by its Helix track ID. */
export function removeFromQueue(trackId: string): Promise<void> {
  return invokeCommand<void>('remove_from_queue', { trackId });
}

/** Clear the entire queue and stop playback. */
export function clearQueue(): Promise<void> {
  return invokeCommand<void>('clear_queue');
}

/** Insert a selected track immediately after the current queue position. */
export function playNext(trackId: string): Promise<void> {
  return invokeCommand<void>('play_next', { trackId });
}

export function getQueue(): Promise<QueueState> {
  return invokeCommand<QueueState>('get_queue');
}

export function getVersion(): Promise<string> {
  return invokeCommand<string>('get_version');
}

/** Toggle favorite state for a track. Returns true if now favorited, false if removed. */
export function toggleFavorite(trackId: string): Promise<boolean> {
  return invokeCommand<boolean>('toggle_favorite', { trackId });
}

/** Check whether a track is currently favorited. */
export function isFavorite(trackId: string): Promise<boolean> {
  return invokeCommand<boolean>('is_favorite', { trackId });
}

/** Set shuffle mode on or off. */
export function setShuffle(enabled: boolean): Promise<void> {
  return invokeCommand<void>('set_shuffle', { enabled });
}

/** Set repeat mode by name ("Off", "All", or "One"). */
export function setRepeat(mode: string): Promise<void> {
  return invokeCommand<void>('set_repeat', { mode });
}

/** Cycle repeat mode Off -> All -> One -> Off. Returns the new mode name. */
export function cycleRepeat(): Promise<string> {
  return invokeCommand<string>('cycle_repeat');
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

/** Get play history, ordered by most recent first (max 100). */
export function getHistory(): Promise<HistoryEntry[]> {
  return invokeCommand<HistoryEntry[]>('get_history');
}

/** Clear all play history. */
export function clearHistory(): Promise<void> {
  return invokeCommand<void>('clear_history');
}

// ── Local Scanner commands ──────────────────────────────────────────

/** Scan a folder for audio files and add to local library. */
export function scanFolder(folderPath: string): Promise<ScanResult> {
  return invokeCommand<ScanResult>('scan_folder', { folderPath });
}

/** Get all local tracks, optionally filtered by folder path. */
export function getLocalTracks(folderPath?: string): Promise<LocalTrackEntry[]> {
  return invokeCommand<LocalTrackEntry[]>('get_local_tracks', { folderPath: folderPath ?? null });
}

/** Get all watched folders. */
export function getWatchedFolders(): Promise<WatchedFolder[]> {
  return invokeCommand<WatchedFolder[]>('get_watched_folders');
}

/** Remove a watched folder and its associated tracks. */
export function removeWatchedFolder(folderPath: string): Promise<void> {
  return invokeCommand<void>('remove_watched_folder', { folderPath });
}