<script lang="ts">
  import { navigate } from '@app/router/navigation';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import JellyxLogo from './JellyxLogo.svelte';

  export let id: string;
  export let name: string = 'Unknown Artist';
  export let thumbnail: string | undefined = undefined;
  export let trackCount: number = 0;

  function handleClick() {
    navigate(`/artist/${encodeURIComponent(id)}`);
  }
</script>

<button class="artist-card" on:click={handleClick} aria-label="View {name}">
  {#if albumArtUrl(thumbnail)}
    <img class="artist-art" src={albumArtUrl(thumbnail)} alt={name} />
  {:else}
    <div class="artist-art-placeholder">
      <JellyxLogo size={48} monochrome={true} />
    </div>
  {/if}
  <div class="artist-info">
    <span class="artist-name">{name}</span>
    <span class="artist-meta">{trackCount} track{trackCount === 1 ? '' : 's'}</span>
  </div>
</button>

<style>
  .artist-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 12px;
    background:
      radial-gradient(circle at 10% 10%, rgba(217, 70, 255, 0.06), transparent 60%),
      var(--bg-elevated, #1f2937);
    border: 1px solid rgba(217, 70, 255, 0.08);
    text-align: left;
    cursor: pointer;
    transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
  }

  .artist-card:hover {
    transform: translateY(-3px);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35), 0 0 0 1px rgba(217, 70, 255, 0.15);
    border-color: rgba(217, 70, 255, 0.2);
  }

  .artist-card:focus-visible {
    outline: 2px solid var(--color-jellyx-cyan, #00E5FF);
    outline-offset: 2px;
  }

  .artist-art {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 8px;
    object-fit: cover;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
  }

  .artist-art-placeholder {
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

  .artist-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow: hidden;
  }

  .artist-name {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.15s;
  }

  .artist-card:hover .artist-name {
    color: var(--color-jellyx-magenta, #D946FF);
  }

  .artist-meta {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
