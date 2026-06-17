<script lang="ts">
  import { onMount } from 'svelte';
  import { Link } from 'svelte-routing';
  import { Play, Search } from 'lucide-svelte';
  import { t } from '@i18n';
  import { getHistory } from '@services/commands';
  import { playTrack } from '@shared/utils/actions';
  import type { HistoryEntry, Track } from '@shared/types/models';

  let historyEntries: HistoryEntry[] = [];
  let loading = true;

  onMount(async () => {
    try {
      historyEntries = await getHistory();
    } catch (e) {
      console.error('Failed to load history:', e);
    } finally {
      loading = false;
    }
  });

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

<div class="page-home">
  <h1>{$t('routes.home')}</h1>

  {#if loading}
    <p class="loading">{$t('common.loading')}</p>
  {:else if historyEntries.length === 0}
    <div class="empty-state">
      <Search size={48} />
      <p>{$t('home.empty')}</p>
      <Link to="/search" class="search-link">
        <Search size={16} />
        {$t('home.go_search')}
      </Link>
    </div>
  {:else}
    <section class="recently-played">
      <h2>{$t('home.recently_played')}</h2>
      <ul class="history-list">
        {#each historyEntries as entry (entry.id)}
          <li class="history-item">
            <button class="play-btn" on:click={() => handlePlay(entry.track)} aria-label="Play {entry.track.title}">
              <Play size={14} />
            </button>
            <div class="track-info">
              <span class="track-title">{entry.track.title}</span>
              <span class="track-artist">{entry.track.artist}</span>
            </div>
            <span class="track-duration">{formatDuration(entry.track.duration)}</span>
            <span class="track-source">{entry.track.source}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .page-home {
    padding: 1rem;
  }

  h1 {
    color: var(--text-primary, #e0e0e0);
    font-size: 1.5rem;
    margin-bottom: 1.5rem;
  }

  .loading {
    color: var(--text-secondary, #9ca3af);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    color: var(--text-secondary, #9ca3af);
    text-align: center;
  }

  .empty-state p {
    font-size: 1rem;
  }

  :global(.search-link) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    color: var(--color-accent, #6366f1);
    font-size: 0.9rem;
    transition: background 0.2s, color 0.2s;
  }

  :global(.search-link:hover) {
    background: var(--color-accent, #6366f1);
    color: white;
  }

  .recently-played h2 {
    color: var(--text-secondary, #9ca3af);
    font-size: 1.1rem;
    margin-bottom: 0.75rem;
    font-weight: 500;
  }

  .history-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .history-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
    transition: background 0.15s;
  }

  .history-item:hover {
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

  .track-source {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 4px;
    flex-shrink: 0;
  }
</style>