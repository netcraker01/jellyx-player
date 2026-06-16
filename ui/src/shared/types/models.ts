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