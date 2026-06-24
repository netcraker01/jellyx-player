<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { navigate } from '@app/router/navigation';
  import { t } from '@i18n';
  import { Heart } from 'lucide-svelte';
  import { artistFavorites } from '@features/artist-favorites/stores/artistFavorites';
  import {
    artistDetail,
    isLoadingArtistDetail,
    artistDetailError,
  } from '@features/library/stores/artistDetail';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import AlbumCard from '@shared/components/AlbumCard.svelte';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import { albumArtUrl } from '@shared/utils/assetUrl';

  export let id: string;

  let lastLoadedId: string | null = null;
  let isFav = false;

  async function checkFavorite() {
    if (!id) return;
    isFav = await artistFavorites.isFavorite(id);
  }

  async function toggleFavorite() {
    if (!id || !$artistDetail) return;
    if (isFav) {
      await artistFavorites.remove(id);
      isFav = false;
    } else {
      await artistFavorites.add(id, $artistDetail.name, $artistDetail.thumbnail);
      isFav = true;
    }
  }

  $: if (id && id !== lastLoadedId) {
    lastLoadedId = id;
    artistDetail.load(id);
    checkFavorite();
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
          <HelixLogo size={72} monochrome={true} />
        </div>
      {/if}
      <div class="artist-header-info">
        <div class="artist-header-row">
          <h1 class="artist-name">{$artistDetail.name}</h1>
          <button
            class="heart-btn"
            class:filled={isFav}
            on:click={toggleFavorite}
            title={isFav ? 'Remove from favorites' : 'Add to favorites'}
            type="button"
          >
            {#if isFav}
              <Heart size={24} fill="currentColor" />
            {:else}
              <Heart size={24} />
            {/if}
          </button>
        </div>
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
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .loading-state,
  .error-state {
    padding: 3rem 1rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
  }

  .error-state {
    color: #fca5a5;
    background:
      radial-gradient(circle at 30% 30%, rgba(239, 68, 68, 0.08), transparent 70%),
      rgba(239, 68, 68, 0.06);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 12px;
  }

  .back-btn {
    margin-top: 1rem;
    padding: 0.5rem 1.5rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    background: transparent;
    color: var(--color-accent, #6366f1);
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: background 0.2s ease, color 0.2s ease, box-shadow 0.2s ease;
  }

  .back-btn:hover {
    background: var(--color-accent, #6366f1);
    color: #ffffff;
    box-shadow: 0 0 12px rgba(217, 70, 255, 0.35);
  }

  .artist-header {
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    padding: 2.5rem;
    border-radius: 16px;
    background:
      radial-gradient(circle at 10% 10%, rgba(217, 70, 255, 0.08), transparent 60%),
      var(--bg-elevated, #1f2937);
    border: 1px solid rgba(217, 70, 255, 0.08);
    position: relative;
    overflow: hidden;
  }

  .artist-header::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(138, 92, 255, 0.04) 0%, transparent 50%);
    pointer-events: none;
  }

  .artist-header-art {
    width: 180px;
    height: 180px;
    border-radius: 12px;
    object-fit: cover;
    flex-shrink: 0;
    box-shadow:
      0 12px 40px rgba(0, 0, 0, 0.45),
      0 0 0 1px rgba(217, 70, 255, 0.15);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }

  .artist-header-art:hover {
    transform: translateY(-2px);
    box-shadow:
      0 16px 48px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(217, 70, 255, 0.25);
  }

  .artist-header-art.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      radial-gradient(circle at 30% 30%, rgba(0, 229, 255, 0.08), transparent 60%),
      var(--bg-primary, #111827);
    color: var(--text-secondary, #9ca3af);
  }

  .artist-header-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    position: relative;
    z-index: 1;
  }

  .artist-header-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .heart-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.5rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, background 0.2s;
  }

  .heart-btn:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .heart-btn.filled {
    color: #ef4444;
  }

  .artist-name {
    margin: 0;
    font-size: 2.25rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    background: linear-gradient(135deg, #00E5FF 0%, #8A5CFF 58%, #D946FF 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    color: var(--color-accent, #6366f1);
  }

  @supports not (-webkit-background-clip: text) {
    .artist-name {
      color: var(--text-primary, #e0e0e0);
      -webkit-text-fill-color: unset;
    }
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section-title {
    margin: 0;
    color: var(--text-primary, #e0e0e0);
    font-size: 1.1rem;
    font-weight: 600;
    letter-spacing: -0.01em;
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
    border-radius: 12px;
  }

  .album-card-btn:focus-visible {
    outline: 2px solid var(--color-helix-cyan, #00E5FF);
    outline-offset: 2px;
    border-radius: 12px;
  }

  .album-card-btn :global(.album-card) {
    width: 100%;
  }
</style>
