/**
 * Album detail store tests.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
  albumDetail,
  loadAlbumDetail,
  clearAlbumDetail,
  isLoadingAlbumDetail,
  albumDetailError,
} from '@features/library/stores/albumDetail';

const mocks = vi.hoisted(() => ({
  getAlbumDetailCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  getAlbumDetail: mocks.getAlbumDetailCmd,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

describe('albumDetail store', () => {
  beforeEach(() => {
    mocks.getAlbumDetailCmd.mockReset();
    clearAlbumDetail();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    clearAlbumDetail();
  });

  it('loads album detail', async () => {
    const detail = {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      cover: 'https://img.test/cover.jpg',
      year: 2001,
      tracks: [],
    };
    mocks.getAlbumDetailCmd.mockResolvedValueOnce(detail);

    await loadAlbumDetail('album:discovery:daft-punk');

    expect(get(albumDetail)).toEqual(detail);
    expect(get(isLoadingAlbumDetail)).toBe(false);
    expect(get(albumDetailError)).toBeNull();
  });

  it('sets error on failure', async () => {
    mocks.getAlbumDetailCmd.mockRejectedValueOnce(new Error('not found'));

    await expect(loadAlbumDetail('album:ghost')).rejects.toThrow('not found');

    expect(get(albumDetail)).toBeNull();
    expect(get(isLoadingAlbumDetail)).toBe(false);
    expect(get(albumDetailError)).toBe('not found');
  });

  it('clears state', () => {
    loadAlbumDetail('album:discovery:daft-punk');
    clearAlbumDetail();

    expect(get(albumDetail)).toBeNull();
    expect(get(albumDetailError)).toBeNull();
  });
});
