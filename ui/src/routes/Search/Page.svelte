<script lang="ts">
  import { t } from '@i18n';
  import SearchBar from '@features/search/components/SearchBar.svelte';
  import GroupedResults from '@features/search/components/GroupedResults.svelte';
  import {
    searchGrouped,
    clearSearchGrouped,
    groupedSearchResults,
    isSearchingGrouped,
    groupedSearchError,
  } from '@features/search/stores/searchGrouped';
  import type { SearchFilter } from '@shared/types/models';

  let hasSearched = false;
  let currentQuery = '';
  let currentFilter: SearchFilter | 'all' = 'all';

  function handleSearch(e: CustomEvent<{ query: string }>) {
    hasSearched = true;
    currentQuery = e.detail.query;
    searchGrouped(currentQuery, currentFilter === 'all' ? undefined : currentFilter);
  }

  function handleFilter(filter: SearchFilter | 'all') {
    currentFilter = filter;
    if (currentQuery.trim()) {
      searchGrouped(currentQuery, filter === 'all' ? undefined : filter);
    }
  }

  let result = null;
  groupedSearchResults.subscribe((v) => { result = v; });
</script>

<div class="page-search">
  <h1>{$t('routes.search')}</h1>
  <div class="search-container">
    <SearchBar on:search={handleSearch} disabled={$isSearchingGrouped} />
  </div>
  <div class="results-container">
    <GroupedResults
      {result}
      filter={currentFilter}
      loading={$isSearchingGrouped}
      error={$groupedSearchError}
      onFilter={handleFilter}
    />
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