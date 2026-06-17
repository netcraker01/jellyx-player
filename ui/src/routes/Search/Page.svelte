<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '@i18n';
  import SearchBar from '@features/search/components/SearchBar.svelte';
  import ResultsList from '@features/search/components/ResultsList.svelte';
  import {
    searchResults,
    searchQuery,
    isSearching,
    searchError,
  } from '@features/search/stores/search';

  let hasSearched = false;

  function handleSearch(e: CustomEvent<{ query: string }>) {
    hasSearched = true;
    searchQuery.set(e.detail.query);
    searchResults.search(e.detail.query);
  }

  let results: Track[] = [];
  searchResults.subscribe((v) => { results = v; });

  import type { Track } from '@shared/types/models';
</script>

<div class="page-search">
  <h1>{$t('routes.search')}</h1>
  <div class="search-container">
    <SearchBar on:search={handleSearch} disabled={$isSearching} />
  </div>
  <div class="results-container">
    <ResultsList
      tracks={results}
      loading={$isSearching}
      error={$searchError}
      {hasSearched}
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