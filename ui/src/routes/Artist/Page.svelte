<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { navigate } from 'svelte-routing';
  import { User } from 'lucide-svelte';
  import { t } from '@i18n';
  import {
    artistDetail,
    isLoadingArtistDetail,
    artistDetailError,
  } from '@features/library/stores/artistDetail';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import AlbumCard from '@shared/components/AlbumCard.svelte';
  import { albumArtUrl } from '@shared/utils/assetUrl';

  export let id: string;

  let lastLoadedId: string | null = null;

  $: if (id && id !== lastLoadedId) {
    lastLoadedId = id;
    artistDetail.load(id);
  }

  onDestroy(() => {
    artistDetail.clear();
    lastLoadedId = null;
  });

  function handleAlbumClick(albumId: string) {
    navigate(`/album/${encodeURIComponent(albumId)}`);
  }
</script>

<div class="page-artist">
  {#if $isLoadingArtistDetail}
    <div class="loading-state">{$t('common.loading')}</div>
  {:else if $artistDetailError}
    <div class="error-state">
      <p>{$artistDetailError}</p>
      <button class="back-btn" on:click={() => navigate('/search')}>{$t('common.back')}</button>
    </div>
  {:else if $artistDetail}
    <div class="artist-header">
      {#if albumArtUrl($artistDetail.thumbnail)}
        <img
          class="artist-header-art"
          src={albumArtUrl($artistDetail.thumbnail)}
          alt={$artistDetail.name}
        />
      {:else}
        <div class="artist-header-art placeholder">
          <User size={64} />
        </div>
      {/if}
      <div class="artist-header-info">
        <h1 class="artist-name">{$artistDetail.name}</h1>
      </div>
    </div>

    {#if $artistDetail.topTracks.length > 0}
      <section class="section">
        <h2 class="section-title">{$t('artist.top_tracks')}</h2>
        <div class="track-list">
          {#each $artistDetail.topTracks as track (track.id)}
            <TrackRow {track} />
          {/each}
        </div>
      </section>
    {/if}

    {#if $artistDetail.albums.length > 0}
      <section class="section">
        <h2 class="section-title">{$t('artist.albums')}</h2>
        <div class="albums-grid">
          {#each $artistDetail.albums as album (album.id)}
            <button
              class="album-card-btn"
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
      </section>
    {/if}
  {/if}
</div>

<style>
  .page-artist {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .loading-state,
  .error-state {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
  }

  .error-state {
    color: #fca5a5;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 8px;
  }

  .back-btn {
    margin-top: 1rem;
    padding: 0.5rem 1.25rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    background: transparent;
    color: var(--color-accent, #6366f1);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .back-btn:hover {
    background: var(--color-accent, #6366f1);
    color: white;
  }

  .artist-header {
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    padding: 2rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 12px;
  }

  .artist-header-art {
    width: 180px;
    height: 180px;
    border-radius: 12px;
    object-fit: cover;
    flex-shrink: 0;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .artist-header-art.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary, #111827);
    color: var(--text-secondary, #9ca3af);
  }

  .artist-header-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .artist-name {
    margin: 0;
    color: var(--text-primary, #e0e0e0);
    font-size: 2rem;
    font-weight: 700;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    margin: 0;
    color: var(--text-primary, #e0e0e0);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .track-list {
    display: flex;
    flex-direction: column;
  }

  .albums-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1rem;
  }

  .album-card-btn {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    text-align: left;
    cursor: pointer;
    border-radius: 8px;
  }

  .album-card-btn :global(.album-card) {
    width: 100%;
  }
</style>
