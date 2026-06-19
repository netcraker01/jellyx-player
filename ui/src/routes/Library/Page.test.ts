/**
 * Library page tests.
 *
 * Verifies the Library page mounts safely and renders its expected UI
 * states when the underlying store is populated.
 *
 * Spec: FR-013 — Library page is reachable and renders.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  loadWatchedFolders: vi.fn(),
  loadLocalTracks: vi.fn(),
  scanNewFolder: vi.fn(),
  removeFolder: vi.fn(),
}));

const storeMock = vi.hoisted(() => {
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
    };
  };

  return {
    watchedFolders: createWritable<unknown[]>([]),
    localTracks: createWritable<unknown[]>([]),
    isScanning: createWritable(false),
    scanStatus: createWritable<unknown>(null),
    scanError: createWritable<string | null>(null),
  };
});

vi.mock('@features/library/stores/library', () => ({
  watchedFolders: storeMock.watchedFolders,
  localTracks: storeMock.localTracks,
  isScanning: storeMock.isScanning,
  scanStatus: storeMock.scanStatus,
  scanError: storeMock.scanError,
  loadWatchedFolders: mocks.loadWatchedFolders,
  loadLocalTracks: mocks.loadLocalTracks,
  scanNewFolder: mocks.scanNewFolder,
  removeFolder: mocks.removeFolder,
}));

import LibraryPage from './Page.svelte';

describe('Library page', () => {
  beforeEach(() => {
    mocks.loadWatchedFolders.mockReset().mockResolvedValue(undefined);
    mocks.loadLocalTracks.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    storeMock.watchedFolders.set([]);
    storeMock.localTracks.set([]);
    storeMock.isScanning.set(false);
    storeMock.scanStatus.set(null);
    storeMock.scanError.set(null);
  });

  it('mounts safely and renders the library heading', () => {
    const { container } = render(LibraryPage);
    expect(container.textContent).toContain('Local Library');
  });

  it('renders the empty state when no folders are configured', () => {
    const { container } = render(LibraryPage);
    expect(container.textContent).toContain('No folders added yet');
  });

  it('renders local tracks when the store is populated', () => {
    storeMock.watchedFolders.set([{ path: '/music', trackCount: 2 }]);
    storeMock.localTracks.set([
      {
        folderPath: '/music',
        track: { title: 'Song One', artist: 'Artist A', duration: 180 },
      },
      {
        folderPath: '/music',
        track: { title: 'Song Two', artist: 'Artist B', duration: 240 },
      },
    ]);

    const { container } = render(LibraryPage);

    expect(container.textContent).toContain('All Local Tracks (2)');
    expect(container.textContent).toContain('Song One');
    expect(container.textContent).toContain('Song Two');
  });
});
