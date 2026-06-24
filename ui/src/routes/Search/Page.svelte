<script lang="ts">
  import { t } from '@i18n';
  import SearchBar from '@features/search/components/SearchBar.svelte';
  import SearchResults from '@features/search/components/SearchResults.svelte';
  import { searchQuery } from '@features/search/stores/search';
  import { groupedSearchResults, isSearchingGrouped, groupedSearchError, searchGrouped } from '@features/search/stores/searchGrouped';

  type SearchFilter = 'all' | 'videos' | 'artists';

  // Derive "has searched" from the persistent store — survives navigation
  $: hasSearched = $searchQuery.trim().length > 0;
  let currentFilter: SearchFilter = 'all';

  function handleSearch(e: CustomEvent<{ query: string }>) {
    searchQuery.set(e.detail.query);
    searchGrouped(e.detail.query, currentFilter === 'all' ? undefined : currentFilter === 'videos' ? 'songs' : 'artists');
  }

  function handleFilter(filter: SearchFilter) {
    currentFilter = filter;
    if ($searchQuery) {
      searchGrouped($searchQuery, filter === 'all' ? undefined : filter === 'videos' ? 'songs' : 'artists');
    }
  }
</script>

<div class="page-search">
  <h1>{$t('routes.search')}</h1>
  <div class="search-container">
    <SearchBar on:search={handleSearch} disabled={$isSearchingGrouped} />
  </div>
  <div class="results-container">
    {#if hasSearched}
      <SearchResults
        result={$groupedSearchResults}
        filter={currentFilter}
        loading={$isSearchingGrouped}
        error={$groupedSearchError}
        onFilter={handleFilter}
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

  .results-container {
    margin-top: 0.5rem;
  }
</style>