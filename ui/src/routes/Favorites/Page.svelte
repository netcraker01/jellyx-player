<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Heart, X } from 'lucide-svelte';
  import { favorites } from '@features/favorites/stores/favorites';
  import { playTrack } from '@shared/utils/actions';
  import { t } from '@i18n';
  import type { FavoriteEntry, Track } from '@shared/types/models';

  onMount(() => {
    favorites.load();
  });

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  async function handlePlay(track: Track) {
    await playTrack(track);
  }

  async function handleRemove(trackId: string) {
    await favorites.remove(trackId);
  }
</script>

<div class="page-favorites">
  <h1>{$t('routes.favorites')}</h1>

  {#if $favorites.length === 0}
    <p class="empty-message">{$t('search.no_results')}</p>
  {:else}
    <ul class="favorites-list">
      {#each $favorites as entry (entry.track.id)}
        <li class="favorite-item">
          <button class="play-btn" on:click={() => handlePlay(entry.track)} aria-label="Play {entry.track.title}">
            <Play size={14} />
          </button>
          {#if entry.track.thumbnail}
            <img class="thumbnail" src={entry.track.thumbnail} alt={entry.track.title} />
          {/if}
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
            aria-label="Remove {entry.track.title} from favorites"
          >
            <X size={16} />
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
    color: var(--text-secondary, #9ca3af);
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
    gap: 0.75rem;
    padding: 0.6rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
    transition: background 0.15s;
  }

  .favorite-item:hover {
    background: var(--bg-elevated, #1f2937);
  }

  .play-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: color 0.15s;
  }

  .play-btn:hover {
    color: var(--color-accent, #6366f1);
  }

  .thumbnail {
    width: 40px;
    height: 40px;
    border-radius: 4px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .track-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    overflow: hidden;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-meta {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-shrink: 0;
  }

  .track-duration {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .track-source {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 4px;
  }

  .remove-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.3rem;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background 0.15s;
  }

  .remove-btn:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }
</style>