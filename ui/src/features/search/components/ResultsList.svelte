<script lang="ts">
  import { Play, Plus, Heart, PlayCircle } from 'lucide-svelte';
  import { t } from '@i18n';
  import { playTrack, addToQueueAction, playNextAction } from '@shared/utils/actions';
  import { favorites } from '@features/favorites/stores/favorites';
  import type { Track } from '@shared/types/models';

  export let tracks: Track[] = [];
  export let loading = false;
  export let error: string | null = null;
  export let hasSearched = false;

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  async function handlePlay(track: Track) {
    if (track.streamUrl) {
      await playTrack(track.streamUrl);
    } else if (track.localPath) {
      await playTrack(track.localPath);
    }
  }

  async function handleAddToQueue(track: Track) {
    await addToQueueAction(track.id);
  }

  async function handlePlayNext(track: Track) {
    await playNextAction(track.id);
  }

  async function handleAddToFavorites(track: Track) {
    await favorites.add(track);
  }
</script>

<div class="results-list">
  {#if loading}
    <div class="loading">{$t('search.loading')}</div>
  {:else if error}
    <div class="error">{$t('errors.SEARCH_FAILED', { reason: error })}</div>
  {:else if hasSearched && tracks.length === 0}
    <div class="no-results">{$t('search.no_results')}</div>
  {:else}
    {#each tracks as track (track.id)}
      <div class="result-row">
        <button class="play-btn" on:click={() => handlePlay(track)} aria-label="Play {track.title}">
          <Play size={14} />
        </button>
        {#if track.thumbnail}
          <img class="thumbnail" src={track.thumbnail} alt={track.title} />
        {/if}
        <div class="track-info">
          <span class="track-title">{track.title}</span>
          <span class="track-artist">{track.artist}</span>
          {#if track.album}
            <span class="track-album">{track.album}</span>
          {/if}
        </div>
        <div class="track-meta">
          <span class="track-duration">{formatDuration(track.duration)}</span>
          <span class="track-source">{track.source}</span>
        </div>
        <div class="track-actions">
          <button class="action-btn" on:click={() => handlePlayNext(track)} title={$t('search.play_next')} aria-label={$t('search.play_next')}>
            <PlayCircle size={14} />
          </button>
          <button class="action-btn" on:click={() => handleAddToQueue(track)} title={$t('search.add_to_queue')} aria-label={$t('search.add_to_queue')}>
            <Plus size={14} />
          </button>
          <button class="action-btn fav-btn" on:click={() => handleAddToFavorites(track)} title={$t('search.add_to_favorites')} aria-label={$t('search.add_to_favorites')}>
            <Heart size={14} />
          </button>
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .results-list {
    display: flex;
    flex-direction: column;
  }

  .loading,
  .no-results {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
  }

  .error {
    padding: 0.75rem 1rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 6px;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .result-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
    transition: background 0.15s;
  }

  .result-row:hover {
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

  .track-album {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-meta {
    display: flex;
    gap: 0.5rem;
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

  .track-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .result-row:hover .track-actions {
    opacity: 1;
  }

  .action-btn {
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

  .action-btn:hover {
    color: var(--color-accent, #6366f1);
    background: rgba(99, 102, 241, 0.1);
  }

  .fav-btn:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }
</style>