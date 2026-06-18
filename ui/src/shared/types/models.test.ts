/**
 * Models type tests.
 *
 * Verifies the shape of new artist/album search DTO types
 * introduced for grouped search and detail views.
 */
import { describe, it, expect } from 'vitest';
import { Source } from '@shared/types/models';
import type {
  GroupedSearchResult,
  ArtistSummary,
  AlbumSummary,
  ArtistDetail,
  AlbumDetail,
  SearchFilter,
} from '@shared/types/models';

describe('Grouped search DTO types', () => {
  it('builds a GroupedSearchResult with songs, artists, and albums', () => {
    const track = {
      id: 'track:daft-punk:one-more-time',
      source: Source.YouTube,
      sourceId: 'yt-123',
      title: 'One More Time',
      artist: 'Daft Punk',
      album: 'Discovery',
      duration: 320,
      thumbnail: 'https://img.test/daft.jpg',
      metadata: {},
    };

    const artist: ArtistSummary = {
      id: 'artist:daft-punk',
      name: 'Daft Punk',
      thumbnail: 'https://img.test/artist.jpg',
      trackCount: 12,
    };

    const album: AlbumSummary = {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      cover: 'https://img.test/cover.jpg',
      year: 2001,
      trackCount: 14,
    };

    const result: GroupedSearchResult = {
      songs: [track],
      artists: [artist],
      albums: [album],
    };

    expect(result.songs).toHaveLength(1);
    expect(result.artists[0].trackCount).toBe(12);
    expect(result.albums[0].year).toBe(2001);
  });

  it('allows optional thumbnail/cover/year fields to be omitted', () => {
    const artist: ArtistSummary = {
      id: 'artist:queen',
      name: 'Queen',
      trackCount: 10,
    };

    const album: AlbumSummary = {
      id: 'album:greatest-hits:queen',
      title: 'Greatest Hits',
      artist: 'Queen',
      trackCount: 17,
    };

    expect(artist.thumbnail).toBeUndefined();
    expect(album.cover).toBeUndefined();
    expect(album.year).toBeUndefined();
  });
});

describe('Detail DTO types', () => {
  it('builds an ArtistDetail with top tracks and albums', () => {
    const detail: ArtistDetail = {
      id: 'artist:queen',
      name: 'Queen',
      thumbnail: 'https://img.test/queen.jpg',
      topTracks: [],
      albums: [
        {
          id: 'album:greatest-hits:queen',
          title: 'Greatest Hits',
          artist: 'Queen',
          year: 1981,
          trackCount: 17,
        },
      ],
    };

    expect(detail.topTracks).toEqual([]);
    expect(detail.albums).toHaveLength(1);
  });

  it('builds an AlbumDetail with tracks', () => {
    const detail: AlbumDetail = {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      cover: 'https://img.test/cover.jpg',
      year: 2001,
      tracks: [],
    };

    expect(detail.artistId).toBe('artist:daft-punk');
    expect(detail.tracks).toEqual([]);
  });
});

describe('SearchFilter type', () => {
  it('accepts valid filter values', () => {
    const songs: SearchFilter = 'songs';
    const artists: SearchFilter = 'artists';
    const albums: SearchFilter = 'albums';

    expect([songs, artists, albums]).toEqual(['songs', 'artists', 'albums']);
  });
});
