<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { navigate } from '@app/router/navigation';
  import { t } from '@i18n';
  import { Play, Trash2, Edit3, ArrowLeft, Music } from 'lucide-svelte';
  import { playlists } from '@features/playlists/stores/playlists';
  import { playTrack, addToQueueAction } from '@shared/utils/actions';
  import { getPlaylistTracks, removeTrackFromPlaylist, getPlaylistThumbnails } from '@services/commands';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import type { Track, PlaylistTrackEntry } from '@shared/types/models';

  export let id: string;

  let tracks: PlaylistTrackEntry[] = [];
  let thumbnails: string[] = [];
  let loading = true;
  let editingTitle = false;
  let editTitleValue = '';
  let showDeleteDialog = false;

  onMount(() => {
    loadTracks();
    loadThumbnails();
  });

  async function loadTracks() {
    try {
      tracks = await getPlaylistTracks(id);
    } catch (e) {
      // error handled by notifications store
    } finally {
      loading = false;
    }
  }

  async function loadThumbnails() {
    try {
      thumbnails = await getPlaylistThumbnails(id);
    } catch {
      thumbnails = [];
    }
  }

  function getPlaylistTitle(): string {
    const list = get(playlists);
    const found = list.find((p) => p.id === id);
    return found ? found.title : 'List';
  }

  async function handleRemoveTrack(position: number) {
    await removeTrackFromPlaylist(id, position);
    await loadTracks();
  }

  async function handlePlayAll() {
    if (tracks.length === 0) return;
    // Play first track, then add the rest to the queue
    await playTrack(tracks[0].track);
    for (let i = 1; i < tracks.length; i++) {
      await addToQueueAction(tracks[i].track);
    }
  }

  function startRename() {
    editTitleValue = getPlaylistTitle();
    editingTitle = true;
  }

  async function finishRename() {
    if (editTitleValue.trim()) {
      await playlists.rename(id, editTitleValue.trim());
    }
    editingTitle = false;
  }

  async function handleDeleteList() {
    await playlists.delete(id);
    navigate('/playlists');
  }

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }
</script>

