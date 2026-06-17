<script lang="ts">
  import { Play, Shuffle } from 'lucide-svelte';
  import { t } from '@i18n';
  import { queue, currentIndex, shuffle, playTrack } from '../stores/player';
  import type { Track } from '@shared/types/models';

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
</script>

  <div class="queue">
    <div class="queue-header">
      <h3 class="queue-title">{$t('now_playing.queue_title')}</h3>
      {#if $shuffle}
        <span class="shuffle-badge">
          <Shuffle size={12} />
          {$t('player.shuffle')}
        </span>
      {/if}
    </div>
    {#if $queue.length === 0}
      <p class="queue-empty">{$t('now_playing.queue_empty')}</p>
    {:else}
      <ul class="queue-list">
        {#each $queue as track, i (track.id)}
          <li class="queue-item" class:current={$currentIndex === i}>
            <span class="track-number">
              {#if $currentIndex === i}
                <Play size={12} />
              {:else}
                {i + 1}
              {/if}
            </span>
            <button class="play-btn" on:click={() => handlePlay(track)} aria-label="Play {track.title}">
              <Play size={12} />
            </button>
            <div class="track-info">
              <span class="track-title">{track.title}</span>
              <span class="track-artist">{track.artist}</span>
            </div>
            <span class="track-duration">{formatDuration(track.duration)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

<style>
  .queue {
    padding: 0.75rem;
  }

  .queue-title {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
    font-weight: 500;
    margin: 0;
  }

  .queue-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .shuffle-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--color-accent, #6366f1);
  }

  .queue-empty {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    font-style: italic;
  }

  .queue-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .queue-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .queue-item:hover {
    background: var(--bg-elevated, #1f2937);
  }

  .queue-item.current {
    background: rgba(99, 102, 241, 0.12);
    border-left: 3px solid var(--color-accent, #6366f1);
  }

  .play-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.2rem;
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

  .track-number {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .track-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.85rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-duration {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
</style>