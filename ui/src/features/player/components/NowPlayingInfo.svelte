<script lang="ts">
  import { Heart } from 'lucide-svelte';
  import { t } from '@i18n';
  import { navigate } from '@app/router/navigation';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import { normalizeArtistId, normalizeAlbumId } from '@shared/utils/ids';
  import { isCurrentTrackFavorited } from '../stores/player';
  import { favorites } from '@features/favorites/stores/favorites';
  import type { Track } from '@shared/types/models';

  export let track: Track | null = null;

  $: artistId = track?.artist ? normalizeArtistId(track.artist) : null;
  $: albumId = track?.album && track?.artist ? normalizeAlbumId(track.album, track.artist) : null;

  async function handleFavoriteToggle() {
    if (!track?.id) return;
    await favorites.toggle(track.id);
  }

  function handleOpenArtist() {
    if (artistId) {
      navigate(`/artist/${encodeURIComponent(artistId)}`);
    }
  }

  function handleOpenAlbum() {
    if (albumId) {
      navigate(`/album/${encodeURIComponent(albumId)}`);
    }
  }
</script>

<div class="now-playing-info">
  {#if track}
    <div class="info-layout">
      {#if track.thumbnail}
        <img class="album-art" src={albumArtUrl(track.thumbnail)} alt={track.title} />
      {:else}
        <div class="album-art-placeholder">
          <HelixLogo size={72} monochrome={true} />
        </div>
      {/if}
      <div class="track-details">
        <div class="title-row">
          <h2 class="track-title">{track.title}</h2>
          <button
            class="favorite-btn"
            class:active={$isCurrentTrackFavorited}
            on:click={handleFavoriteToggle}
            aria-label={$isCurrentTrackFavorited ? $t('player.remove_from_favorites') : $t('player.add_to_favorites')}
          >
            <Heart size={20} fill={$isCurrentTrackFavorited ? 'currentColor' : 'none'} />
          </button>
        </div>
        {#if artistId}
          <button class="track-artist link" on:click={handleOpenArtist}>{track.artist}</button>
        {:else}
          <span class="track-artist">{track.artist}</span>
        {/if}
        {#if albumId}
          <button class="track-album link" on:click={handleOpenAlbum}>{track.album}</button>
        {:else if track.album}
          <span class="track-album">{track.album}</span>
        {/if}
        <span class="track-source">{track.source}</span>
      </div>
    </div>
  {:else}
    <div class="empty-state">
      <p>{$t('now_playing.no_track')}</p>
    </div>
  {/if}
</div>

<style>
  .now-playing-info {
    padding: 1rem;
  }

  .info-layout {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    text-align: center;
  }

  .album-art {
    width: 260px;
    height: 260px;
    border-radius: 16px;
    object-fit: cover;
    box-shadow:
      0 12px 40px rgba(0, 0, 0, 0.45),
      0 0 0 1px rgba(138, 92, 255, 0.15);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }

  .album-art:hover {
    transform: translateY(-2px);
    box-shadow:
      0 16px 48px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(138, 92, 255, 0.25);
  }

  .album-art-placeholder {
    width: 260px;
    height: 260px;
    border-radius: 16px;
    background:
      radial-gradient(circle at 30% 30%, rgba(0, 229, 255, 0.08), transparent 60%),
      var(--bg-elevated, #1f2937);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .album-art-placeholder::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(138, 92, 255, 0.06) 0%, transparent 60%);
  }

  .track-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .title-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 1.35rem;
    font-weight: 700;
    margin: 0;
    letter-spacing: -0.01em;
  }

  .favorite-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, transform 0.1s;
  }

  .favorite-btn:hover {
    color: var(--color-helix-magenta, #D946FF);
  }

  .favorite-btn.active {
    color: var(--color-helix-magenta, #D946FF);
  }

  .favorite-btn:active {
    transform: scale(0.95);
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.95rem;
  }

  .track-artist.link,
  .track-album.link {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .track-artist.link:hover,
  .track-album.link:hover {
    color: var(--color-helix-cyan, #00E5FF);
  }

  .track-album {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
  }

  .track-source {
    display: inline-block;
    margin-top: 0.25rem;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    padding: 0.2rem 0.6rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 6px;
    border: 1px solid var(--border-color, #1f2937);
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
  }
</style>