<div class="page-playlist-detail">
  <button class="back-btn" on:click={() => navigate('/playlists')} type="button">
    <ArrowLeft size={18} />
    <span>Back</span>
  </button>

  <div class="playlist-header">
    {#if thumbnails.length >= 4}
      {@const thumbs = thumbnails.slice(0, 4)}
      <div class="header-cover grid-2x2">
        {#each thumbs as thumb (thumb)}
          <img src={albumArtUrl(thumb)} alt="" class="header-cover-img" />
        {/each}
      </div>
    {:else if thumbnails.length === 1}
      <div class="header-cover single">
        <img src={albumArtUrl(thumbnails[0])} alt="" class="header-cover-img" />
      </div>
    {:else if thumbnails.length >= 2}
      {@const thumbs = thumbnails.slice(0, 4)}
      <div class="header-cover grid-2x2">
        {#each thumbs as thumb (thumb)}
          <img src={albumArtUrl(thumb)} alt="" class="header-cover-img" />
        {/each}
        {#each { length: 4 - thumbs.length } as _}
          <div class="header-cover-placeholder"><Music size={16} /></div>
        {/each}
      </div>
    {:else}
      <div class="header-cover-placeholder">
        <Music size={48} />
      </div>
    {/if}
    <div class="header-text">
      {#if editingTitle}
        <input
          class="title-input"
          bind:value={editTitleValue}
          on:blur={finishRename}
          on:keydown={(e) => e.key === 'Enter' && finishRename()}
          autofocus
        />
      {:else}
        <h1 class="playlist-title">{getPlaylistTitle()}</h1>
      {/if}
      <div class="header-actions">
        <button class="icon-btn" on:click={startRename} title="Rename" type="button">
          <Edit3 size={16} />
        </button>
        <button class="icon-btn play-btn" on:click={handlePlayAll} disabled={tracks.length === 0} title="Play all" type="button">
          <Play size={16} />
          <span>Play all</span>
        </button>
        <button class="icon-btn delete-btn" on:click={() => (showDeleteDialog = true)} title="Delete list" type="button">
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  </div>

  {#if loading}
    <p class="loading">Loading...</p>
  {:else if tracks.length === 0}
    <div class="empty-state">
      <Music size={40} />
      <p>{$t('playlists.empty_list')}</p>
    </div>
  {:else}
    <div class="track-list">
      {#each tracks as entry (entry.position)}
        <div class="track-row">
          <button class="play-btn" on:click={() => playTrack(entry.track)} type="button">
            <Play size={14} />
          </button>
          {#if albumArtUrl(entry.track.thumbnail)}
            <img class="track-thumb" src={albumArtUrl(entry.track.thumbnail)} alt={entry.track.title} />
          {:else}
            <div class="track-thumb-placeholder">
              <HelixLogo size={20} monochrome={true} />
            </div>
          {/if}
          <div class="track-info">
            <span class="track-title">{entry.track.title}</span>
            <span class="track-artist">{entry.track.artist}</span>
          </div>
          <span class="track-duration">{formatDuration(entry.track.duration)}</span>
          <button class="remove-btn" on:click={() => handleRemoveTrack(entry.position)} title="Remove" type="button">
            <Trash2 size={14} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if showDeleteDialog}
    <div class="dialog-overlay" on:click={() => (showDeleteDialog = false)} role="dialog" aria-modal="true">
      <div class="dialog" on:click|stopPropagation>
        <h3>Delete playlist?</h3>
        <p class="dialog-text">This will permanently delete <strong>{getPlaylistTitle()}</strong> and all its tracks.</p>
        <div class="dialog-actions">
          <button class="btn-secondary" on:click={() => (showDeleteDialog = false)} type="button">Cancel</button>
          <button class="btn-danger" on:click={handleDeleteList} type="button">Delete</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .page-playlist-detail {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .back-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    font-size: 0.9rem;
    width: fit-content;
  }

  .back-btn:hover {
    color: var(--text-primary, #e0e0e0);
  }

  .playlist-header {
    display: flex;
    align-items: center;
    gap: 1.25rem;
    flex-wrap: wrap;
  }

  .header-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .header-cover {
    width: 80px;
    height: 80px;
    border-radius: 12px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .header-cover.single {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .header-cover.single .header-cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .header-cover.grid-2x2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 2px;
  }

  .header-cover.grid-2x2 .header-cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .header-cover-placeholder {
    width: 80px;
    height: 80px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(99, 102, 241, 0.15);
    color: var(--color-accent, #6366f1);
    flex-shrink: 0;
  }

  .header-cover .header-cover-placeholder {
    width: auto;
    height: auto;
    border-radius: 0;
  }

  .playlist-title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary, #e0e0e0);
  }

  .title-input {
    font-size: 1.5rem;
    font-weight: 700;
    padding: 0.25rem 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 8px;
    color: var(--text-primary, #e0e0e0);
    width: 300px;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.75rem;
    background: none;
    border: none;
    border-radius: 8px;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }

  .icon-btn:hover {
    background: var(--bg-elevated, #1f2937);
    color: var(--text-primary, #e0e0e0);
  }

  .play-btn {
    background: var(--color-accent, #6366f1);
    color: white;
  }

  .play-btn:hover {
    background: var(--color-accent-hover, #4f52d9);
  }

  .loading {
    color: var(--text-secondary, #9ca3af);
    text-align: center;
    padding: 2rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 4rem;
    color: var(--text-secondary, #9ca3af);
  }

  .track-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .track-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-radius: 8px;
    transition: background 0.15s;
  }

  .track-row:hover {
    background: var(--bg-elevated, #1f2937);
  }

  .play-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .track-thumb {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .track-thumb-placeholder {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated, #1f2937);
  }

  .track-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
  }

  .track-duration {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .remove-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.2s, color 0.2s;
  }

  .track-row:hover .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    color: #ef4444;
  }

  .delete-btn {
    color: #ef4444;
  }

  .delete-btn:hover {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }

  .dialog-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    z-index: 100;
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.5rem;
    background: var(--bg-surface, #111827);
    border-radius: 12px;
    border: 1px solid var(--border-color, #1f2937);
    min-width: 320px;
  }

  .dialog h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary, #e0e0e0);
  }

  .dialog-text {
    margin: 0;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-secondary {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
  }

  .btn-danger {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: #ef4444;
    color: white;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-danger:hover {
    background: #dc2626;
  }
</style>
