<script lang="ts">
  import { Play, X } from 'lucide-svelte';
  import { favorites } from '@features/favorites/stores/favorites';
  import { playTrack } from '@shared/utils/actions';
  import type { FavoriteEntry, Track } from '@shared/types/models';

  export let entries: FavoriteEntry[] = [];

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

  async function handleRemove(trackId: string) {
    await favorites.remove(trackId);
  }
</script>

<div class="favorites-list">
  {#each entries as entry (entry.track.id)}
    <div class="favorite-item">
      <button class="play-btn" on:click={() => handlePlay(entry.track)} aria-label="Play {entry.track.title}">
        <Play size={14} />
      </button>
      <div class="track-info">
        <span class="track-title">{entry.track.title}</span>
        <span class="track-artist">{entry.track.artist}</span>
      </div>
      <span class="track-duration">{formatDuration(entry.track.duration)}</span>
      <button class="remove-btn" on:click={() => handleRemove(entry.track.id)} aria-label="Remove">
        <X size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .favorites-list {
    display: flex;
    flex-direction: column;
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

  .track-duration {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
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