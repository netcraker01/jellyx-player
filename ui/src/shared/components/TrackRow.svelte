<script lang="ts">
  import { Play, Plus, PlayCircle, ListMusic } from 'lucide-svelte';
  import ListPicker from '@features/playlists/components/ListPicker.svelte';
  import { playTrack, addToQueueAction, playNextAction } from '@shared/utils/actions';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import HelixLogo from './HelixLogo.svelte';
  import type { Track } from '@shared/types/models';

  export let track: Track;
  export let showActions: boolean = true;
  export let showSource: boolean = true;

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  async function handlePlay() {
    await playTrack(track);
  }

  async function handleAddToQueue() {
    await addToQueueAction(track);
  }

  async function handlePlayNext() {
    await playNextAction(track);
  }

  let showPicker = false;
  let pickerAnchorX = 0;
  let pickerAnchorY = 0;

  function handleOpenPicker(e: MouseEvent) {
    pickerAnchorX = e.clientX;
    pickerAnchorY = e.clientY;
    showPicker = true;
  }

  function handleClosePicker() {
    showPicker = false;
  }
</script>

{#if showPicker}
  <ListPicker {track} visible={showPicker} anchorX={pickerAnchorX} anchorY={pickerAnchorY} on:close={handleClosePicker} />
{/if}

<div class="track-row">
  <button class="play-btn" on:click={handlePlay} aria-label="Play {track.title}">
    <Play size={14} />
  </button>
  {#if albumArtUrl(track.thumbnail)}
    <img class="track-thumb" src={albumArtUrl(track.thumbnail)} alt={track.title} />
  {:else}
    <div class="track-thumb-placeholder">
      <HelixLogo size={20} monochrome={true} />
    </div>
  {/if}
  <div class="track-info">
    <span class="track-title">{track.title}</span>
    <span class="track-artist">{track.artist}</span>
  </div>
  {#if track.album}
    <span class="track-album">{track.album}</span>
  {/if}
  <div class="track-meta">
    <span class="track-duration">{formatDuration(track.duration)}</span>
    {#if showSource}
      <span class="track-source">{track.source}</span>
    {/if}
  </div>
  {#if showActions}
    <div class="track-actions">
      <button class="action-btn" on:click={handlePlayNext} title="Play Next" aria-label="Play {track.title} next">
        <PlayCircle size={14} />
      </button>
      <button class="action-btn" on:click={handleAddToQueue} title="Add to Queue" aria-label="Add {track.title} to queue">
        <Plus size={14} />
      </button>
      <button class="action-btn list-btn" on:click={handleOpenPicker} title="Add to List" aria-label="Add {track.title} to a list">
        <ListMusic size={14} />
      </button>
    </div>
  {/if}
</div>

<style>
  .track-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
    transition: background 0.2s ease, border-color 0.2s ease;
    border-radius: 6px;
  }

  .track-row:hover {
    background: rgba(138, 92, 255, 0.06);
    border-bottom-color: rgba(138, 92, 255, 0.12);
  }

  .play-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: color 0.15s, transform 0.1s;
  }

  .play-btn:hover {
    color: var(--color-helix-cyan, #00E5FF);
    transform: scale(1.08);
  }

  .track-thumb {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  }

  .track-thumb-placeholder {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    background:
      radial-gradient(circle at 30% 30%, rgba(0, 229, 255, 0.08), transparent 60%),
      var(--bg-elevated, #1f2937);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .track-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    overflow: hidden;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.15s;
  }

  .track-row:hover .track-title {
    color: var(--color-helix-cyan, #00E5FF);
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-album {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    min-width: 120px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-meta {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-shrink: 0;
  }

  .track-duration {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .track-source {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 6px;
    border: 1px solid var(--border-color, #1f2937);
  }

  .track-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .track-row:hover .track-actions {
    opacity: 1;
  }

  .action-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.3rem;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background 0.15s, transform 0.1s;
  }

  .action-btn:hover {
    color: var(--color-helix-violet, #8A5CFF);
    background: rgba(138, 92, 255, 0.12);
    transform: scale(1.06);
  }

  .list-btn:hover {
    color: var(--color-helix-violet, #8A5CFF);
    background: rgba(138, 92, 255, 0.12);
    transform: scale(1.06);
  }
</style>
