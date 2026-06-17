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
 * A favorited track with metadata about when it was added.
 * Matches the Rust `FavoriteEntry` struct with `serde(rename_all = "camelCase")`.
 */
export interface FavoriteEntry {
  track: Track;
  addedAt: string;
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