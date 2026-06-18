/**
 * Artist detail store tests.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
  artistDetail,
  loadArtistDetail,
  clearArtistDetail,
  isLoadingArtistDetail,
  artistDetailError,
} from '@features/library/stores/artistDetail';

const mocks = vi.hoisted(() => ({
  getArtistDetailCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  getArtistDetail: mocks.getArtistDetailCmd,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

describe('artistDetail store', () => {
  beforeEach(() => {
    mocks.getArtistDetailCmd.mockReset();
    clearArtistDetail();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    clearArtistDetail();
  });

  it('loads artist detail', async () => {
    const detail = {
      id: 'artist:queen',
      name: 'Queen',
      thumbnail: 'https://img.test/queen.jpg',
      topTracks: [],
      albums: [],
    };
    mocks.getArtistDetailCmd.mockResolvedValueOnce(detail);

    await loadArtistDetail('artist:queen');

    expect(get(artistDetail)).toEqual(detail);
    expect(get(isLoadingArtistDetail)).toBe(false);
    expect(get(artistDetailError)).toBeNull();
  });

  it('sets error on failure', async () => {
    mocks.getArtistDetailCmd.mockRejectedValueOnce(new Error('not found'));

    await expect(loadArtistDetail('artist:ghost')).rejects.toThrow('not found');

    expect(get(artistDetail)).toBeNull();
    expect(get(isLoadingArtistDetail)).toBe(false);
    expect(get(artistDetailError)).toBe('not found');
  });

  it('clears state', () => {
    loadArtistDetail('artist:queen');
    clearArtistDetail();

    expect(get(artistDetail)).toBeNull();
    expect(get(artistDetailError)).toBeNull();
  });
});
