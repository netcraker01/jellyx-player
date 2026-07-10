<script lang="ts">
  import { onDestroy } from 'svelte';
  import { navigate } from '@app/router/navigation';
  import { Play } from 'lucide-svelte';
  import { t } from '@i18n';
  import {
    albumDetail,
    isLoadingAlbumDetail,
    albumDetailError,
  } from '@features/library/stores/albumDetail';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import JellyxLogo from '@shared/components/JellyxLogo.svelte';
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
          <JellyxLogo size={72} monochrome={true} />
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
    box-shadow: 0 0 12px rgba(138, 92, 255, 0.35);
  }

  .album-header {
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    padding: 2.5rem;
    border-radius: 16px;
    background:
      radial-gradient(circle at 10% 10%, rgba(138, 92, 255, 0.08), transparent 60%),
      var(--bg-elevated, #1f2937);
    border: 1px solid rgba(138, 92, 255, 0.08);
    position: relative;
    overflow: hidden;
  }

  .album-header::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(0, 229, 255, 0.04) 0%, transparent 50%);
    pointer-events: none;
  }

  .album-cover-art {
    width: 200px;
    height: 200px;
    border-radius: 12px;
    object-fit: cover;
    flex-shrink: 0;
    box-shadow:
      0 12px 40px rgba(0, 0, 0, 0.45),
      0 0 0 1px rgba(138, 92, 255, 0.15);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }

  .album-cover-art:hover {
    transform: translateY(-2px);
    box-shadow:
      0 16px 48px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(138, 92, 255, 0.25);
  }

  .album-cover-art.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      radial-gradient(circle at 30% 30%, rgba(0, 229, 255, 0.08), transparent 60%),
      var(--bg-primary, #111827);
  }

  .album-header-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: flex-start;
    position: relative;
    z-index: 1;
  }

  .album-title {
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
    .album-title {
      color: var(--text-primary, #e0e0e0);
      -webkit-text-fill-color: unset;
    }
  }

  .artist-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--color-jellyx-cyan, #00E5FF);
    font-size: 1rem;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
    transition: color 0.15s;
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
    padding: 0.65rem 1.5rem;
    border: none;
    border-radius: 24px;
    background: linear-gradient(135deg, #00E5FF 0%, #8A5CFF 70%, #D946FF 100%);
    color: white;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    transition: transform 0.15s ease, box-shadow 0.2s ease, filter 0.2s ease;
    position: relative;
    overflow: hidden;
  }

  .play-album-btn::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(255,255,255,0.12), transparent 60%);
    pointer-events: none;
  }

  .play-album-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 24px rgba(138, 92, 255, 0.35);
    filter: brightness(1.08);
  }

  .play-album-btn:active {
    transform: scale(0.97);
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
