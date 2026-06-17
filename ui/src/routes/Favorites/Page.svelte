<script lang="ts">
  import { onMount } from 'svelte';
  import { favorites } from '@features/favorites/stores/favorites';
  import type { FavoriteEntry } from '@shared/types/models';

  onMount(() => {
    favorites.load();
  });

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  async function handleRemove(trackId: string) {
    await favorites.remove(trackId);
  }
</script>

<div class="page-favorites">
  <h1>Favorites</h1>

  {#if $favorites.length === 0}
    <p class="empty-message">No favorites yet. Add tracks from search results!</p>
  {:else}
    <ul class="favorites-list">
      {#each $favorites as entry (entry.track.id)}
        <li class="favorite-item">
          <div class="track-info">
            <span class="track-title">{entry.track.title}</span>
            <span class="track-artist">{entry.track.artist}</span>
          </div>
          <div class="track-meta">
            <span class="track-duration">{formatDuration(entry.track.duration)}</span>
            <span class="track-source">{entry.track.source}</span>
          </div>
          <button
            class="remove-btn"
            on:click={() => handleRemove(entry.track.id)}
            title="Remove from favorites"
          >
            ✕
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .page-favorites {
    padding: 1rem;
  }

  h1 {
    color: var(--text-primary, #e0e0e0);
    font-size: 1.5rem;
    margin-bottom: 1rem;
  }

  .empty-message {
    color: var(--text-secondary, #999);
    font-size: 0.9rem;
  }

  .favorites-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .favorite-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #333);
  }

  .favorite-item:hover {
    background: var(--surface-hover, #1a1a1a);
  }

  .track-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.95rem;
  }

  .track-artist {
    color: var(--text-secondary, #999);
    font-size: 0.8rem;
  }

  .track-meta {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    margin-right: 0.5rem;
  }

  .track-duration,
  .track-source {
    color: var(--text-secondary, #999);
    font-size: 0.8rem;
  }

  .remove-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #999);
    cursor: pointer;
    font-size: 1rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
  }

  .remove-btn:hover {
    color: var(--danger, #ff4444);
    background: var(--surface-hover, #1a1a1a);
  }
</style>