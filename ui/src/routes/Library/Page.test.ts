/**
 * Library page tests.
 *
 * Verifies the Library page mounts safely and renders folder cards (not a
 * flat track table) as the primary browsing surface.
 *
 * Spec: library-folder-view — Watched folders render as clickable cards.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from '@testing-library/svelte';

const { readable } = await vi.hoisted(() => import('svelte/store'));

const mocks = vi.hoisted(() => ({
  loadWatchedFolders: vi.fn(),
  loadLocalTracks: vi.fn(),
  scanNewFolder: vi.fn(),
  removeFolder: vi.fn(),
  navigate: vi.fn(),
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

  // Minimal derived store mock: a Map of folder → entries.
  const createDerived = (get: () => Map<string, unknown[]>) => {
    let value = get();
    const subs = new Set<(v: Map<string, unknown[]>) => void>();
    return {
      subscribe(fn: (v: Map<string, unknown[]>) => void) {
        fn(value);
        subs.add(fn);
        return () => subs.delete(fn);
      },
      get,
      refresh() {
        value = get();
        subs.forEach((fn) => fn(value));
      },
    };
  };

  const localTracksWritable = createWritable<unknown[]>([]);
  return {
    watchedFolders: createWritable<unknown[]>([]),
    localTracks: localTracksWritable,
    tracksByFolder: createDerived(() => {
      const map = new Map<string, unknown[]>();
      for (const entry of localTracksWritable.subscribe.length ? [] : []) {
        // no-op
      }
      return map;
    }),
    isScanning: createWritable(false),
    scanStatus: createWritable<unknown>(null),
    scanError: createWritable<string | null>(null),
  };
});

vi.mock('@features/library/stores/library', () => ({
  watchedFolders: storeMock.watchedFolders,
  localTracks: storeMock.localTracks,
  tracksByFolder: storeMock.tracksByFolder,
  isScanning: storeMock.isScanning,
  scanStatus: storeMock.scanStatus,
  scanError: storeMock.scanError,
  loadWatchedFolders: mocks.loadWatchedFolders,
  loadLocalTracks: mocks.loadLocalTracks,
  scanNewFolder: mocks.scanNewFolder,
  removeFolder: mocks.removeFolder,
}));

vi.mock('@app/router/navigation', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@i18n', () => {
  const translateFn = (key: string) => {
    const map: Record<string, string> = {
      'library.local_files': 'Local File',
      'library.watched_folders': 'Watched Folders',
      'library.add_folder': 'Add Folder',
      'library.scanning': 'Scanning...',
      'library.folder_tracks': 'tracks',
      'library.open_folder': 'Open folder',
      'library.empty_folders': 'No folders added yet. Click "Add Folder" to scan your music.',
      'library.empty_tracks': 'No tracks found. Try scanning a folder with audio files.',
    };
    return map[key] ?? key;
  };
  return { t: readable(translateFn, () => {}) };
});

import LibraryPage from './Page.svelte';

describe('Library page', () => {
  beforeEach(() => {
    mocks.loadWatchedFolders.mockReset().mockResolvedValue(undefined);
    mocks.loadLocalTracks.mockReset().mockResolvedValue(undefined);
    mocks.navigate.mockReset();
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
    expect(container.textContent).toContain('Local File');
  });

  it('renders the empty state when no folders are configured', () => {
    const { container } = render(LibraryPage);
    expect(container.textContent).toContain('No folders added yet');
  });

  it('renders folder cards (not a flat track table) when folders are present', () => {
    storeMock.watchedFolders.set([
      { path: '/music/rock', addedAt: '2026-01-01' },
      { path: '/music/jazz', addedAt: '2026-01-02' },
    ]);
    storeMock.localTracks.set([
      { folderPath: '/music/rock', track: { title: 'Song One', artist: 'Artist A', duration: 180 } },
      { folderPath: '/music/rock', track: { title: 'Song Two', artist: 'Artist B', duration: 240 } },
    ]);
    storeMock.tracksByFolder.refresh();

    const { container } = render(LibraryPage);

    // Folder cards should be rendered.
    const cards = container.querySelectorAll('.folder-card');
    expect(cards.length).toBe(2);

    // No flat track table should be present.
    expect(container.querySelector('table.track-table')).toBeNull();

    // Folder names (final segment) should appear in the cards.
    expect(container.textContent).toContain('rock');
    expect(container.textContent).toContain('jazz');
    // Full path should be available as a tooltip on the folder-path element.
    const folderPath = container.querySelector('.folder-path[title="/music/rock"]');
    expect(folderPath).toBeTruthy();
  });

  it('navigates to folder detail when a folder card is clicked', async () => {
    const { fireEvent } = await import('@testing-library/svelte');
    storeMock.watchedFolders.set([{ path: '/music/rock', addedAt: '2026-01-01' }]);
    storeMock.localTracks.set([]);
    storeMock.tracksByFolder.refresh();

    const { container } = render(LibraryPage);
    const card = container.querySelector('.folder-card') as HTMLElement;
    expect(card).toBeTruthy();

    await fireEvent.click(card);
    expect(mocks.navigate).toHaveBeenCalledWith('/library/folder/' + encodeURIComponent('/music/rock'));
  });
});