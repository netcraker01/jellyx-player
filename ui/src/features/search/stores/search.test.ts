/**
 * Playlist search store tests.
 *
 * Verifies the playlist search store calls searchPlaylists command
 * and manages loading/error state correctly.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
  playlistResults,
  isSearchingPlaylists,
  searchError,
} from '@features/search/stores/search';
import { Source } from '@shared/types/models';
import type { Playlist } from '@shared/types/models';

const mocks = vi.hoisted(() => ({
  searchPlaylistsCmd: vi.fn(),
  searchCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  search: mocks.searchCmd,
  searchPlaylists: mocks.searchPlaylistsCmd,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

describe('playlistResults store', () => {
  beforeEach(() => {
    mocks.searchPlaylistsCmd.mockReset();
    playlistResults.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    playlistResults.clear();
  });

  it('loads playlist results via searchPlaylists()', async () => {
    const playlists: Playlist[] = [
      {
        id: 'playlist:yt-1',
        source: Source.YouTube,
        sourceId: 'https://youtube.com/playlist?list=PL123',
        title: 'Daft Punk Essentials',
        trackCount: 25,
        tracks: [],
      },
    ];
    mocks.searchPlaylistsCmd.mockResolvedValueOnce(playlists);

    await playlistResults.searchPlaylists('daft punk');

    expect(get(playlistResults)).toEqual(playlists);
    expect(get(isSearchingPlaylists)).toBe(false);
    expect(get(searchError)).toBeNull();
    expect(mocks.searchPlaylistsCmd).toHaveBeenCalledWith('daft punk');
  });

  it('sets error state on searchPlaylists failure', async () => {
    mocks.searchPlaylistsCmd.mockRejectedValueOnce(new Error('network error'));

    await playlistResults.searchPlaylists('daft');

    expect(get(playlistResults)).toEqual([]);
    expect(get(isSearchingPlaylists)).toBe(false);
    expect(get(searchError)).toBe('network error');
  });

  it('clears playlist results', async () => {
    mocks.searchPlaylistsCmd.mockResolvedValueOnce([
      {
        id: 'playlist:yt-1',
        source: Source.YouTube,
        sourceId: 'pl-1',
        title: 'Test Playlist',
        trackCount: 10,
        tracks: [],
      },
    ]);
    await playlistResults.searchPlaylists('test');

    playlistResults.clear();

    expect(get(playlistResults)).toEqual([]);
  });

  it('skips search for empty query', async () => {
    await playlistResults.searchPlaylists('   ');

    expect(mocks.searchPlaylistsCmd).not.toHaveBeenCalled();
    expect(get(playlistResults)).toEqual([]);
  });
});