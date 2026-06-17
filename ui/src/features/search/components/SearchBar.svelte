<script lang="ts">
  import { Search } from 'lucide-svelte';
  import { t } from '@i18n';

  export let value = '';
  export let disabled = false;

  function handleSubmit() {
    if (value.trim()) {
      dispatch('search', { query: value.trim() });
    }
  }

  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher<{ search: { query: string } }>();
</script>

<form class="search-bar" on:submit|preventDefault={handleSubmit}>
  <Search size={18} class="search-icon" />
  <input
    type="text"
    class="search-input"
    bind:value
    placeholder={$t('search.placeholder')}
    disabled={disabled}
    aria-label="Search"
  />
  <button type="submit" class="search-btn" disabled={disabled || !value.trim()}>
    {$t('common.search')}
  </button>
</form>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 24px;
    padding: 0.5rem 1rem;
    transition: border-color 0.2s;
  }

  .search-bar:focus-within {
    border-color: var(--color-accent, #6366f1);
  }

  :global(.search-icon) {
    color: var(--text-secondary, #9ca3af);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    font-family: inherit;
  }

  .search-input::placeholder {
    color: var(--text-secondary, #9ca3af);
  }

  .search-btn {
    padding: 0.35rem 1rem;
    border: none;
    border-radius: 16px;
    background: var(--color-accent, #6366f1);
    color: white;
    font-size: 0.85rem;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .search-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>