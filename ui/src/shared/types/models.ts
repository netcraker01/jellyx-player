/**
 * Core domain types mirroring ARCHITECTURE.md §4 and Rust models.
 */

export enum Source {
  YouTube = 'YouTube',
  SoundCloud = 'SoundCloud',
  Local = 'Local',
}

export interface Track {
  id: string;
  source: Source;
  sourceId: string;
  title: string;
  artist: string;
  album?: string;
  duration?: number;
  thumbnail?: string;
  streamUrl?: string;
  localPath?: string;
  playlistId?: string;
  metadata: Record<string, string>;
}

export interface Artist {
  id: string;
  name: string;
  thumbnail?: string;
  source: Source;
  sourceId: string;
}

export interface Album {
  id: string;
  title: string;
  artist: string;
  cover?: string;
  year?: number;
  source: Source;
  sourceId: string;
  tracks: string[];
}

/**
 * A playlist of tracks from a remote source (YouTube, SoundCloud, etc.).
 * Matches the Rust `Playlist` struct with `serde(rename_all = "camelCase")`.
 */
export interface Playlist {
  id: string;
  source: Source;
  sourceId: string;
  title: string;
  thumbnail?: string;
  trackCount: number;
  tracks: Track[];
}

/**
 * Frequency data payload decoded from binary FFT frames.
 *
 * Binary frame layout (all little-endian):
 * - Bytes 0-3: sample_rate (u32 LE)
 * - Bytes 4-7: peak (f32 LE)
 * - Bytes 8+: bins (N * f32 LE, N = fft_size/2)
 *
 * The `bins` field is a Float32Array view over the raw buffer,
 * avoiding conversion to number[] for performance at 60fps.
 */
export interface FrequencyData {
  bins: Float32Array;   // f32 array from FFT binary frame, length = fft_size/2
  sampleRate: number;   // u32, decoded from binary frame header
  peak: number;         // f32, max bin value for amplitude reference
}

/**
 * A play history entry with timestamp.
 * Matches the Rust `HistoryEntry` struct with `serde(rename_all = "camelCase")`.
 */
export interface HistoryEntry {
  id: number;
  track: Track;
  playedAt: string;
}

/**
 * Repeat mode values matching the Rust `RepeatMode` enum.
 * Serialized as PascalCase ("Off", "All", "One").
 */
export type RepeatMode = 'Off' | 'All' | 'One';

/**
 * Full queue snapshot from the Rust backend.
 * Matches the Rust `QueueState` struct with `serde(rename_all = "camelCase")`.
 */
export interface QueueState {
  tracks: Track[];
  currentIndex: number | null;
  shuffle: boolean;
  repeatMode: RepeatMode;
  playedIndices: number[];
}

/**
 * A watched folder for the local file scanner.
 * Matches the Rust `WatchedFolder` struct with `serde(rename_all = "camelCase")`.
 */
export interface WatchedFolder {
  path: string;
  lastScannedAt?: string;
  addedAt: string;
}

/**
 * A local track entry from the file scanner.
 * Matches the Rust `LocalTrackEntry` struct with `serde(rename_all = "camelCase")`.
 */
export interface LocalTrackEntry {
  track: Track;
  filePath: string;
  folderPath: string;
  fileModifiedAt?: string;
}

/**
 * Result of a folder scan operation.
 * Matches the Rust `ScanResult` struct with `serde(rename_all = "camelCase")`.
 */
export interface ScanResult {
  folderPath: string;
  filesScanned: number;
  filesAdded: number;
  filesUpdated: number;
  filesSkipped: number;
  errors: number;
}

// ── Grouped search DTOs ───────────────────────────────────────────────

/**
 * Filter for grouped search: limit results to a single entity type.
 * Mirrors the Rust `SearchFilter` enum serialized as lowercase camelCase.
 */
export type SearchFilter = 'songs' | 'artists' | 'albums';

/**
 * Grouped search result returned by `search_grouped`.
 * Matches the Rust `GroupedSearchResult` struct with `serde(rename_all = "camelCase")`.
 */
export interface GroupedSearchResult {
  songs: Track[];
  artists: ArtistSummary[];
  albums: AlbumSummary[];
  /** Whether more song results are available via pagination. */
  hasMoreSongs?: boolean;
}

/**
 * Lightweight artist summary for search results.
 * Matches the Rust `ArtistSummary` struct with `serde(rename_all = "camelCase")`.
 */
export interface ArtistSummary {
  id: string;
  name: string;
  thumbnail?: string;
  trackCount: number;
}

/**
 * Lightweight album summary for search results.
 * Matches the Rust `AlbumSummary` struct with `serde(rename_all = "camelCase")`.
 */
export interface AlbumSummary {
  id: string;
  title: string;
  artist: string;
  cover?: string;
  year?: number;
  trackCount: number;
}

/**
 * Full artist detail for `/artist/:id` view.
 * Matches the Rust `ArtistDetail` struct with `serde(rename_all = "camelCase")`.
 */
export interface ArtistDetail {
  id: string;
  name: string;
  thumbnail?: string;
  topTracks: Track[];
  albums: AlbumSummary[];
}

/**
 * Full album detail for `/album/:id` view.
 * Matches the Rust `AlbumDetail` struct with `serde(rename_all = "camelCase")`.
 */
export interface AlbumDetail {
  id: string;
  title: string;
  artist: string;
  artistId: string;
  cover?: string;
  year?: number;
  tracks: Track[];
}

/**
 * A user-created local playlist.
 * Matches the Rust `UserPlaylist` struct with `serde(rename_all = "camelCase")`.
 */
export interface UserPlaylist {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * A track entry inside a user playlist.
 * Matches the Rust `PlaylistTrackEntry` struct with `serde(rename_all = "camelCase")`.
 */
export interface PlaylistTrackEntry {
  playlistId: string;
  position: number;
  track: Track;
  addedAt: string;
}

/**
 * A favorited artist.
 * Matches the Rust `ArtistFavorite` struct with `serde(rename_all = "camelCase")`.
 */
export interface ArtistFavorite {
  artistId: string;
  artistName: string;
  thumbnail?: string;
  addedAt: string;
}

/**
 * A single recommendation item: a track, artist, or album with a reason.
 * Mirrors the Rust `RecommendationItem` enum (tagged union with type field).
 */
export type RecommendationItem =
  | { type: 'Track'; track: Track; reason: string }
  | { type: 'Artist'; id: string; name: string; thumbnail?: string; trackCount: number; reason: string }
  | { type: 'Album'; id: string; title: string; artist: string; cover?: string; trackCount: number; reason: string };

/**
 * Home snapshot: recently played tracks and recommendations.
 * Matches the Rust `HomeSnapshot` struct with `serde(rename_all = "camelCase")`.
 */
export interface HomeSnapshot {
  recentlyPlayed: HistoryEntry[];
  recommendations: RecommendationItem[];
}

/**
 * A suggestion category for the Discover section.
 * Matches the Rust `SuggestionCategory` struct with `serde(rename_all = "camelCase")`.
 */
export interface SuggestionCategory {
  id: string;
  label: string;
  icon: string;
  query: string;
  color: string;
}