/**
 * Deterministic artist/album ID generation.
 *
 * Mirrors the Rust backend normalization so the frontend can derive
 * stable IDs from raw metadata strings without a round-trip.
 */

/** Normalize an artist name into the backend artist ID format. */
export function normalizeArtistId(name: string): string {
  const normalized = name
    .toLowerCase()
    .trim()
    .replace(/\s+/g, '-');
  return `artist:${normalized}`;
}

/** Normalize an album title + artist into the backend album ID format. */
export function normalizeAlbumId(title: string, artist: string): string {
  const normalizedTitle = title
    .toLowerCase()
    .trim()
    .replace(/\s+/g, '-');
  const normalizedArtist = artist
    .toLowerCase()
    .trim()
    .replace(/\s+/g, '-');
  return `album:${normalizedTitle}:${normalizedArtist}`;
}
