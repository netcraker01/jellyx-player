<script lang="ts">
  import { Search } from 'lucide-svelte';
  import { searchQuery } from '@features/search/stores/search';
  import { navigate } from '../router/navigation';
  import { t } from '@i18n';

  let value = '';

  function handleSubmit() {
    const q = value.trim();
    if (!q) return;
    searchQuery.set(q);
    navigate('/search');
  }
</script>

<form class="global-search" on:submit|preventDefault={handleSubmit}>
  <Search size={18} />
  <input
    type="text"
    placeholder={$t('search.placeholder')}
    bind:value
  />
</form>

<style>
  .global-search {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 10px;
    border: 1px solid var(--border-color, #2d3748);
    margin-bottom: 1.25rem;
    transition: border-color 0.2s;
  }

  .global-search:focus-within {
    border-color: var(--color-accent, #6366f1);
  }

  .global-search :global(svg) {
    color: var(--text-secondary, #9ca3af);
    flex-shrink: 0;
  }

  input {
    flex: 1;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    font-family: inherit;
  }

  input::placeholder {
    color: var(--text-tertiary, #6b7280);
  }
</style>
