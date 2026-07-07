<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '@i18n';
  import SearchBar from '@features/search/components/SearchBar.svelte';
  import SearchResults from '@features/search/components/SearchResults.svelte';
  import { searchQuery } from '@features/search/stores/search';
  import { groupedSearchResults, isSearchingGrouped, groupedSearchError, searchGrouped, loadMoreResults, isLoadingMoreResults } from '@features/search/stores/searchGrouped';
  import {
    suggestionCategories,
    isLoadingCategories,
    loadSuggestionCategories,
  } from '@features/search/stores/suggestions';

  type SearchFilter = 'all' | 'videos' | 'artists' | 'local';

  // Derive "has searched" from the persistent store — survives navigation
  $: hasSearched = $searchQuery.trim().length > 0;
  let currentFilter: SearchFilter = 'all';

  onMount(() => {
    loadSuggestionCategories();
  });

  function handleSearch(e: CustomEvent<{ query: string }>) {
    searchQuery.set(e.detail.query);
    searchGrouped(e.detail.query, mapFilter(currentFilter));
  }

  function handleFilter(filter: SearchFilter) {
    currentFilter = filter;
    if ($searchQuery) {
      searchGrouped($searchQuery, mapFilter(filter));
    }
  }

  function handleSuggestionClick(query: string) {
    searchQuery.set(query);
    searchGrouped(query, mapFilter(currentFilter));
  }

  /** Map the UI filter to the store's SearchFilter type.
   *  `all` → undefined (no filter), `videos` → 'songs', `artists` → 'artists',
   *  `local` → 'local' (frontend-only filter). */
  function mapFilter(filter: SearchFilter): 'songs' | 'artists' | 'albums' | 'local' | undefined {
    if (filter === 'all') return undefined;
    if (filter === 'videos') return 'songs';
    if (filter === 'artists') return 'artists';
    return 'local';
  }
</script>

<div class="page-search">
  <h1>{$t('routes.search')}</h1>
  <div class="search-container">
    <SearchBar on:search={handleSearch} disabled={$isSearchingGrouped} />
  </div>

  {#if !hasSearched && $suggestionCategories.length > 0}
    <div class="suggestions-section">
      <p class="suggestions-label">{$t('search.suggestions') ?? 'Suggestions'}</p>
      <div class="suggestions-chips">
        {#each $suggestionCategories as cat}
          <button
            class="suggestion-chip"
            style="--chip-color: {cat.color}"
            on:click={() => handleSuggestionClick(cat.query)}
          >
            {cat.label}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="results-container">
    {#if hasSearched}
      <SearchResults
        result={$groupedSearchResults}
        filter={currentFilter}
        loading={$isSearchingGrouped}
        error={$groupedSearchError}
        onFilter={handleFilter}
        hasMoreSongs={$groupedSearchResults?.hasMoreSongs ?? false}
        loadingMore={$isLoadingMoreResults}
        onLoadMore={loadMoreResults}
      />
    {/if}
  </div>
</div>

<style>
  .page-search {
    padding: 1rem;
  }

  h1 {
    color: var(--text-primary, #e0e0e0);
    font-size: 1.5rem;
    margin-bottom: 1.5rem;
  }

  .search-container {
    max-width: 600px;
    margin-bottom: 1.5rem;
  }

  .suggestions-section {
    margin-bottom: 1.5rem;
  }

  .suggestions-label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.875rem;
    font-weight: 500;
    margin: 0 0 0.75rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .suggestions-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .suggestion-chip {
    padding: 0.4rem 0.9rem;
    border: 1px solid color-mix(in srgb, var(--chip-color) 40%, var(--bg-elevated, #1a1a2e));
    border-radius: 20px;
    background: color-mix(in srgb, var(--chip-color) 10%, var(--bg-elevated, #1a1a2e));
    color: var(--chip-color);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.2s, transform 0.1s;
  }

  .suggestion-chip:hover {
    background: color-mix(in srgb, var(--chip-color) 20%, var(--bg-elevated, #1a1a2e));
    box-shadow: 0 2px 12px color-mix(in srgb, var(--chip-color) 25%, transparent);
    transform: translateY(-1px);
  }

  .suggestion-chip:active {
    transform: translateY(0);
  }

  .results-container {
    margin-top: 0.5rem;
  }
</style>