<script lang="ts">
  import { tick } from 'svelte';
  import { get } from 'svelte/store';
  import { navigate } from '@app/router/navigation';
  import { t } from '@i18n';
  import { Play, Trash2, Edit3, ArrowLeft, Music, Folder } from 'lucide-svelte';
  import { playlists } from '@features/playlists/stores/playlists';
  import { playTrack, addToQueueAction } from '@shared/utils/actions';
  import { getPlaylistTracks, removeTrackFromPlaylist, getPlaylistThumbnails, getChildPlaylists, countPlaylistTracks } from '@services/commands';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import JellyxLogo from '@shared/components/JellyxLogo.svelte';
  import type { Track, PlaylistTrackEntry, UserPlaylist } from '@shared/types/models';

  export let id: string;

  let tracks: PlaylistTrackEntry[] = [];
  let thumbnails: string[] = [];
  let childPlaylists: UserPlaylist[] = [];
  let childTrackCounts: Map<string, number> = new Map();
  let loading = true;
  let editingTitle = false;
  let editTitleValue = '';
  let showDeleteDialog = false;
  let titleInputEl: HTMLInputElement | undefined;

  let lastLoadedId = '';

  // Reload when navigating to a different playlist (e.g. clicking a child)
  $: if (id && id !== lastLoadedId) {
    lastLoadedId = id;
    loadAll();
  }

  async function loadAll() {
    loading = true;
    tracks = [];
    thumbnails = [];
    childPlaylists = [];
    childTrackCounts = new Map();
    await Promise.all([loadTracks(), loadThumbnails(), loadChildren()]);
    loading = false;
  }

  async function loadTracks() {
    try {
      tracks = await getPlaylistTracks(id);
    } catch (e) {
      // error handled by notifications store
    }
  }

  async function loadThumbnails() {
    try {
      let thumbs = await getPlaylistThumbnails(id);
      // Parent folder playlists may have no direct tracks; aggregate the first
      // available thumbnails from child playlists so the header still shows art.
      if (thumbs.length === 0) {
        const children = await getChildPlaylists(id);
        for (const child of children) {
          if (thumbs.length >= 4) break;
          try {
            const childThumbs = await getPlaylistThumbnails(child.id);
            for (const ct of childThumbs) {
              if (!thumbs.includes(ct)) {
                thumbs.push(ct);
              }
              if (thumbs.length >= 4) break;
            }
          } catch {
            // ignore child thumbnail failures
          }
        }
      }
      thumbnails = thumbs;
    } catch {
      thumbnails = [];
    }
  }

  async function loadChildren() {
    try {
      childPlaylists = await getChildPlaylists(id);
      for (const child of childPlaylists) {
        try {
          const count = await countPlaylistTracks(child.id);
          childTrackCounts.set(child.id, count);
        } catch {
          childTrackCounts.set(child.id, 0);
        }
      }
      childTrackCounts = new Map(childTrackCounts);
    } catch {
      childPlaylists = [];
    }
  }

  function getPlaylistTitle(): string {
    const list = get(playlists);
    const found = list.find((p) => p.id === id);
    return found ? found.title : 'List';
  }

  function isFolderPlaylist(): boolean {
    const list = get(playlists);
    const found = list.find((p) => p.id === id);
    return !!found && found.kind === 'folder';
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

  $: if (editingTitle) {
    tick().then(() => titleInputEl?.focus());
  }

  async function handleDeleteList() {
    await playlists.delete(id);
    navigate('/playlists');
  }

  function openChild(childId: string) {
    navigate(`/playlists/${encodeURIComponent(childId)}`);
  }

  function formatDuration(seconds?: number): string {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  $: uniqueThumbnails = Array.from(new Set(thumbnails));
</script>

<div class="page-playlist-detail">
  <button class="back-btn" on:click={() => navigate('/playlists')} type="button">
    <ArrowLeft size={18} />
    <span>Back</span>
  </button>

  <div class="playlist-header">
    {#if uniqueThumbnails.length >= 4}
      {@const thumbs = uniqueThumbnails.slice(0, 4)}
      <div class="header-cover grid-2x2">
        {#each thumbs as thumb, i (`${thumb}-${i}`)}
          <img src={albumArtUrl(thumb)} alt="" class="header-cover-img" />
        {/each}
      </div>
    {:else if uniqueThumbnails.length === 1}
      <div class="header-cover single">
        <img src={albumArtUrl(uniqueThumbnails[0])} alt="" class="header-cover-img" />
      </div>
    {:else if uniqueThumbnails.length >= 2}
      {@const thumbs = uniqueThumbnails.slice(0, 4)}
      <div class="header-cover grid-2x2">
        {#each thumbs as thumb, i (`${thumb}-${i}`)}
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
          bind:this={titleInputEl}
        />
      {:else}
        <h1 class="playlist-title">{getPlaylistTitle()}</h1>
      {/if}
      {#if isFolderPlaylist()}
        <span class="folder-badge" title={$t('playlists.folder_badge')}>
          <Folder size={12} />
          {$t('playlists.from_folder')}
        </span>
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

  {#if childPlaylists.length > 0}
    <section class="child-section">
      <h2 class="section-title">{$t('playlists.child_playlists')}</h2>
      <div class="child-grid">
        {#each childPlaylists as child (child.id)}
          <button
            class="child-card"
            on:click={() => openChild(child.id)}
            type="button"
            aria-label="Open {child.title}"
          >
            <div class="child-icon"><Music size={18} /></div>
            <div class="child-info">
              <span class="child-title">{child.title}</span>
              <span class="child-meta">{childTrackCounts.get(child.id) ?? 0} {$t('playlists.tracks')}</span>
            </div>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if loading}
    <p class="loading">Loading...</p>
  {:else if tracks.length === 0 && childPlaylists.length === 0}
    <div class="empty-state">
      <Music size={40} />
      <p>{$t('playlists.empty_list')}</p>
    </div>
  {:else if tracks.length === 0}
    <!-- Parent has only children, no direct tracks. Show a soft hint. -->
    <p class="loading">{$t('playlists.empty_list')}</p>
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
              <JellyxLogo size={20} monochrome={true} />
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
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog-overlay" on:click={() => (showDeleteDialog = false)} on:keydown={(e) => e.key === 'Escape' && (showDeleteDialog = false)} role="dialog" aria-modal="true" tabindex="-1">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dialog" on:click|stopPropagation on:keydown={(e) => e.key === 'Escape' && (showDeleteDialog = false)}>
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

  .folder-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-accent, #6366f1);
    background: color-mix(in srgb, var(--color-accent, #6366f1) 12%, transparent);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    width: fit-content;
  }

  .child-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .child-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
  }

  .child-card {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary, #e0e0e0);
    transition: background 0.2s, border-color 0.2s;
  }

  .child-card:hover {
    background: rgba(138, 92, 255, 0.1);
    border-color: var(--color-accent, #6366f1);
  }

  .child-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(99, 102, 241, 0.15);
    color: var(--color-accent, #6366f1);
    flex-shrink: 0;
  }

  .child-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }

  .child-info .child-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .child-meta {
    font-size: 0.75rem;
    color: var(--text-secondary, #9ca3af);
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
