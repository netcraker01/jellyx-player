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
  HistoryEntry,
  WatchedFolder,
  LocalTrackEntry,
  ScanResult,
  GroupedSearchResult,
  ArtistDetail,
  AlbumDetail,
  HomeSnapshot,
  RecommendationItem,
  Playlist,
  UserPlaylist,
  PlaylistTrackEntry,
  ArtistFavorite,
  SuggestionCategory,
} from '@shared/types/models';

// ── Playback commands ──────────────────────────────────────────────

export function play(url: string): Promise<void> {
  return invokeCommand<void>('play', { url });
}

export function playLocal(path: string): Promise<void> {
  return invokeCommand<void>('play_local', { path });
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

/** Set playback volume. `volume` is in backend units (0.0..1.0), matching the
 *  Rust `set_volume(volume: f32)` command. Convert UI 0-100 to 0.0-1.0 in the
 *  player action layer before calling this. */
export function setVolume(volume: number): Promise<void> {
  return invokeCommand<void>('set_volume', { volume });
}

export function search(query: string): Promise<Track[]> {
  return invokeCommand<Track[]>('search', { query });
}

/** Search with grouped results (songs, artists, albums). Optional filter: songs, artists, albums, or none for all.
 *  Pagination: offset and limit control the remote song results slice. */
export function searchGrouped(query: string, filter?: string, offset?: number, limit?: number): Promise<GroupedSearchResult> {
  return invokeCommand<GroupedSearchResult>('search_grouped', {
    query,
    filter: filter ?? null,
    offset: offset ?? null,
    limit: limit ?? null,
  });
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

/** Add a track to the queue using the full Track object — skips slow resolve. */
export function addToQueueWithTrack(track: Track): Promise<void> {
  return invokeCommand<void>('add_to_queue_with_track', { track });
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

/** Insert a track immediately after the current queue position using full Track — skips slow resolve. */
export function playNextWithTrack(track: Track): Promise<void> {
  return invokeCommand<void>('play_next_with_track', { track });
}

export function getQueue(): Promise<QueueState> {
  return invokeCommand<QueueState>('get_queue');
}

export function getVersion(): Promise<string> {
  return invokeCommand<string>('get_version');
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

// ── Home commands ──────────────────────────────────────────────────────

export function getHomeRecommendations(): Promise<RecommendationItem[]> {
  return invokeCommand<RecommendationItem[]>('get_home_recommendations');
}

/** Get the Home snapshot: recently played + recommendations. */
export function getHomeSnapshot(): Promise<HomeSnapshot> {
  return invokeCommand<HomeSnapshot>('get_home_snapshot');
}

/** Get suggestion categories for the Discover section. */
export function getSuggestionCategories(): Promise<SuggestionCategory[]> {
  return invokeCommand<SuggestionCategory[]>('get_suggestion_categories');
}

// ── Streaming & Playlist commands ──────────────────────────────────

/** Play a remote track by resolving its stream URL. */
export function playStream(track: Track): Promise<void> {
  return invokeCommand<void>('play_stream', { track });
}

/** Download a remote stream URL to a local cache file for instant seeking.
 * Returns the absolute path to the cached file. */
export function cacheRemoteStream(cacheId: string, remoteUrl: string): Promise<string> {
  return invokeCommand<string>('cache_remote_stream', { cacheId, remoteUrl });
}

/** Search for playlists across all registered sources. */
export function searchPlaylists(query: string): Promise<Playlist[]> {
  return invokeCommand<Playlist[]>('search_playlists', { query });
}

/** Resolve a full playlist by source and URL. */
export function resolvePlaylist(source: string, url: string): Promise<Playlist> {
  return invokeCommand<Playlist>('resolve_playlist', { source, url });
}

/** Play all tracks in a playlist, replacing the current queue. */
export function playPlaylist(source: string, url: string): Promise<void> {
  return invokeCommand<void>('play_playlist', { source, url });
}

/** Resolve a track's stream URL without starting playback. */
export function resolveTrack(source: string, id: string): Promise<Track> {
  return invokeCommand<Track>('resolve_track', { source, id });
}

// ── User Playlist commands ────────────────────────────────────────

export function createPlaylist(title: string): Promise<UserPlaylist> {
  return invokeCommand<UserPlaylist>('create_playlist', { title });
}
export function renamePlaylist(id: string, title: string): Promise<void> {
  return invokeCommand<void>('rename_playlist', { id, title });
}
export function deletePlaylist(id: string): Promise<void> {
  return invokeCommand<void>('delete_playlist', { id });
}
export function getAllPlaylists(): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('get_all_playlists');
}
export function getRecentPlaylists(limit?: number): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('get_recent_playlists', { limit: limit ?? null });
}
export function searchUserPlaylists(query: string): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('search_user_playlists', { query });
}
export function addTrackToPlaylist(playlistId: string, track: Track): Promise<void> {
  return invokeCommand<void>('add_track_to_playlist', { playlistId, track });
}
export function addTracksToPlaylist(playlistId: string, tracks: Track[]): Promise<number> {
  return invokeCommand<number>('add_tracks_to_playlist', { playlistId, tracks });
}
export function removeTrackFromPlaylist(playlistId: string, position: number): Promise<void> {
  return invokeCommand<void>('remove_track_from_playlist', { playlistId, position });
}
export function getPlaylistTracks(playlistId: string): Promise<PlaylistTrackEntry[]> {
  return invokeCommand<PlaylistTrackEntry[]>('get_playlist_tracks', { playlistId });
}

/** Get track count for a playlist (without loading all tracks). */
export function countPlaylistTracks(playlistId: string): Promise<number> {
  return invokeCommand<number>('count_playlist_tracks', { playlistId });
}

/** Get up to 4 thumbnail URLs from a playlist's tracks for cover art. */
export function getPlaylistThumbnails(playlistId: string): Promise<string[]> {
  return invokeCommand<string[]>('get_playlist_thumbnails', { playlistId });
}

/** Generate one playlist per artist from the local track catalog (idempotent).
 *  Returns the playlists that were created or had tracks added. */
export function generateArtistPlaylists(): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('generate_artist_playlists');
}

