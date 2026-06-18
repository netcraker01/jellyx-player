/**
 * Grouped search store tests.
 *
 * Verifies search, clear, loading, and error state for the
 * grouped search results store.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
  searchGrouped,
  clearSearchGrouped,
  groupedSearchResults,
  isSearchingGrouped,
  groupedSearchError,
} from '@features/search/stores/searchGrouped';

const mocks = vi.hoisted(() => ({
  searchGroupedCmd: vi.fn(),
  getArtistDetailCmd: vi.fn(),
  getAlbumDetailCmd: vi.fn(),
  playAlbumCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  searchGrouped: mocks.searchGroupedCmd,
  getArtistDetail: mocks.getArtistDetailCmd,
  getAlbumDetail: mocks.getAlbumDetailCmd,
  playAlbum: mocks.playAlbumCmd,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

describe('groupedSearchResults store', () => {
  beforeEach(() => {
    mocks.searchGroupedCmd.mockReset();
    clearSearchGrouped();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    clearSearchGrouped();
  });

  it('loads grouped results via search()', async () => {
    const result = {
      songs: [
        {
          id: 'track:1',
          source: 'YouTube',
          sourceId: 'yt-1',
          title: 'One More Time',
          artist: 'Daft Punk',
          metadata: {},
        },
      ],
      artists: [{ id: 'artist:daft-punk', name: 'Daft Punk', trackCount: 5 }],
      albums: [{ id: 'album:discovery:daft-punk', title: 'Discovery', artist: 'Daft Punk', trackCount: 14 }],
    };
    mocks.searchGroupedCmd.mockResolvedValueOnce(result);

    await searchGrouped('daft');

    expect(get(groupedSearchResults)).toEqual(result);
    expect(get(isSearchingGrouped)).toBe(false);
    expect(get(groupedSearchError)).toBeNull();
    expect(mocks.searchGroupedCmd).toHaveBeenCalledWith('daft', undefined);
  });

  it('passes filter to searchGrouped command', async () => {
    mocks.searchGroupedCmd.mockResolvedValueOnce({ songs: [], artists: [], albums: [] });

    await searchGrouped('daft', 'artists');

    expect(mocks.searchGroupedCmd).toHaveBeenCalledWith('daft', 'artists');
  });

  it('sets loading and error state on failure', async () => {
    mocks.searchGroupedCmd.mockRejectedValueOnce(new Error('backend down'));

    await expect(searchGrouped('daft')).rejects.toThrow('backend down');

    expect(get(groupedSearchResults)).toBeNull();
    expect(get(isSearchingGrouped)).toBe(false);
    expect(get(groupedSearchError)).toBe('backend down');
  });

  it('clears results, error, and loading state', async () => {
    mocks.searchGroupedCmd.mockResolvedValueOnce({ songs: [], artists: [], albums: [] });
    await searchGrouped('daft');

    clearSearchGrouped();

    expect(get(groupedSearchResults)).toBeNull();
    expect(get(groupedSearchError)).toBeNull();
    expect(get(isSearchingGrouped)).toBe(false);
  });
});
