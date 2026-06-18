<script lang="ts">
  import { navigate } from 'svelte-routing';
  import { albumArtUrl } from '@shared/utils/assetUrl';

  export let id: string;
  export let name: string = 'Unknown Artist';
  export let thumbnail: string | undefined = undefined;
  export let trackCount: number = 0;

  function handleClick() {
    navigate(`/artist/${encodeURIComponent(id)}`);
  }
</script>

<button class="artist-card" on:click={handleClick} aria-label="View {name}">
  {#if albumArtUrl(thumbnail)}
    <img class="artist-art" src={albumArtUrl(thumbnail)} alt={name} />
  {:else}
    <div class="artist-art-placeholder">
      <span class="placeholder-text">♪</span>
    </div>
  {/if}
  <div class="artist-info">
    <span class="artist-name">{name}</span>
    <span class="artist-meta">{trackCount} track{trackCount === 1 ? '' : 's'}</span>
  </div>
</button>

<style>
  .artist-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 8px;
    background: var(--bg-elevated, #1f2937);
    border: none;
    text-align: left;
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.15s;
  }

  .artist-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  .artist-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 6px;
    object-fit: cover;
  }

  .artist-art-placeholder {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 6px;
    background: var(--bg-primary, #111827);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .placeholder-text {
    font-size: 2rem;
    color: var(--text-secondary, #9ca3af);
  }

  .artist-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow: hidden;
  }

  .artist-name {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .artist-meta {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
