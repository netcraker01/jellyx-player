/**
 * NowPlayingInfo component tests.
 *
 * Verifies artist/album navigation links are rendered and call navigate.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  favoriteToggle: vi.fn(),
  favoritedValue: false,
}));

vi.mock('svelte-routing', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@features/favorites/stores/favorites', () => ({
  favorites: {
    toggle: mocks.favoriteToggle,
  },
}));

vi.mock('@features/player/stores/player', () => ({
  isCurrentTrackFavorited: {
    subscribe(fn: (v: boolean) => void) {
      fn(mocks.favoritedValue);
      return () => {};
    },
  },
}));

vi.mock('@shared/utils/assetUrl', () => ({
  albumArtUrl: (path: string | undefined) => path,
}));

import NowPlayingInfo from './NowPlayingInfo.svelte';
import { Source } from '@shared/types/models';

const trackWithMetadata = {
  id: 'track:1',
  source: Source.Local,
  sourceId: 'local-1',
  title: 'One More Time',
  artist: 'Daft Punk',
  album: 'Discovery',
  duration: 320,
  thumbnail: '/art/one.jpg',
  metadata: {},
};

const trackWithoutAlbum = {
  id: 'track:2',
  source: Source.Local,
  sourceId: 'local-2',
  title: 'Bohemian Rhapsody',
  artist: 'Queen',
  duration: 354,
  metadata: {},
};

const trackWithoutArtist = {
  id: 'track:3',
  source: Source.Local,
  sourceId: 'local-3',
  title: 'Instrumental',
  artist: '',
  duration: 180,
  metadata: {},
};

describe('NowPlayingInfo', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders clickable artist link when artist metadata is present', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithMetadata } });
    const artistLink = container.querySelector('.track-artist.link') as HTMLElement;
    expect(artistLink).toBeTruthy();
    expect(artistLink.textContent).toContain('Daft Punk');
  });

  it('renders clickable album link when album and artist metadata is present', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithMetadata } });
    const albumLink = container.querySelector('.track-album.link') as HTMLElement;
    expect(albumLink).toBeTruthy();
    expect(albumLink.textContent).toContain('Discovery');
  });

  it('navigates to artist page when artist link is clicked', async () => {
    mocks.navigate.mockImplementation(() => {});
    const { container } = render(NowPlayingInfo, { props: { track: trackWithMetadata } });
    const artistLink = container.querySelector('.track-artist.link') as HTMLElement;
    await fireEvent.click(artistLink);
    expect(mocks.navigate).toHaveBeenCalledWith('/artist/artist%3Adaft-punk');
  });

  it('navigates to album page when album link is clicked', async () => {
    mocks.navigate.mockImplementation(() => {});
    const { container } = render(NowPlayingInfo, { props: { track: trackWithMetadata } });
    const albumLink = container.querySelector('.track-album.link') as HTMLElement;
    await fireEvent.click(albumLink);
    expect(mocks.navigate).toHaveBeenCalledWith('/album/album%3Adiscovery%3Adaft-punk');
  });

  it('does not render album link when album is missing', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithoutAlbum } });
    expect(container.querySelector('.track-album.link')).toBeFalsy();
    expect(container.textContent).not.toContain('Discovery');
  });

  it('does not render artist link when artist is missing', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithoutArtist } });
    expect(container.querySelector('.track-artist.link')).toBeFalsy();
  });
});
