<script lang="ts">
  import { albumArtUrl } from '@shared/utils/assetUrl';

  export let title: string = 'Unknown Album';
  export let artist: string = 'Unknown Artist';
  export let thumbnail: string | undefined = undefined;
</script>

<div class="album-card">
  {#if albumArtUrl(thumbnail)}
    <img class="album-art" src={albumArtUrl(thumbnail)} alt={title} />
  {:else}
    <div class="album-art-placeholder">
      <span class="placeholder-text">♪</span>
    </div>
  {/if}
  <div class="album-info">
    <span class="album-title">{title}</span>
    <span class="album-artist">{artist}</span>
  </div>
</div>

<style>
  .album-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 8px;
    background: var(--bg-elevated, #1f2937);
    cursor: pointer;
    transition: transform 0.15s, box-shadow 0.15s;
  }

  .album-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  .album-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 6px;
    object-fit: cover;
  }

  .album-art-placeholder {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 6px;
    background: var(--bg-primary, #111827);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .placeholder-text {
    font-size: 2rem;
    color: var(--text-secondary, #9ca3af);
  }

  .album-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow: hidden;
  }

  .album-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>