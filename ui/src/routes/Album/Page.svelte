<script lang="ts">
  import { onDestroy } from 'svelte';
  import { navigate } from 'svelte-routing';
  import { Play } from 'lucide-svelte';
  import { t } from '@i18n';
  import {
    albumDetail,
    isLoadingAlbumDetail,
    albumDetailError,
  } from '@features/library/stores/albumDetail';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import { playAlbum } from '@services/commands';
  import { notifications } from '@shared/stores/notifications';

  export let id: string;

  let lastLoadedId: string | null = null;

  $: if (id && id !== lastLoadedId) {
    lastLoadedId = id;
    albumDetail.load(id);
  }

  onDestroy(() => {
    albumDetail.clear();
    lastLoadedId = null;
  });

  function handleArtistClick(artistId: string) {
    navigate(`/artist/${encodeURIComponent(artistId)}`);
  }

  async function handlePlayAlbum() {
    if (!id) return;
    try {
      await playAlbum(id);
      notifications.push({
        type: 'success',
        title: 'Now Playing',
        message: 'Album started',
        dismissible: true,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      notifications.push({
        type: 'error',
        title: 'Playback Error',
        message: msg,
        dismissible: true,
      });
    }
  }
</script>

<div class="page-album">
  {#if $isLoadingAlbumDetail}
    <div class="loading-state">{$t('common.loading')}</div>
  {:else if $albumDetailError}
    <div class="error-state">
      <p>{$albumDetailError}</p>
      <button class="back-btn" on:click={() => navigate('/search')}>{$t('common.back')}</button>
    </div>
  {:else if $albumDetail}
    <div class="album-header">
      {#if albumArtUrl($albumDetail.cover)}
        <img
          class="album-cover-art"
          src={albumArtUrl($albumDetail.cover)}
          alt={$albumDetail.title}
        />
      {:else}
        <div class="album-cover-art placeholder">
          <span class="placeholder-icon">♪</span>
        </div>
      {/if}
      <div class="album-header-info">
        <h1 class="album-title">{$albumDetail.title}</h1>
        {#if $albumDetail.artistId}
          <button
            class="artist-link"
            on:click={() => handleArtistClick($albumDetail.artistId)}
          >
            {$albumDetail.artist}
          </button>
        {:else}
          <span class="artist-name">{$albumDetail.artist}</span>
        {/if}
        {#if $albumDetail.year}
          <span class="album-year">{$albumDetail.year}</span>
        {/if}
        <button class="play-album-btn" on:click={handlePlayAlbum}>
          <Play size={18} />
          {$t('album.play_album')}
        </button>
      </div>
    </div>

    {#if $albumDetail.tracks.length > 0}
      <section class="section">
        <h2 class="section-title">{$t('album.tracks')}</h2>
        <ol class="track-list">
          {#each $albumDetail.tracks as track, index (track.id)}
            <li class="track-item">
              <span class="track-number">{index + 1}</span>
              <TrackRow {track} showSource={false} />
            </li>
          {/each}
        </ol>
      </section>
    {/if}
  {/if}
</div>

<style>
  .page-album {
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

  .album-header {
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    padding: 2rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 12px;
  }

  .album-cover-art {
    width: 200px;
    height: 200px;
    border-radius: 12px;
    object-fit: cover;
    flex-shrink: 0;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .album-cover-art.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary, #111827);
  }

  .placeholder-icon {
    font-size: 3rem;
    color: var(--text-secondary, #9ca3af);
  }

  .album-header-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .album-title {
    margin: 0;
    color: var(--text-primary, #e0e0e0);
    font-size: 2rem;
    font-weight: 700;
  }

  .artist-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--color-accent, #6366f1);
    font-size: 1rem;
    cursor: pointer;
    text-decoration: underline;
  }

  .artist-link:hover {
    color: var(--text-primary, #e0e0e0);
  }

  .artist-name {
    color: var(--text-secondary, #9ca3af);
    font-size: 1rem;
  }

  .album-year {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .play-album-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 1rem;
    padding: 0.6rem 1.25rem;
    border: none;
    border-radius: 24px;
    background: var(--color-accent, #6366f1);
    color: white;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    transition: background 0.2s, transform 0.1s;
  }

  .play-album-btn:hover {
    background: #4f46e5;
  }

  .play-album-btn:active {
    transform: scale(0.98);
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
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .track-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .track-number {
    width: 1.5rem;
    text-align: right;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .track-item :global(.track-row) {
    flex: 1;
  }
</style>
