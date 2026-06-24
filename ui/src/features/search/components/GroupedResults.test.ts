/**
 * GroupedResults component tests.
 *
 * Verifies the component renders songs, artists, and albums in separate
 * sections, handles filter tabs, and shows empty states.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

vi.mock('@app/router/navigation', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: vi.fn().mockResolvedValue(undefined),
  addToQueueAction: vi.fn().mockResolvedValue(undefined),
  playNextAction: vi.fn().mockResolvedValue(undefined),
}));

import GroupedResults from '@features/search/components/GroupedResults.svelte';
import { Source } from '@shared/types/models';

const baseResult = {
  songs: [
    {
      id: 'track:1',
      source: Source.YouTube,
      sourceId: 'yt-1',
      title: 'One More Time',
      artist: 'Daft Punk',
      duration: 320,
      streamUrl: 'https://stream.test/track.mp3',
      metadata: {},
    },
  ],
  artists: [{ id: 'artist:daft-punk', name: 'Daft Punk', trackCount: 12 }],
  albums: [{ id: 'album:discovery:daft-punk', title: 'Discovery', artist: 'Daft Punk', trackCount: 14, year: 2001 }],
};

function renderGroupedResults(props: Partial<ComponentProps<GroupedResults>> = {}) {
  return render(GroupedResults, {
    props: {
      result: baseResult,
      filter: 'all',
      loading: false,
      error: null,
      ...props,
    },
  });
}

describe('GroupedResults', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders all three sections when result is provided', () => {
    const { container } = renderGroupedResults();

    expect(container.querySelector('.section-songs')).toBeTruthy();
    expect(container.querySelector('.section-artists')).toBeTruthy();
    expect(container.querySelector('.section-albums')).toBeTruthy();
  });

  it('shows a track row for each song', () => {
    const { container } = renderGroupedResults();

    const trackRows = container.querySelectorAll('.track-row');
    expect(trackRows.length).toBe(1);
    expect(container.textContent).toContain('One More Time');
  });

  it('shows artist cards', () => {
    const { container } = renderGroupedResults();

    expect(container.textContent).toContain('Daft Punk');
    expect(container.querySelector('.artist-card')).toBeTruthy();
  });

  it('shows album cards', () => {
    const { container } = renderGroupedResults();

    expect(container.textContent).toContain('Discovery');
    expect(container.querySelector('.album-card')).toBeTruthy();
  });

  it('shows a global empty state when all sections are empty in All filter', () => {
    const { container } = renderGroupedResults({
      result: { songs: [], artists: [], albums: [] },
    });

    expect(container.querySelector('.empty-state')).toBeTruthy();
    expect(container.textContent).toContain('No results found.');
  });

  it('shows per-section empty state when a specific filter has no matches', () => {
    const { container } = renderGroupedResults({
      result: { songs: [], artists: [], albums: [] },
      filter: 'songs',
    });

    expect(container.querySelector('.section-songs .empty-section')).toBeTruthy();
    expect(container.textContent).toContain('No songs found.');
  });

  it('renders filter tabs', () => {
    const { container } = renderGroupedResults();

    const tabs = container.querySelectorAll('.filter-tab');
    expect(tabs.length).toBe(4);
  });
});
