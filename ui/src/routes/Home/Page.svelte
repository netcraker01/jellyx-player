<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '@app/router/navigation';
  import { Search, Sparkles, Clock, ListMusic, Play } from 'lucide-svelte';
  import { t } from '@i18n';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import {
    suggestionCategories,
    isLoadingCategories,
    loadSuggestionCategories,
  } from '@features/search/stores/suggestions';
  import { searchQuery } from '@features/search/stores/search';
  import { searchGrouped } from '@features/search/stores/searchGrouped';
  import { getHistory, getRecentPlaylists, getPlaylistTracks } from '@services/commands';
  import { playTrack, addToQueueAction } from '@shared/utils/actions';
  import type { HistoryEntry, UserPlaylist } from '@shared/types/models';

  let recentTracks: HistoryEntry[] = [];
  let recentLists: UserPlaylist[] = [];

  onMount(() => {
    loadSuggestionCategories();
    loadRecent();
  });

  async function loadRecent() {
    try {
      const [history, lists] = await Promise.all([
        getHistory(),
        getRecentPlaylists(5),
      ]);
      recentTracks = (history ?? []).slice(0, 5);
      recentLists = lists ?? [];
    } catch {
      // ignore — recents just won't show
    }
  }

  function handleCategoryClick(query: string) {
    searchQuery.set(query);
    searchGrouped(query);
    navigate('/search');
  }

  function openPlaylist(id: string) {
    navigate(`/playlists/${encodeURIComponent(id)}`);
  }

  async function handlePlayPlaylist(id: string) {
    try {
      const entries = await getPlaylistTracks(id);
      if (entries.length === 0) return;
      await playTrack(entries[0].track);
      for (let i = 1; i < entries.length; i++) {
        await addToQueueAction(entries[i].track);
      }
    } catch {
      // ignore — user will see no playback
    }
  }
</script>

<div class="page-home">
  <div class="hero-center">
    <div class="brand-glow" aria-hidden="true"></div>
    <HelixLogo size={96} />

    <h1 class="brand-heading">
      {$t('home.welcome')}
    </h1>
    <p class="tagline">
      {($t('home.tagline') !== 'home.tagline' ? $t('home.tagline') : 'Your sound, your space.')}
    </p>

    <button class="search-btn" on:click={() => navigate('/search')}>
      <Search size={18} />
      {$t('common.search')}
    </button>
  </div>

  {#if $suggestionCategories.length > 0}
    <div class="discover-section">
      <div class="discover-header">
        <Sparkles size={18} />
        <span>{$t('home.discover') ?? 'Discover'}</span>
      </div>
      <div class="category-grid">
        {#each $suggestionCategories as cat}
          <button
            class="category-card"
            style="--cat-color: {cat.color}"
            on:click={() => handleCategoryClick(cat.query)}
          >
            <span class="cat-label">{cat.label}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if recentTracks.length > 0 || recentLists.length > 0}
    <div class="recents-section">
      {#if recentTracks.length > 0}
        <div class="recent-group">
          <div class="recent-header">
            <Clock size={16} />
            <span>{$t('home.recently_played')}</span>
          </div>
          <div class="recent-tracks">
            {#each recentTracks as entry (entry.id)}
              <TrackRow track={entry.track} />
            {/each}
          </div>
        </div>
      {/if}

      {#if recentLists.length > 0}
        <div class="recent-group">
          <div class="recent-header">
            <ListMusic size={16} />
            <span>{$t('home.recent_lists')}</span>
          </div>
          <div class="recent-lists">
            {#each recentLists as pl (pl.id)}
              <div class="recent-list-card-wrapper">
                <button class="recent-list-card" on:click={() => openPlaylist(pl.id)} type="button">
                  <ListMusic size={20} />
                  <span class="list-title">{pl.title}</span>
                </button>
                <button class="recent-list-play" on:click|stopPropagation={() => handlePlayPlaylist(pl.id)} title={$t('common.play')} type="button">
                  <Play size={14} fill="currentColor" />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page-home {
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    padding: 2rem;
    overflow-y: auto;
    background: radial-gradient(
      ellipse at 50% 30%,
      rgba(109, 92, 255, 0.08) 0%,
      transparent 60%
    ),
    var(--bg-base, #0a0a0f);
  }

  .hero-center {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    text-align: center;
    z-index: 1;
  }

  .brand-glow {
    position: absolute;
    top: -2rem;
    width: 12rem;
    height: 12rem;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      rgba(0, 229, 255, 0.15) 0%,
      rgba(138, 92, 255, 0.08) 45%,
      transparent 70%
    );
    filter: blur(24px);
    pointer-events: none;
    z-index: -1;
  }

  .brand-heading {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    background: linear-gradient(135deg, #00E5FF 0%, #8A5CFF 58%, #D946FF 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    color: var(--color-accent, #6366f1);
  }

  @supports not (-webkit-background-clip: text) {
    .brand-heading {
      color: var(--color-accent, #6366f1);
      -webkit-text-fill-color: unset;
    }
  }

  .tagline {
    margin: 0;
    font-size: 1rem;
    color: var(--text-secondary, #9ca3af);
  }

  .search-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding: 0.65rem 1.75rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    background: transparent;
    color: var(--color-accent, #6366f1);
    font-size: 1rem;
    cursor: pointer;
    transition: background 0.2s, color 0.2s, box-shadow 0.2s;
  }

  .search-btn:hover {
    background: var(--color-accent, #6366f1);
    color: #ffffff;
    box-shadow: 0 0 12px rgba(138, 92, 255, 0.35);
  }

  .search-btn:active {
    transform: translateY(1px);
  }

  .discover-section {
    width: 100%;
    max-width: 720px;
    margin-top: 2.5rem;
  }

  .discover-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
  }

  .category-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0.75rem;
  }

  .category-card {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 12px;
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--cat-color) 15%, var(--bg-elevated, #1a1a2e)),
      color-mix(in srgb, var(--cat-color) 5%, var(--bg-elevated, #1a1a2e))
    );
    color: var(--cat-color);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.2s, background 0.2s;
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px color-mix(in srgb, var(--cat-color) 30%, transparent);
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--cat-color) 25%, var(--bg-elevated, #1a1a2e)),
      color-mix(in srgb, var(--cat-color) 10%, var(--bg-elevated, #1a1a2e))
    );
  }

  .category-card:active {
    transform: translateY(0);
  }

  .cat-label {
    text-align: center;
  }

  .recents-section {
    width: 100%;
    max-width: 720px;
    margin-top: 2.5rem;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .recent-group {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .recent-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
  }

  .recent-tracks {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .recent-lists {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.75rem;
  }

  .recent-list-card-wrapper {
    position: relative;
  }

  .recent-list-card {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.75rem 2.8rem 0.75rem 1rem;
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 10px;
    background: var(--bg-elevated, #1f2937);
    color: var(--text-primary, #e0e0e0);
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s, transform 0.15s;
  }

  .recent-list-card:hover {
    background: var(--bg-hover, #374151);
    border-color: var(--color-accent, #6366f1);
    transform: translateY(-2px);
  }

  .recent-list-play {
    position: absolute;
    top: 50%;
    right: 0.6rem;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: rgba(99, 102, 241, 0.15);
    color: var(--color-accent, #6366f1);
    cursor: pointer;
    transition: background 0.2s, color 0.2s, transform 0.15s;
  }

  .recent-list-play:hover {
    background: var(--color-accent, #6366f1);
    color: white;
    transform: translateY(-50%) scale(1.05);
  }

  .list-title {
    font-size: 0.9rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
