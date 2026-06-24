/**
 * Album detail page tests.
 *
 * Verifies loading, error, and detail states, plus play-album and navigation.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  loadAlbumDetail: vi.fn(),
  clearAlbumDetail: vi.fn(),
  playAlbum: vi.fn(),
  navigate: vi.fn(),
  playTrack: vi.fn(),
  addToQueueAction: vi.fn(),
  playNextAction: vi.fn(),
  notifyPush: vi.fn(),
  notifyGet: vi.fn(),
}));

const albumStoreMock = vi.hoisted(() => {
  const createWritable = <T>(initial: T) => {
    let value = initial;
    const subs = new Set<(v: T) => void>();
    return {
      subscribe(fn: (v: T) => void) {
        fn(value);
        subs.add(fn);
        return () => subs.delete(fn);
      },
      set(v: T) {
        value = v;
        subs.forEach((fn) => fn(v));
      },
      get() {
        return value;
      },
    };
  };

  const albumDetailWritable = createWritable<any>(null);

  return {
    albumDetail: {
      subscribe: albumDetailWritable.subscribe,
      set: albumDetailWritable.set,
      load: mocks.loadAlbumDetail,
      clear: mocks.clearAlbumDetail,
    },
    isLoadingAlbumDetail: createWritable(false),
    albumDetailError: createWritable<string | null>(null),
  };
});

vi.mock('@features/library/stores/albumDetail', () => ({
  albumDetail: albumStoreMock.albumDetail,
  isLoadingAlbumDetail: albumStoreMock.isLoadingAlbumDetail,
  albumDetailError: albumStoreMock.albumDetailError,
}));

vi.mock('@services/commands', () => ({
  playAlbum: mocks.playAlbum,
}));

vi.mock('@app/router/navigation', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: mocks.playTrack,
  addToQueueAction: mocks.addToQueueAction,
  playNextAction: mocks.playNextAction,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: mocks.notifyPush,
  },
}));

import { translations } from '@i18n';
import AlbumPage from './Page.svelte';

function setStore(loading: boolean, error: string | null, detail: any) {
  albumStoreMock.isLoadingAlbumDetail.set(loading);
  albumStoreMock.albumDetailError.set(error);
  albumStoreMock.albumDetail.set(detail);
}

describe('AlbumPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    translations.set({
      routes: { album: 'Album' },
      common: { loading: 'Loading...', back: 'Back' },
      album: {
        play_album: 'Play Album',
        tracks: 'Tracks',
        not_found: 'Album not found',
      },
    });
  });

  afterEach(() => {
    setStore(false, null, null);
  });

  it('loads album detail on mount when an id is provided', async () => {
    mocks.loadAlbumDetail.mockResolvedValueOnce(undefined);
    render(AlbumPage, { props: { id: 'album:discovery:daft-punk' } });
    await waitFor(() => {
      expect(mocks.loadAlbumDetail).toHaveBeenCalledWith('album:discovery:daft-punk');
    });
  });

  it('renders loading state', () => {
    setStore(true, null, null);
    const { container } = render(AlbumPage, { props: { id: 'album:discovery:daft-punk' } });
    expect(container.textContent).toContain('Loading...');
  });

  it('renders error state when album is not found', () => {
    setStore(false, 'Album not found', null);
    const { container } = render(AlbumPage, { props: { id: 'album:missing' } });
    expect(container.textContent).toContain('Album not found');
  });

  it('renders album header with title, artist link, year, and tracks', () => {
    setStore(false, null, {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      cover: '/art/discovery.jpg',
      year: 2001,
      tracks: [
        {
          id: 'track:1',
          source: 'Local',
          sourceId: 'local-1',
          title: 'One More Time',
          artist: 'Daft Punk',
          duration: 320,
          metadata: {},
        },
        {
          id: 'track:2',
          source: 'Local',
          sourceId: 'local-2',
          title: 'Aerodynamic',
          artist: 'Daft Punk',
          duration: 212,
          metadata: {},
        },
      ],
    });
    const { container } = render(AlbumPage, { props: { id: 'album:discovery:daft-punk' } });
    expect(container.textContent).toContain('Discovery');
    expect(container.textContent).toContain('Daft Punk');
    expect(container.textContent).toContain('2001');
    expect(container.textContent).toContain('One More Time');
    expect(container.textContent).toContain('Aerodynamic');
  });

  it('renders placeholder when album has no cover', () => {
    setStore(false, null, {
      id: 'album:unknown:artist',
      title: 'Unknown Album',
      artist: 'Unknown Artist',
      artistId: 'artist:unknown-artist',
      tracks: [],
    });
    const { container } = render(AlbumPage, { props: { id: 'album:unknown:artist' } });
    expect(container.querySelector('.album-cover-art')).toBeTruthy();
  });

  it('calls playAlbum when the play album button is clicked', async () => {
    mocks.playAlbum.mockResolvedValueOnce(undefined);
    setStore(false, null, {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      tracks: [],
    });
    const { container } = render(AlbumPage, { props: { id: 'album:discovery:daft-punk' } });
    const btn = container.querySelector('.play-album-btn') as HTMLElement;
    expect(btn).toBeTruthy();
    await fireEvent.click(btn);
    expect(mocks.playAlbum).toHaveBeenCalledWith('album:discovery:daft-punk');
  });

  it('shows a toast when playAlbum fails', async () => {
    mocks.playAlbum.mockRejectedValueOnce(new Error('Playback failed'));
    setStore(false, null, {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      tracks: [],
    });
    const { container } = render(AlbumPage, { props: { id: 'album:discovery:daft-punk' } });
    const btn = container.querySelector('.play-album-btn') as HTMLElement;
    await fireEvent.click(btn);
    await waitFor(() => {
      expect(mocks.notifyPush).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', title: 'Playback Error' }),
      );
    });
  });

  it('navigates to artist page when artist name is clicked', async () => {
    setStore(false, null, {
      id: 'album:discovery:daft-punk',
      title: 'Discovery',
      artist: 'Daft Punk',
      artistId: 'artist:daft-punk',
      tracks: [],
    });
    mocks.navigate.mockImplementation(() => {});
    const { container } = render(AlbumPage, { props: { id: 'album:discovery:daft-punk' } });
    const link = container.querySelector('.artist-link') as HTMLElement;
    expect(link).toBeTruthy();
    await fireEvent.click(link);
    expect(mocks.navigate).toHaveBeenCalledWith('/artist/artist%3Adaft-punk');
  });
});
