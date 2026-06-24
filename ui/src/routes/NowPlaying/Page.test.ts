/**
 * NowPlaying page tests.
 *
 * Verifies the blurred artwork background appears when a track with artwork
 * is playing, and is absent when there is no artwork or no track.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import { writable } from 'svelte/store';
import NowPlayingPage from './Page.svelte';

// Track store mock — we need a writable so we can change it per test
let mockCurrentTrack = writable<any>(null);

vi.mock('@features/player/stores/player', () => ({
  get currentTrack() { return mockCurrentTrack; },
  isPlaying: { subscribe: (fn: any) => { fn(false); return () => {}; } },
  isCurrentTrackFavorited: { subscribe: (fn: any) => { fn(false); return () => {}; } },
  progress: { subscribe: (fn: any) => { fn({ position: 0, duration: 0 }); return () => {}; } },
  shuffle: { subscribe: (fn: any) => { fn(false); return () => {}; } },
  repeatMode: { subscribe: (fn: any) => { fn('Off'); return () => {}; } },
  queue: { subscribe: (fn: any) => { fn([]); return () => {}; } },
  currentIndex: { subscribe: (fn: any) => { fn(null); return () => {}; } },
  frequencyData: { subscribe: (fn: any) => { fn(null); return () => {}; }, set: vi.fn() },
  modoCineActive: { subscribe: (fn: any) => { fn(false); return () => {}; }, set: vi.fn() },
  seekTo: vi.fn(),
  playTrack: vi.fn(),
  removeTrack: vi.fn(),
  clearQueue: vi.fn(),
  togglePlayPause: vi.fn(),
  nextTrack: vi.fn(),
  previousTrack: vi.fn(),
  toggleShuffle: vi.fn(),
  cycleRepeat: vi.fn(),
}));

vi.mock('@services/events', () => ({
  createFftChannel: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@shared/utils/assetUrl', () => ({
  albumArtUrl: (path: string | undefined) => {
    if (!path) return undefined;
    if (path.startsWith('http')) return path;
    return `asset://${path}`;
  },
}));

vi.mock('@i18n', () => ({
  t: { subscribe: (fn: (v: any) => void) => { fn((key: string) => ({ 'now_playing.no_track': 'No track', 'common.no_data': 'No description available' }[key] ?? key)); return () => {}; } },
}));

describe('NowPlaying Page', () => {
  beforeEach(() => {
    mockCurrentTrack = writable<any>(null);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('renders empty state when no track is playing', () => {
    mockCurrentTrack.set(null);
    const { container } = render(NowPlayingPage);
    expect(container.querySelector('.empty-state')).toBeTruthy();
    expect(container.querySelector('.artwork-background')).toBeFalsy();
  });

  it('renders artwork background when track has thumbnail', () => {
    mockCurrentTrack.set({
      id: 'track:1',
      title: 'Song',
      artist: 'Artist',
      thumbnail: '/art/cover.jpg',
    });
    const { container } = render(NowPlayingPage);
    const bg = container.querySelector('.artwork-background') as HTMLElement;
    expect(bg).toBeTruthy();
    expect(bg.style.backgroundImage).toContain('asset:///art/cover.jpg');
  });

  it('does not render artwork background when thumbnail is missing', () => {
    mockCurrentTrack.set({
      id: 'track:2',
      title: 'Song',
      artist: 'Artist',
    });
    const { container } = render(NowPlayingPage);
    expect(container.querySelector('.artwork-background')).toBeFalsy();
    expect(container.querySelector('.now-playing-layout')).toBeTruthy();
  });

  it('does not render artwork background for remote thumbnail without https', () => {
    mockCurrentTrack.set({
      id: 'track:3',
      title: 'Song',
      artist: 'Artist',
      thumbnail: 'http://example.com/cover.jpg',
    });
    const { container } = render(NowPlayingPage);
    const bg = container.querySelector('.artwork-background') as HTMLElement;
    expect(bg).toBeTruthy();
    expect(bg.style.backgroundImage).toContain('http://example.com/cover.jpg');
  });

  it('renders layout and controls when track is present', () => {
    mockCurrentTrack.set({
      id: 'track:4',
      title: 'Song',
      artist: 'Artist',
      thumbnail: 'https://img.youtube.com/vi/abc/0.jpg',
    });
    const { container } = render(NowPlayingPage);
    expect(container.querySelector('.now-playing-layout')).toBeTruthy();
    expect(container.querySelector('.main-section')).toBeTruthy();
    const bg = container.querySelector('.artwork-background') as HTMLElement;
    expect(bg).toBeTruthy();
    expect(bg.style.backgroundImage).toContain('https://img.youtube.com/vi/abc/0.jpg');
  });

  it('renders the current track description below the controls', () => {
    mockCurrentTrack.set({
      id: 'track:5',
      title: 'Song',
      artist: 'Artist',
      thumbnail: 'https://img.youtube.com/vi/abc/0.jpg',
      metadata: { description: 'Live set recorded in the California high desert.' },
    });
    const { container } = render(NowPlayingPage);
    const description = container.querySelector('.description-panel');
    expect(description).toBeTruthy();
    expect(description?.textContent).toContain('Live set recorded in the California high desert.');
  });
});
