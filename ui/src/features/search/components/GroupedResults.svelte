<script lang="ts">
  import { Disc, Mic2, Music } from 'lucide-svelte';
  import { navigate } from '@app/router/navigation';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import ArtistCard from '@shared/components/ArtistCard.svelte';
  import AlbumCard from '@shared/components/AlbumCard.svelte';
  import type { GroupedSearchResult, SearchFilter } from '@shared/types/models';

  export let result: GroupedSearchResult | null = null;
  export let filter: SearchFilter | 'all' = 'all';
  export let loading: boolean = false;
  export let error: string | null = null;
  export let onFilter: (filter: SearchFilter | 'all') => void = () => {};

  const filters: { key: SearchFilter | 'all'; label: string; icon: any }[] = [
    { key: 'all', label: 'All', icon: Music },
    { key: 'songs', label: 'Songs', icon: Disc },
    { key: 'artists', label: 'Artists', icon: Mic2 },
    { key: 'albums', label: 'Albums', icon: Disc },
  ];

  function handleArtistClick(id: string) {
    navigate(`/artist/${encodeURIComponent(id)}`);
  }

  function handleAlbumClick(id: string) {
    navigate(`/album/${encodeURIComponent(id)}`);
  }

  $: showSongs = filter === 'all' || filter === 'songs';
  $: showArtists = filter === 'all' || filter === 'artists';
  $: showAlbums = filter === 'all' || filter === 'albums';
  $: hasAnyResults = Boolean(
    result && (result.songs.length > 0 || result.artists.length > 0 || result.albums.length > 0),
  );
  $: showGlobalEmpty = result && filter === 'all' && !hasAnyResults;
</script>

<div class="grouped-results">
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

  {#if loading}
    <div class="loading-state">Searching...</div>
  {:else if error}
    <div class="error-state">Search failed: {error}</div>
  {:else if showGlobalEmpty}
    <div class="empty-state">No results found.</div>
  {:else if result}
    {#if showSongs}
      <section class="section section-songs">
        <h3 class="section-title">Songs</h3>
        {#if result.songs.length > 0}
          <div class="track-list">
            {#each result.songs as track (track.id)}
              <TrackRow {track} />
            {/each}
          </div>
        {:else}
          <p class="empty-section">No songs found.</p>
        {/if}
      </section>
    {/if}

    {#if showArtists}
      <section class="section section-artists">
        <h3 class="section-title">Artists</h3>
        {#if result.artists.length > 0}
          <div class="cards-grid">
            {#each result.artists as artist (artist.id)}
              <button
                class="card-btn"
                on:click={() => handleArtistClick(artist.id)}
                aria-label="View {artist.name}"
              >
                <ArtistCard
                  id={artist.id}
                  name={artist.name}
                  thumbnail={artist.thumbnail}
                  trackCount={artist.trackCount}
                />
              </button>
            {/each}
          </div>
        {:else}
          <p class="empty-section">No artists found.</p>
        {/if}
      </section>
    {/if}

    {#if showAlbums}
      <section class="section section-albums">
        <h3 class="section-title">Albums</h3>
        {#if result.albums.length > 0}
          <div class="cards-grid">
            {#each result.albums as album (album.id)}
              <button
                class="card-btn"
                on:click={() => handleAlbumClick(album.id)}
                aria-label="View {album.title}"
              >
                <AlbumCard
                  id={album.id}
                  title={album.title}
                  artist={album.artist}
                  cover={album.cover}
                  year={album.year}
                  trackCount={album.trackCount}
                />
              </button>
            {/each}
          </div>
        {:else}
          <p class="empty-section">No albums found.</p>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .grouped-results {
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

  .loading-state,
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
    gap: 1rem;
  }

  .card-btn {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    text-align: left;
    cursor: pointer;
    border-radius: 8px;
  }

  .card-btn :global(.artist-card),
  .card-btn :global(.album-card) {
    width: 100%;
  }
</style>
