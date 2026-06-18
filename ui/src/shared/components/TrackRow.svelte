<script lang="ts">
  import { Play, Plus, Heart, PlayCircle } from 'lucide-svelte';
  import { favorites } from '@features/favorites/stores/favorites';
  import { playTrack, addToQueueAction, playNextAction } from '@shared/utils/actions';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import type { Track } from '@shared/types/models';

  export let track: Track;
  export let showActions: boolean = true;
  export let showSource: boolean = true;

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  async function handlePlay() {
    if (track.streamUrl) {
      await playTrack(track.streamUrl);
    } else if (track.localPath) {
      await playTrack(track.localPath);
    }
  }

  async function handleAddToQueue() {
    await addToQueueAction(track.id);
  }

  async function handlePlayNext() {
    await playNextAction(track.id);
  }

  async function handleAddToFavorites() {
    await favorites.add(track);
  }
</script>

<div class="track-row">
  <button class="play-btn" on:click={handlePlay} aria-label="Play {track.title}">
    <Play size={14} />
  </button>
  {#if albumArtUrl(track.thumbnail)}
    <img class="track-thumb" src={albumArtUrl(track.thumbnail)} alt={track.title} />
  {:else}
    <div class="track-thumb-placeholder"></div>
  {/if}
  <div class="track-info">
    <span class="track-title">{track.title}</span>
    <span class="track-artist">{track.artist}</span>
  </div>
  {#if track.album}
    <span class="track-album">{track.album}</span>
  {/if}
  <div class="track-meta">
    <span class="track-duration">{formatDuration(track.duration)}</span>
    {#if showSource}
      <span class="track-source">{track.source}</span>
    {/if}
  </div>
  {#if showActions}
    <div class="track-actions">
      <button class="action-btn" on:click={handlePlayNext} title="Play Next" aria-label="Play {track.title} next">
        <PlayCircle size={14} />
      </button>
      <button class="action-btn" on:click={handleAddToQueue} title="Add to Queue" aria-label="Add {track.title} to queue">
        <Plus size={14} />
      </button>
      <button class="action-btn fav-btn" on:click={handleAddToFavorites} title="Add to Favorites" aria-label="Add {track.title} to favorites">
        <Heart size={14} />
      </button>
    </div>
  {/if}
</div>

<style>
  .track-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
    transition: background 0.15s;
  }

  .track-row:hover {
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

  .track-thumb {
    width: 40px;
    height: 40px;
    border-radius: 4px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .track-thumb-placeholder {
    width: 40px;
    height: 40px;
    border-radius: 4px;
    background: var(--bg-elevated, #1f2937);
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
    font-size: 0.8rem;
    min-width: 120px;
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

  .track-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .track-row:hover .track-actions {
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
