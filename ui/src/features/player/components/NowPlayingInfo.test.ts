/**
 * NowPlayingInfo component tests.
 *
 * Verifies artist/album navigation links are rendered and call navigate,
 * thumbnail rendering, and list picker button.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  addTrackToPlaylist: vi.fn(),
  getRecentPlaylists: vi.fn(),
  getAllPlaylists: vi.fn(),
  searchUserPlaylists: vi.fn(),
  createPlaylist: vi.fn(),
  getPlaylistTracks: vi.fn(),
}));

vi.mock('@app/router/navigation', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@features/playlists/stores/playlists', () => ({
  playlists: {
    subscribe: (fn: (v: any[]) => void) => { fn([]); return () => {}; },
    load: vi.fn(),
    addTrack: mocks.addTrackToPlaylist,
    create: mocks.createPlaylist,
  },
  recentPlaylists: {
    subscribe: (fn: (v: any[]) => void) => { fn([]); return () => {}; },
  },
}));

vi.mock('@services/commands', () => ({
  getRecentPlaylists: mocks.getRecentPlaylists,
  getAllPlaylists: mocks.getAllPlaylists,
  searchUserPlaylists: mocks.searchUserPlaylists,
  getPlaylistTracks: mocks.getPlaylistTracks,
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

const remoteYouTubeTrack = {
  id: 'track:yt:1',
  source: Source.YouTube,
  sourceId: 'yt-abc123',
  title: 'Remote Song',
  artist: 'Remote Artist',
  duration: 240,
  thumbnail: 'https://img.youtube.com/vi/yt-abc123/0.jpg',
  metadata: { description: 'Live set recorded in the California high desert.' },
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

  it('renders thumbnail for local track with artwork', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithMetadata } });
    const img = container.querySelector('.album-art') as HTMLImageElement;
    expect(img).toBeTruthy();
    expect(img.src.endsWith('/art/one.jpg')).toBe(true);
  });

  it('does not render artist or album links for remote YouTube track', () => {
    const { container } = render(NowPlayingInfo, { props: { track: remoteYouTubeTrack } });
    expect(container.querySelector('.track-artist.link')).toBeFalsy();
    expect(container.querySelector('.track-album.link')).toBeFalsy();
    expect(container.textContent).toContain('Remote Artist');
    expect(container.textContent).not.toContain('Discovery');
  });

  it('renders thumbnail for remote YouTube track', () => {
    const { container } = render(NowPlayingInfo, { props: { track: remoteYouTubeTrack } });
    const img = container.querySelector('.album-art') as HTMLImageElement;
    expect(img).toBeTruthy();
    expect(img.src).toBe('https://img.youtube.com/vi/yt-abc123/0.jpg');
  });

  it('renders list picker button', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithMetadata } });
    expect(container.querySelector('.list-btn')).toBeTruthy();
  });

  it('renders placeholder when thumbnail is missing', () => {
    const { container } = render(NowPlayingInfo, { props: { track: trackWithoutAlbum } });
    expect(container.querySelector('.album-art')).toBeFalsy();
    expect(container.querySelector('.album-art-placeholder')).toBeTruthy();
  });
});