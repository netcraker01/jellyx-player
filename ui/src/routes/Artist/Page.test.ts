/**
 * Artist detail page tests.
 *
 * Verifies loading, error, and detail states, plus navigation wiring.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  loadArtistDetail: vi.fn(),
  clearArtistDetail: vi.fn(),
  navigate: vi.fn(),
  playTrack: vi.fn(),
  addToQueueAction: vi.fn(),
  playNextAction: vi.fn(),
  favoriteAdd: vi.fn(),
  storeSubscribe: vi.fn(),
  loadingSubscribe: vi.fn(),
  errorSubscribe: vi.fn(),
}));

const artistStoreMock = vi.hoisted(() => {
  const createWritable = (initial: any) => {
    let value = initial;
    const subs = new Set<(v: any) => void>();
    return {
      subscribe(fn: (v: any) => void) {
        fn(value);
        subs.add(fn);
        return () => subs.delete(fn);
      },
      set(v: any) {
        value = v;
        subs.forEach((fn) => fn(v));
      },
      get() {
        return value;
      },
    };
  };

  const artistDetailWritable = createWritable<any>(null);

  return {
    artistDetail: {
      subscribe: artistDetailWritable.subscribe,
      set: artistDetailWritable.set,
      load: mocks.loadArtistDetail,
      clear: mocks.clearArtistDetail,
    },
    isLoadingArtistDetail: createWritable(false),
    artistDetailError: createWritable<string | null>(null),
  };
});

vi.mock('@features/library/stores/artistDetail', () => ({
  artistDetail: artistStoreMock.artistDetail,
  isLoadingArtistDetail: artistStoreMock.isLoadingArtistDetail,
  artistDetailError: artistStoreMock.artistDetailError,
}));

vi.mock('svelte-routing', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: mocks.playTrack,
  addToQueueAction: mocks.addToQueueAction,
  playNextAction: mocks.playNextAction,
}));

vi.mock('@features/favorites/stores/favorites', () => ({
  favorites: { add: mocks.favoriteAdd },
}));

import { translations } from '@i18n';
import ArtistPage from './Page.svelte';

function setStore(loading: boolean, error: string | null, detail: any) {
  artistStoreMock.isLoadingArtistDetail.set(loading);
  artistStoreMock.artistDetailError.set(error);
  artistStoreMock.artistDetail.set(detail);
}

describe('ArtistPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    translations.set({
      routes: { artist: 'Artist' },
      common: { loading: 'Loading...', back: 'Back' },
      artist: {
        top_tracks: 'Top tracks',
        albums: 'Albums',
        not_found: 'Artist not found',
      },
    });
  });

  afterEach(() => {
    setStore(false, null, null);
  });

  it('loads artist detail on mount when an id is provided', async () => {
    mocks.loadArtistDetail.mockResolvedValueOnce(undefined);
    render(ArtistPage, { props: { id: 'artist:daft-punk' } });
    await waitFor(() => {
      expect(mocks.loadArtistDetail).toHaveBeenCalledWith('artist:daft-punk');
    });
  });

  it('renders loading state', () => {
    setStore(true, null, null);
    const { container } = render(ArtistPage, { props: { id: 'artist:daft-punk' } });
    expect(container.textContent).toContain('Loading...');
  });

  it('renders error state when artist is not found', () => {
    setStore(false, 'Artist not found', null);
    const { container } = render(ArtistPage, { props: { id: 'artist:missing' } });
    expect(container.textContent).toContain('Artist not found');
  });

  it('renders artist header with name and top tracks/albums sections', () => {
    setStore(false, null, {
      id: 'artist:daft-punk',
      name: 'Daft Punk',
      thumbnail: '/art/daft.jpg',
      topTracks: [
        {
          id: 'track:1',
          source: 'Local',
          sourceId: 'local-1',
          title: 'One More Time',
          artist: 'Daft Punk',
          duration: 320,
          thumbnail: '/art/one.jpg',
          metadata: {},
        },
      ],
      albums: [
        {
          id: 'album:discovery:daft-punk',
          title: 'Discovery',
          artist: 'Daft Punk',
          trackCount: 14,
          year: 2001,
        },
      ],
    });
    const { container } = render(ArtistPage, { props: { id: 'artist:daft-punk' } });
    expect(container.textContent).toContain('Daft Punk');
    expect(container.textContent).toContain('Top tracks');
    expect(container.textContent).toContain('One More Time');
    expect(container.textContent).toContain('Albums');
    expect(container.textContent).toContain('Discovery');
  });

  it('renders placeholder when artist has no thumbnail', () => {
    setStore(false, null, {
      id: 'artist:unknown',
      name: 'Unknown Artist',
      topTracks: [],
      albums: [],
    });
    const { container } = render(ArtistPage, { props: { id: 'artist:unknown' } });
    expect(container.querySelector('.artist-header-art')).toBeTruthy();
  });

  it('navigates to album page when an album card is clicked', async () => {
    setStore(false, null, {
      id: 'artist:daft-punk',
      name: 'Daft Punk',
      topTracks: [],
      albums: [
        {
          id: 'album:discovery:daft-punk',
          title: 'Discovery',
          artist: 'Daft Punk',
          trackCount: 14,
        },
      ],
    });
    mocks.navigate.mockImplementation(() => {});
    const { container } = render(ArtistPage, { props: { id: 'artist:daft-punk' } });
    const card = container.querySelector('.album-card') as HTMLElement;
    expect(card).toBeTruthy();
    await fireEvent.click(card);
    expect(mocks.navigate).toHaveBeenCalledWith('/album/album%3Adiscovery%3Adaft-punk');
  });
});
