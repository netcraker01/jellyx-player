/**
 * SearchResults component tests.
 *
 * Verifies the component renders videos and artists in separate
 * sections, handles filter tabs, and shows empty states.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';

const { readable } = await vi.hoisted(() => import('svelte/store'));

const mocks = vi.hoisted(() => ({
  playPlaylist: vi.fn(),
  playStream: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  playPlaylist: mocks.playPlaylist,
  playStream: mocks.playStream,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: vi.fn().mockResolvedValue(undefined),
  addToQueueAction: vi.fn().mockResolvedValue(undefined),
  playNextAction: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@features/favorites/stores/favorites', () => ({
  favorites: {
    add: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('@i18n', () => {
  const translateFn = (key: string) => {
    const map: Record<string, string> = {
      'search.local': 'Local',
      'search.videos': 'Videos',
      'search.artists': 'Artists',
    };
    return map[key] ?? key;
  };
  return { t: readable(translateFn, () => {}) };
});

vi.mock('@app/router/navigation', () => ({
  navigate: vi.fn(),
}));

import SearchResults from '@features/search/components/SearchResults.svelte';
import { Source } from '@shared/types/models';
import type { Track, GroupedSearchResult } from '@shared/types/models';

const sampleTrack: Track = {
  id: 'track:1',
  source: Source.YouTube,
  sourceId: 'yt-1',
  title: 'One More Time',
  artist: 'Daft Punk',
  duration: 320,
  metadata: {},
};

const sampleResult: GroupedSearchResult = {
  songs: [sampleTrack],
  artists: [
    { id: 'artist--daft-punk', name: 'Daft Punk', thumbnail: undefined, trackCount: 1 },
  ],
  albums: [],
};

const emptyResult: GroupedSearchResult = {
  songs: [],
  artists: [],
  albums: [],
};

function renderSearchResults(props: Partial<ComponentProps<SearchResults>> = {}) {
  return render(SearchResults, {
    props: {
      result: sampleResult,
      filter: 'all',
      loading: false,
      error: null,
      ...props,
    },
  });
}

describe('SearchResults', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders videos and artists sections when result is provided', () => {
    const { container } = renderSearchResults();

    expect(container.querySelector('.section-videos')).toBeTruthy();
    expect(container.querySelector('.section-artists')).toBeTruthy();
  });

  it('shows a track row for each song', () => {
    const { container } = renderSearchResults();

    const trackRows = container.querySelectorAll('.track-row');
    expect(trackRows.length).toBe(1);
    expect(container.textContent).toContain('One More Time');
  });

  it('shows artist cards', () => {
    const { container } = renderSearchResults();

    expect(container.textContent).toContain('Daft Punk');
    expect(container.querySelector('.artist-card')).toBeTruthy();
  });

  it('shows a global empty state when all sections are empty in All filter', () => {
    const { container } = renderSearchResults({
      result: emptyResult,
    });

    expect(container.querySelector('.empty-state')).toBeTruthy();
    expect(container.textContent).toContain('No results found.');
  });

  it('renders filter tabs with All, Tracks, Artists, and Local', () => {
    const { container } = renderSearchResults();

    const tabs = container.querySelectorAll('.filter-tab');
    expect(tabs.length).toBe(4);
    expect(container.textContent).toContain('Tracks');
    expect(container.textContent).toContain('Artists');
    expect(container.textContent).toContain('Local');
  });

  it('shows per-section empty state for videos filter with no songs', async () => {
    const { container } = renderSearchResults({
      result: emptyResult,
      filter: 'videos',
    });

    // Svelte reactive statements need a tick to settle
    await waitFor(() => {
      expect(container.querySelector('.section-videos')).toBeTruthy();
    });
    expect(container.textContent).toContain('No tracks found.');
  });

  it('shows per-section empty state for artists filter with no artists', async () => {
    const { container } = renderSearchResults({
      result: emptyResult,
      filter: 'artists',
    });

    await waitFor(() => {
      expect(container.querySelector('.section-artists')).toBeTruthy();
    });
    expect(container.textContent).toContain('No artists found.');
  });

  it('shows loading state', () => {
    const { container } = renderSearchResults({ loading: true, result: emptyResult });

    expect(container.querySelector('.section-videos .empty-section')?.textContent).toContain('Searching...');
  });

  it('shows error state', () => {
    const { container } = renderSearchResults({ error: 'Network error' });

    expect(container.textContent).toContain('Network error');
  });
});