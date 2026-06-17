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
 * Frequency data payload from the Rust FFT engine.
 * Matches the Rust `FrequencyData` struct with `serde(rename_all = "camelCase")`.
 * Event: "frequency-data"
 */
export interface FrequencyData {
  bins: number[];      // f32 array from FFT, length = fft_size/2
  sampleRate: number;   // u32, matches Rust serde camelCase
  peak: number;         // f32, max bin value for amplitude reference
}