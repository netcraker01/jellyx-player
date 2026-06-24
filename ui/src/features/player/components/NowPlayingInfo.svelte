<script lang="ts">
  import { ListMusic } from 'lucide-svelte';
  import { t } from '@i18n';
  import { navigate } from '@app/router/navigation';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import { normalizeArtistId, normalizeAlbumId } from '@shared/utils/ids';
  import ListPicker from '@features/playlists/components/ListPicker.svelte';
  import type { Track } from '@shared/types/models';
  import { Source } from '@shared/types/models';

  export let track: Track | null = null;

  $: isLocal = track?.source === Source.Local;
  $: artistId = isLocal && track?.artist ? normalizeArtistId(track.artist) : null;
  $: albumId = isLocal && track?.album && track?.artist ? normalizeAlbumId(track.album, track.artist) : null;

  let showPicker = false;
  let pickerX = 0;
  let pickerY = 0;

  function handleOpenListPicker(e: MouseEvent) {
    pickerX = e.clientX;
    pickerY = e.clientY;
    showPicker = true;
  }

  function handleClosePicker() {
    showPicker = false;
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

{#if track && showPicker}
  <ListPicker track={track} visible={showPicker} anchorX={pickerX} anchorY={pickerY} on:close={handleClosePicker} />
{/if}

  <div class="now-playing-info">
  {#if track && albumArtUrl(track.thumbnail)}
    <div class="artwork-backdrop" aria-hidden="true">
      <img class="artwork-backdrop-image" src={albumArtUrl(track.thumbnail)} alt="" />
    </div>
  {/if}
  {#if track}
    <div class="info-layout">
      {#if albumArtUrl(track.thumbnail)}
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
            class="list-btn"
            on:click={handleOpenListPicker}
            aria-label="Add to list"
            type="button"
          >
            <ListMusic size={20} />
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
    position: relative;
    overflow: hidden;
    padding: 1rem;
    border-radius: 24px;
  }

  .artwork-backdrop {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
    z-index: 0;
  }

  .artwork-backdrop::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      180deg,
      rgba(10, 10, 15, 0.72) 0%,
      rgba(10, 10, 15, 0.82) 45%,
      rgba(10, 10, 15, 0.9) 100%
    );
  }

  .artwork-backdrop-image {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 140%;
    height: 140%;
    object-fit: cover;
    transform: translate(-50%, -50%) scale(1.15);
    filter: blur(36px) saturate(1.05);
    opacity: 0.2;
  }

  .info-layout {
    position: relative;
    z-index: 1;
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

  .list-btn {
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

  .list-btn:hover {
    color: var(--color-helix-violet, #8A5CFF);
  }

  .list-btn:active {
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
