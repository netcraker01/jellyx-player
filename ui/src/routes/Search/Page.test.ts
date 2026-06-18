/**
 * Search page tests for grouped search integration.
 *
 * Verifies the page wires the grouped search store and filter tabs.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';

const mocks = vi.hoisted(() => ({
  searchGroupedCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  searchGrouped: mocks.searchGroupedCmd,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

vi.mock('svelte-routing', () => ({
  navigate: vi.fn(),
}));

import { translations } from '@i18n';
import SearchPage from './Page.svelte';

describe('Search page', () => {
  beforeEach(() => {
    mocks.searchGroupedCmd.mockReset();
    translations.set({
      routes: { search: 'Search' },
      common: { search: 'Search' },
      search: { placeholder: 'Search...' },
    });
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
      songs: [],
      artists: [{ id: 'artist:daft-punk', name: 'Daft Punk', trackCount: 5 }],
      albums: [],
    });

    const { container } = render(SearchPage);
    const input = container.querySelector('input[type="text"]') as HTMLInputElement;
    expect(input).toBeTruthy();

    await fireEvent.input(input, { target: { value: 'daft' } });
    const form = container.querySelector('form');
    await fireEvent.submit(form!);

    await waitFor(() => {
      expect(mocks.searchGroupedCmd).toHaveBeenCalledWith('daft', undefined);
    });

    expect(container.textContent).toContain('Daft Punk');
  });

  it('changes filter tab and searches with the selected filter', async () => {
    mocks.searchGroupedCmd.mockResolvedValue({
      songs: [],
      artists: [],
      albums: [],
    });

    const { container } = render(SearchPage);
    const input = container.querySelector('input[type="text"]') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'daft' } });
    const form = container.querySelector('form');
    await fireEvent.submit(form!);

    await waitFor(() => expect(mocks.searchGroupedCmd).toHaveBeenCalledTimes(1));

    const tabs = container.querySelectorAll('.filter-tab');
    // tabs: All, Songs, Artists, Albums
    expect(tabs.length).toBe(4);

    await fireEvent.click(tabs[2]);

    await waitFor(() => {
      expect(mocks.searchGroupedCmd).toHaveBeenLastCalledWith('daft', 'artists');
    });
  });
});
