<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { initI18n, switchLocale, locale, t } from './i18n';

  let query = '';
  let results = [];
  let currentTrack = null;
  let ready = false;

  onMount(async () => {
    await initI18n();
    ready = true;
  });

  async function doSearch() {
    results = await invoke('search', { query });
  }

  async function play(url) {
    await invoke('play', { url });
  }

  // Available languages for the switcher
  const languages = [
    { code: 'en', label: 'English' },
    { code: 'es', label: 'Español' },
  ];
</script>

<main>
  <header>
    <h1>{$t('app.title')}</h1>
    <span class="tagline">{$t('app.tagline')}</span>
  </header>

  {#if ready}
    <!-- Language switcher -->
    <select
      class="locale-switcher"
      value={$locale}
      on:change={(e) => switchLocale(e.target.value)}
    >
      {#each languages as lang}
        <option value={lang.code}>{lang.label}</option>
      {/each}
    </select>

    <input
      bind:value={query}
      on:keydown={(e) => e.key === 'Enter' && doSearch()}
      placeholder={$t('app.search_placeholder')}
    />

    <div class="results">
      {#each results as track}
        <div class="track" on:click={() => play(track.stream_url)}>
          <img src={track.thumbnail} alt="" />
          <div class="info">
            <strong>{track.title}</strong>
            <span>{track.artist}</span>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="loading">{$t('common.loading')}</p>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #0a0a0f;
    color: #e0e0e0;
    font-family: 'Inter', sans-serif;
  }

  main {
    max-width: 900px;
    margin: 0 auto;
    padding: 2rem;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  h1 {
    color: #00ccff;
    font-size: 2rem;
    margin: 0;
  }

  .tagline {
    color: #666;
    font-size: 0.9rem;
  }

  .locale-switcher {
    position: fixed;
    top: 1rem;
    right: 1rem;
    background: #1a1a2e;
    border: 1px solid #333;
    border-radius: 6px;
    color: #e0e0e0;
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  input {
    width: 100%;
    padding: 0.75rem;
    background: #1a1a2e;
    border: 1px solid #333;
    border-radius: 8px;
    color: #fff;
    font-size: 1rem;
    box-sizing: border-box;
  }

  .loading {
    text-align: center;
    color: #666;
    margin-top: 3rem;
  }

  .track {
    display: flex;
    gap: 1rem;
    padding: 0.75rem;
    cursor: pointer;
    border-radius: 8px;
    transition: background 0.2s;
  }

  .track:hover {
    background: #1a1a2e;
  }

  .track img {
    width: 48px;
    height: 48px;
    border-radius: 4px;
    object-fit: cover;
  }

  .results {
    margin-top: 1rem;
  }
</style>
