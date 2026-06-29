/**
 * Home page tests for the simplified landing view.
 *
 * Verifies the page renders a welcome message and a search button.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { translations } from '@i18n';
import HomePage from './Page.svelte';

vi.mock('@app/router/navigation', () => ({
  navigate: vi.fn(),
}));

const { readable } = await vi.hoisted(() => import('svelte/store'));

vi.mock('@features/search/stores/suggestions', () => ({
  suggestionCategories: readable([]),
  isLoadingCategories: readable(false),
  loadSuggestionCategories: vi.fn().mockResolvedValue(undefined),
  reloadSuggestionCategories: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@features/search/stores/search', () => ({
  searchQuery: readable(''),
}));

vi.mock('@features/search/stores/searchGrouped', () => ({
  searchGrouped: vi.fn().mockResolvedValue(undefined),
  groupedSearchResults: readable(null),
  isSearchingGrouped: readable(false),
  groupedSearchError: readable(null),
}));

import { navigate } from '@app/router/navigation';
const mockedNavigate = vi.mocked(navigate);

describe('Home page (simplified)', () => {
  beforeEach(() => {
    mockedNavigate.mockClear();
    translations.set({
      home: { welcome: 'Welcome', tagline: 'Your sound, your space.' },
      common: { search: 'Search' },
    });
  });

  it('renders the welcome heading', () => {
    const { container } = render(HomePage);
    expect(container.textContent).toContain('Welcome');
  });

  it('renders a search button that navigates to /search', async () => {
    const { container } = render(HomePage);
    const btn = container.querySelector('button');
    expect(btn).toBeTruthy();
    expect(btn!.textContent).toContain('Search');

    await fireEvent.click(btn!);
    expect(mockedNavigate).toHaveBeenCalledWith('/search');
  });
});