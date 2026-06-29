/**
 * Search page tests for grouped search integration.
 *
 * Verifies the page wires the grouped search store
 * and filter tabs for the Videos/Artists flow.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  searchGroupedCmd: vi.fn(),
  playStreamCmd: vi.fn(),
  playPlaylistCmd: vi.fn(),
}));

const { readable } = await vi.hoisted(() => import('svelte/store'));

vi.mock('@services/commands', () => ({
  search: vi.fn(),
  searchGrouped: mocks.searchGroupedCmd,
  searchPlaylists: vi.fn(),
  playStream: mocks.playStreamCmd,
  playPlaylist: mocks.playPlaylistCmd,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

vi.mock('@app/router/navigation', () => ({
  navigate: vi.fn(),
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: vi.fn().mockResolvedValue(undefined),
  addToQueueAction: vi.fn().mockResolvedValue(undefined),
  playNextAction: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@features/search/stores/suggestions', () => ({
  suggestionCategories: readable([]),
  isLoadingCategories: readable(false),
  loadSuggestionCategories: vi.fn().mockResolvedValue(undefined),
  reloadSuggestionCategories: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@i18n', () => {
  const translateFn = (key: string) => {
    const map: Record<string, string> = {
      'routes.search': 'Search',
      'common.search': 'Search',
      'search.placeholder': 'Search...',
      'search.loading': 'Searching...',
      'search.no_results': 'No results found.',
      'search.videos': 'Videos',
      'search.artists': 'Artists',
    };
    return map[key] ?? key;
  };
  const store = readable(translateFn, () => {});
  return { t: store };
});

import SearchPage from './Page.svelte';
import { Source } from '@shared/types/models';

describe('Search page', () => {
  beforeEach(() => {
    mocks.searchGroupedCmd.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the search page heading', () => {
    const { container } = render(SearchPage);
    expect(container.textContent).toContain('Search');
  });

  it('performs a grouped search when query is submitted', async () => {
    mocks.searchGroupedCmd.mockResolvedValueOnce({
      songs: [
        {
          id: 'track:yt-1',
          source: Source.YouTube,
          sourceId: 'yt-1',
          title: 'One More Time',
          artist: 'Daft Punk',
          duration: 320,
          metadata: {},
        },
      ],
      artists: [
        { id: 'artist--daft-punk', name: 'Daft Punk', thumbnail: undefined, trackCount: 1 },
      ],
      albums: [],
      hasMoreSongs: false,
    });

    const { container } = render(SearchPage);
    const input = container.querySelector('input[type="text"]') as HTMLInputElement;
    expect(input).toBeTruthy();

    await fireEvent.input(input, { target: { value: 'daft punk' } });
    const form = container.querySelector('form');
    await fireEvent.submit(form!);

    await waitFor(() => {
      expect(mocks.searchGroupedCmd).toHaveBeenCalledWith('daft punk', undefined, 0, 50);
    });

    expect(container.textContent).toContain('One More Time');
  });

  it('shows filter tabs with All, Videos, and Artists', async () => {
    mocks.searchGroupedCmd.mockResolvedValueOnce({
      songs: [],
      artists: [],
      albums: [],
      hasMoreSongs: false,
    });

    const { container } = render(SearchPage);
    const input = container.querySelector('input[type="text"]') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'test' } });
    await fireEvent.submit(container.querySelector('form')!);

    await waitFor(() => expect(mocks.searchGroupedCmd).toHaveBeenCalledTimes(1));

    const tabs = container.querySelectorAll('.filter-tab');
    expect(tabs.length).toBe(3);
    expect(container.textContent).toContain('Videos');
    expect(container.textContent).toContain('Artists');
  });
});