<script lang="ts">
  import { Disc, Users, Music, HardDrive } from 'lucide-svelte';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import { navigate } from '@app/router/navigation';
  import { t } from '@i18n';
  import type { GroupedSearchResult, ArtistSummary } from '@shared/types/models';

  export let result: GroupedSearchResult | null = null;
  export let filter: 'all' | 'videos' | 'artists' | 'local' = 'all';
  export let loading: boolean = false;
  export let error: string | null = null;
  export let onFilter: (filter: 'all' | 'videos' | 'artists' | 'local') => void = () => {};
  export let onLoadMore: () => void = () => {};
  export let hasMoreSongs: boolean = false;
  export let loadingMore: boolean = false;

  const filters: { key: 'all' | 'videos' | 'artists' | 'local'; label: string; icon: any }[] = [
    { key: 'all', label: 'All', icon: Music },
    { key: 'videos', label: 'Tracks', icon: Disc },
    { key: 'artists', label: 'Artists', icon: Users },
    { key: 'local', label: $t('search.local'), icon: HardDrive },
  ];

  $: songs = result?.songs ?? [];
  $: artists = result?.artists ?? [];
  $: hasAnyResults = songs.length > 0 || artists.length > 0;
  $: showVideos = filter === 'all' || filter === 'videos' || filter === 'local';
  $: showArtists = filter === 'all' || filter === 'artists';
  $: showGlobalEmpty = !loading && !hasAnyResults && (filter === 'all' || filter === 'local');

  // Infinite scroll sentinel: trigger loadMore when the sentinel enters viewport.
  let sentinelEl: HTMLElement | null = null;
  let observer: IntersectionObserver | null = null;

  $: if (sentinelEl && hasMoreSongs && showVideos) {
    if (observer) observer.disconnect();
    observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting && hasMoreSongs && !loadingMore) {
          onLoadMore();
        }
      },
      { rootMargin: '200px' },
    );
    observer.observe(sentinelEl);
  }

  function handleArtistClick(artist: ArtistSummary) {
    navigate(`/artist/${encodeURIComponent(artist.id)}`);
  }
</script>

<div class="search-results">
  <div class="filter-tabs" role="tablist" aria-label="Search filter">
    {#each filters as { key, label, icon } (key)}
      <button
        class="filter-tab"
        class:active={filter === key}
        role="tab"
        aria-selected={filter === key}
        on:click={() => onFilter(key)}
      >
        <svelte:component this={icon} size={16} />
        <span>{label}</span>
      </button>
    {/each}
  </div>

  {#if error}
    <div class="error-state">Search failed: {error}</div>
  {:else if showGlobalEmpty}
    <div class="empty-state">No results found.</div>
  {:else}
    {#if showVideos}
      <section class="section section-videos">
        <h3 class="section-title">Tracks</h3>
        {#if songs.length > 0}
          <div class="track-list">
            {#each songs as track (track.id)}
              <TrackRow {track} />
            {/each}
          </div>
          {#if hasMoreSongs}
            <div class="load-more-sentinel" bind:this={sentinelEl}>
              {#if loadingMore}
                <span class="loading-more">Loading more...</span>
              {/if}
            </div>
          {/if}
        {:else if loading}
          <p class="empty-section">Searching...</p>
        {:else if filter === 'videos' || filter === 'all' || filter === 'local'}
          <p class="empty-section">No tracks found.</p>
        {/if}
      </section>
    {/if}

    {#if showArtists}
      <section class="section section-artists">
        <h3 class="section-title">Artists</h3>
        {#if artists.length > 0}
          <div class="cards-grid">
            {#each artists as artist (artist.id)}
              <button
                class="artist-card"
                on:click={() => handleArtistClick(artist)}
                aria-label="View {artist.name}"
              >
                {#if artist.thumbnail}
                  <img class="artist-thumb" src={artist.thumbnail} alt={artist.name} />
                {:else}
                  <div class="artist-thumb-placeholder">
                    <Users size={24} />
                  </div>
                {/if}
                <div class="artist-info">
                  <span class="artist-name">{artist.name}</span>
                  <span class="artist-meta">{artist.trackCount} {artist.trackCount === 1 ? 'track' : 'tracks'}</span>
                </div>
              </button>
            {/each}
          </div>
        {:else if loading}
          <p class="empty-section">Searching...</p>
        {:else if filter === 'artists' || filter === 'all'}
          <p class="empty-section">No artists found.</p>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .search-results {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .filter-tabs {
    display: flex;
    gap: 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
    padding-bottom: 0.5rem;
  }

  .filter-tab {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.8rem;
    background: none;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    font-size: 0.85rem;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }

  .filter-tab:hover {
    color: var(--text-primary, #e0e0e0);
    background: var(--bg-elevated, #1f2937);
  }

  .filter-tab.active {
    color: var(--color-accent, #6366f1);
    border-color: var(--color-accent, #6366f1);
    background: rgba(99, 102, 241, 0.08);
  }

  .empty-state {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
  }

  .error-state {
    padding: 0.75rem 1rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 6px;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    margin: 0;
    color: var(--text-primary, #e0e0e0);
    font-size: 1rem;
    font-weight: 600;
  }

  .empty-section {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    margin: 0.25rem 0 0;
  }

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 0.75rem;
  }

  .artist-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 10px;
    padding: 0.75rem;
    cursor: pointer;
    text-align: center;
    transition: background 0.15s, border-color 0.15s, transform 0.15s;
  }

  .artist-card:hover {
    background: var(--bg-hover, #374151);
    border-color: var(--color-accent, #6366f1);
    transform: translateY(-2px);
  }

  .artist-thumb {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    object-fit: cover;
  }

  .artist-thumb-placeholder {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: var(--bg-base, #0a0a0f);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary, #9ca3af);
  }

  .artist-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow: hidden;
    width: 100%;
  }

  .artist-name {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.85rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .artist-meta {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
  }

  .load-more-sentinel {
    display: flex;
    justify-content: center;
    padding: 1rem 0;
    min-height: 2rem;
  }

  .loading-more {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
  }
</style>