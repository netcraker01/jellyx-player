<script lang="ts">
  import { navigate } from 'svelte-routing';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import HelixLogo from './HelixLogo.svelte';

  export let id: string;
  export let title: string = 'Unknown Album';
  export let artist: string = 'Unknown Artist';
  export let cover: string | undefined = undefined;
  export let year: number | undefined = undefined;
  export let trackCount: number = 0;

  function handleClick() {
    navigate(`/album/${encodeURIComponent(id)}`);
  }
</script>

<button class="album-card" on:click={handleClick} aria-label="View {title}">
  {#if albumArtUrl(cover)}
    <img class="album-art" src={albumArtUrl(cover)} alt={title} />
  {:else}
    <div class="album-art-placeholder">
      <HelixLogo size={48} monochrome={true} />
    </div>
  {/if}
  <div class="album-info">
    <span class="album-title">{title}</span>
    <span class="album-artist">{artist}</span>
    {#if year}
      <span class="album-meta">{year} • {trackCount} track{trackCount === 1 ? '' : 's'}</span>
    {:else}
      <span class="album-meta">{trackCount} track{trackCount === 1 ? '' : 's'}</span>
    {/if}
  </div>
</button>

<style>
  .album-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 12px;
    background:
      radial-gradient(circle at 10% 10%, rgba(138, 92, 255, 0.06), transparent 60%),
      var(--bg-elevated, #1f2937);
    border: 1px solid rgba(138, 92, 255, 0.08);
    cursor: pointer;
    transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
  }

  .album-card:hover {
    transform: translateY(-3px);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35), 0 0 0 1px rgba(138, 92, 255, 0.15);
    border-color: rgba(138, 92, 255, 0.2);
  }

  .album-card:focus-visible {
    outline: 2px solid var(--color-helix-cyan, #00E5FF);
    outline-offset: 2px;
  }

  .album-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 8px;
    object-fit: cover;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
  }

  .album-art-placeholder {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 8px;
    background:
      radial-gradient(circle at 30% 30%, rgba(0, 229, 255, 0.08), transparent 60%),
      var(--bg-primary, #111827);
    display: flex;
    align-items: center;
    justify-content: center;
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
    transition: color 0.15s;
  }

  .album-card:hover .album-title {
    color: var(--color-helix-cyan, #00E5FF);
  }

  .album-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-meta {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>