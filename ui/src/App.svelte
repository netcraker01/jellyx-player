<script>
  import { invoke } from '@tauri-apps/api/core';

  let query = '';
  let results = [];
  let currentTrack = null;

  async function doSearch() {
    results = await invoke('search', { query });
  }

  async function play(url) {
    await invoke('play', { url });
  }
</script>

<main>
  <h1>Helix</h1>

  <input
    bind:value={query}
    on:keydown={(e) => e.key === 'Enter' && doSearch()}
    placeholder="Search YouTube..."
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

  h1 {
    color: #00ccff;
    font-size: 2rem;
    margin-bottom: 1.5rem;
  }

  input {
    width: 100%;
    padding: 0.75rem;
    background: #1a1a2e;
    border: 1px solid #333;
    border-radius: 8px;
    color: #fff;
    font-size: 1rem;
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
</style>
