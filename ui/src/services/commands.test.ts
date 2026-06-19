/**
 * Command wrapper tests for grouped search and detail views.
 *
 * Verifies typed wrappers invoke the matching Rust command names.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  searchGrouped,
  getArtistDetail,
  getAlbumDetail,
  playAlbum,
} from '@services/commands';

const mocks = vi.hoisted(() => ({
  invokeCommand: vi.fn(),
}));

vi.mock('@services/tauri', () => ({
  invokeCommand: mocks.invokeCommand,
}));

describe('Grouped search commands', () => {
  beforeEach(() => {
    mocks.invokeCommand.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('searchGrouped invokes search_grouped with query only', async () => {
    const expected = {
      songs: [],
      artists: [{ id: 'artist:queen', name: 'Queen', trackCount: 10 }],
      albums: [],
    };
    mocks.invokeCommand.mockResolvedValueOnce(expected);

    const result = await searchGrouped('queen');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('search_grouped', { query: 'queen', filter: null });
    expect(result).toEqual(expected);
  });

  it('searchGrouped passes filter when provided', async () => {
    mocks.invokeCommand.mockResolvedValueOnce({ songs: [], artists: [], albums: [] });

    await searchGrouped('daft', 'artists');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('search_grouped', {
      query: 'daft',
      filter: 'artists',
    });
  });

  it('getArtistDetail invokes get_artist_detail with id', async () => {
    const expected = {
      id: 'artist:queen',
      name: 'Queen',
      thumbnail: 'https://img.test/queen.jpg',
      topTracks: [],
      albums: [],
    };
    mocks.invokeCommand.mockResolvedValueOnce(expected);

    const result = await getArtistDetail('artist:queen');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('get_artist_detail', { id: 'artist:queen' });
    expect(result).toEqual(expected);
  });

  it('getAlbumDetail invokes get_album_detail with id', async () => {
    const expected = {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      cover: 'https://img.test/cover.jpg',
      year: 2001,
      tracks: [],
    };
    mocks.invokeCommand.mockResolvedValueOnce(expected);

    const result = await getAlbumDetail('album:discovery:daft-punk');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('get_album_detail', {
      id: 'album:discovery:daft-punk',
    });
    expect(result).toEqual(expected);
  });

  it('playAlbum invokes play_album with albumId', async () => {
    mocks.invokeCommand.mockResolvedValueOnce(undefined);

    await playAlbum('album:discovery:daft-punk');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('play_album', { albumId: 'album:discovery:daft-punk' });
  });

  it('play invokes play with url', async () => {
    mocks.invokeCommand.mockResolvedValueOnce(undefined);

    const { play } = await import('@services/commands');
    await play('https://stream.test/track.mp3');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('play', {
      url: 'https://stream.test/track.mp3',
    });
  });

  it('playLocal invokes play_local with path', async () => {
    mocks.invokeCommand.mockResolvedValueOnce(undefined);

    const { playLocal } = await import('@services/commands');
    await playLocal('/music/track.mp3');

    expect(mocks.invokeCommand).toHaveBeenCalledWith('play_local', {
      path: '/music/track.mp3',
    });
  });
});
