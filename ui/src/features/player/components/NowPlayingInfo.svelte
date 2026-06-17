<script lang="ts">
  import { t } from '@i18n';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import type { Track } from '@shared/types/models';

  export let track: Track | null = null;
</script>

<div class="now-playing-info">
  {#if track}
    <div class="info-layout">
      {#if track.thumbnail}
        <img class="album-art" src={albumArtUrl(track.thumbnail)} alt={track.title} />
      {:else}
        <div class="album-art-placeholder"></div>
      {/if}
      <div class="track-details">
        <h2 class="track-title">{track.title}</h2>
        <span class="track-artist">{track.artist}</span>
        {#if track.album}
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
    width: 240px;
    height: 240px;
    border-radius: 8px;
    object-fit: cover;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .album-art-placeholder {
    width: 240px;
    height: 240px;
    border-radius: 8px;
    background: var(--bg-elevated, #1f2937);
  }

  .track-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.95rem;
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
    padding: 0.15rem 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 4px;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
  }
</style>