/**
 * ID normalization utility tests.
 *
 * Verifies deterministic artist/album ID generation matching the Rust backend.
 */
import { describe, it, expect } from 'vitest';
import { normalizeArtistId, normalizeAlbumId } from './ids';

describe('normalizeArtistId', () => {
  it('generates a lowercase slug with the artist prefix', () => {
    expect(normalizeArtistId('Daft Punk')).toBe('artist:daft-punk');
  });

  it('trims whitespace and collapses multiple spaces', () => {
    expect(normalizeArtistId('  The   Beatles  ')).toBe('artist:the-beatles');
  });

  it('handles single-word names', () => {
    expect(normalizeArtistId('Queen')).toBe('artist:queen');
  });

  it('returns the same slug for mixed case', () => {
    expect(normalizeArtistId('qUeEn')).toBe('artist:queen');
  });
});

describe('normalizeAlbumId', () => {
  it('generates a lowercase slug with the album prefix and artist', () => {
    expect(normalizeAlbumId('Discovery', 'Daft Punk')).toBe('album:discovery:daft-punk');
  });

  it('normalizes both title and artist', () => {
    expect(normalizeAlbumId('  A Night at the Opera  ', '  Queen  ')).toBe(
      'album:a-night-at-the-opera:queen',
    );
  });

  it('handles single-word album and artist names', () => {
    expect(normalizeAlbumId('Thriller', 'Michael Jackson')).toBe('album:thriller:michael-jackson');
  });
});