/** Generate folder-as-playlist hierarchy for a watched folder (idempotent).
 *  Returns all parent + child playlists touched or existing. */
export function generateFolderPlaylists(folderPath: string): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('generate_folder_playlists', { folderPath });
}

/** Get all playlists generated from a watched folder (parent + children). */
export function getPlaylistsBySourceFolder(folderPath: string): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('get_playlists_by_source_folder', { folderPath });
}

/** Get child playlists of a parent playlist (folder-as-playlist children). */
export function getChildPlaylists(parentId: string): Promise<UserPlaylist[]> {
  return invokeCommand<UserPlaylist[]>('get_child_playlists', { parentId });
}

// ── Artist Favorite commands ─────────────────────────────────────

export function addArtistFavorite(
  artistId: string,
  artistName: string,
  thumbnail?: string,
  source?: string,
  sourceArtistRef?: string,
): Promise<void> {
  return invokeCommand<void>('add_artist_favorite', {
    artistId,
    artistName,
    thumbnail: thumbnail ?? null,
    source: source ?? null,
    sourceArtistRef: sourceArtistRef ?? null,
  });
}
export function removeArtistFavorite(artistId: string, source?: string): Promise<void> {
  return invokeCommand<void>('remove_artist_favorite', {
    artistId,
    source: source ?? null,
  });
}
export function isArtistFavorite(artistId: string, source?: string): Promise<boolean> {
  return invokeCommand<boolean>('is_artist_favorite', {
    artistId,
    source: source ?? null,
  });
}
export function getAllArtistFavorites(): Promise<ArtistFavorite[]> {
  return invokeCommand<ArtistFavorite[]>('get_all_artist_favorites');
}

// ── Source Settings commands ────────────────────────────────────────

export interface SourceSetting {
  source: string;
  enabled: boolean;
  label: string;
}

/** Get all source settings (YouTube, SoundCloud), defaulting to enabled. */
export function getSourceSettings(): Promise<SourceSetting[]> {
  return invokeCommand<SourceSetting[]>('get_source_settings');
}

/** Enable or disable a source plugin. */
export function setSourceEnabled(source: string, enabled: boolean): Promise<void> {
  return invokeCommand<void>('set_source_enabled', { source, enabled });
}

// ── Audio Settings commands ────────────────────────────────────────

export interface AudioSettings {
  normalizeAudio: boolean;
}

/** Get audio settings (normalization toggle, etc.). */
export function getAudioSettings(): Promise<AudioSettings> {
  return invokeCommand<AudioSettings>('get_audio_settings');
}

/** Enable or disable audio normalization (persists to DB). */
export function setNormalizeAudio(enabled: boolean): Promise<void> {
  return invokeCommand<void>('set_normalize_audio', { enabled });
}

/** Set normalization on the local playback backend (immediate effect). */
export function setPlaybackNormalizeAudio(enabled: boolean): Promise<void> {
  return invokeCommand<void>('set_playback_normalize_audio', { enabled });
}