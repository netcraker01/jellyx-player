<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { navigate } from '@app/router/navigation';
  import { t } from '@i18n';
  import { Plus, Music, Heart, Trash2, Play, Link } from 'lucide-svelte';
  import { playlists } from '@features/playlists/stores/playlists';
  import { artistFavorites } from '@features/artist-favorites/stores/artistFavorites';
  import { playTrack, addToQueueAction } from '@shared/utils/actions';
  import { countPlaylistTracks, getPlaylistThumbnails, getPlaylistTracks, resolvePlaylist, addTracksToPlaylist } from '@services/commands';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import { notifications } from '@shared/stores/notifications';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import type { UserPlaylist, ArtistFavorite } from '@shared/types/models';

  let showCreateDialog = false;
  let newListTitle = '';
  let deleteTargetId: string | null = null;
  let deleteTargetTitle = '';
  let playlistTrackCounts: Map<string, number> = new Map();
  let playlistThumbnails: Map<string, string[]> = new Map();
  let cancelled = false;

  // Import-from-URL state
  let showImportDialog = false;
  let importUrl = '';
  let importing = false;

  let createInputEl: HTMLInputElement | undefined;
  let importInputEl: HTMLInputElement | undefined;

  onMount(async () => {
    await playlists.load();
    await artistFavorites.load();
    if (!cancelled) {
      loadTrackCounts();
      loadThumbnails();
    }
  });

  onDestroy(() => {
    cancelled = true;
  });

  $: if (showCreateDialog) {
    tick().then(() => createInputEl?.focus());
  }

  $: if (showImportDialog) {
    tick().then(() => importInputEl?.focus());
  }

  async function loadTrackCounts() {
    const all = get(playlists);
    for (const pl of all) {
      if (cancelled) return;
      try {
        const count = await countPlaylistTracks(pl.id);
        playlistTrackCounts.set(pl.id, count);
      } catch {
        playlistTrackCounts.set(pl.id, 0);
      }
    }
    playlistTrackCounts = new Map(playlistTrackCounts);
  }

  async function loadThumbnails() {
    const all = get(playlists);
    for (const pl of all) {
      if (cancelled) return;
      try {
        const thumbs = await getPlaylistThumbnails(pl.id);
        playlistThumbnails.set(pl.id, thumbs);
      } catch {
        playlistThumbnails.set(pl.id, []);
      }
    }
    playlistThumbnails = new Map(playlistThumbnails);
  }

  async function handleCreateList() {
    if (!newListTitle.trim()) return;
    await playlists.create(newListTitle.trim());
    newListTitle = '';
    showCreateDialog = false;
    await loadTrackCounts();
    await loadThumbnails();
  }

  function openPlaylist(id: string) {
    navigate(`/playlists/${encodeURIComponent(id)}`);
  }

  function goToArtist(artistId: string) {
    navigate(`/artist/${encodeURIComponent(artistId)}`);
  }

  function confirmDeletePlaylist(id: string, title: string) {
    deleteTargetId = id;
    deleteTargetTitle = title;
  }

  async function handlePlayPlaylist(id: string) {
    try {
      const entries = await getPlaylistTracks(id);
      if (entries.length === 0) return;
      await playTrack(entries[0].track);
      for (let i = 1; i < entries.length; i++) {
        await addToQueueAction(entries[i].track);
      }
    } catch {
      // ignore — user will see no playback
    }
  }

  async function handleDeletePlaylist() {
    if (!deleteTargetId) return;
    await playlists.delete(deleteTargetId);
    deleteTargetId = null;
    deleteTargetTitle = '';
  }

  async function handleImportPlaylist() {
    const url = importUrl.trim();
    if (!url) return;

    importing = true;
    try {
      // Detect source from URL
      const source = url.includes('soundcloud.com') ? 'SoundCloud' : 'YouTube';

      const resolved = await resolvePlaylist(source, url);
      if (!resolved.tracks || resolved.tracks.length === 0) {
        notifications.push({ type: 'error', title: 'Import', message: 'No tracks found in playlist', dismissible: true });
        return;
      }

      // Create a local playlist with the resolved title
      const created = await playlists.create(resolved.title);
      if (!created) return;

      // Batch-add all tracks in a single IPC call
      const added = await addTracksToPlaylist(created.id, resolved.tracks);

      notifications.push({
        type: 'success',
        title: 'Import',
        message: `Imported ${added} tracks`,
        dismissible: true,
      });

      // Reload counts and thumbnails
      await loadTrackCounts();
      await loadThumbnails();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      notifications.push({ type: 'error', title: 'Import Failed', message: msg, dismissible: true });
    } finally {
      importing = false;
      importUrl = '';
      showImportDialog = false;
    }
  }
</script>

<div class="page-playlists">
  <div class="page-header">
    <h1 class="page-title">{$t('playlists.title')}</h1>
    <div class="header-actions">
      <button class="import-btn" on:click={() => (showImportDialog = true)}>
        <Link size={18} />
        <span>{$t('playlists.import_url')}</span>
      </button>
      <button class="create-btn" on:click={() => (showCreateDialog = true)}>
        <Plus size={18} />
        <span>{$t('playlists.create')}</span>
      </button>
    </div>
  </div>

  <!-- Artist Favorites Section -->
  <section class="section">
    <h2 class="section-title">
      <Heart size={16} />
      {$t('playlists.artist_favorites')}
    </h2>
    {#if $artistFavorites.length === 0}
      <p class="empty-text">No favorite artists yet.</p>
    {:else}
      <div class="artist-favorites-grid">
        {#each $artistFavorites as artist (artist.artistId)}
          <button class="artist-card" on:click={() => goToArtist(artist.artistId)} type="button">
            {#if albumArtUrl(artist.thumbnail)}
              <img src={albumArtUrl(artist.thumbnail)} alt={artist.artistName} class="artist-thumb" />
            {:else}
              <div class="artist-thumb-placeholder">
                <HelixLogo size={32} monochrome={true} />
              </div>
            {/if}
            <span class="artist-name">{artist.artistName}</span>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Playlists Section -->
  <section class="section">
    <h2 class="section-title">
      <Music size={16} />
      {$t('playlists.title')}
    </h2>
    {#if $playlists.length === 0}
      <div class="empty-state">
        <p>{$t('playlists.empty')}</p>
      </div>
    {:else}
      <div class="playlists-grid">
        {#each $playlists as pl (pl.id)}
          <div class="playlist-card" on:click={() => openPlaylist(pl.id)} on:keydown={(e) => e.key === 'Enter' && openPlaylist(pl.id)} role="button" tabindex="0">
            {#if (playlistThumbnails.get(pl.id) ?? []).length >= 4}
              {@const thumbs = (playlistThumbnails.get(pl.id) ?? []).slice(0, 4)}
              <div class="playlist-cover grid-2x2">
                {#each thumbs as thumb (thumb)}
                  <img src={albumArtUrl(thumb)} alt="" class="cover-img" />
                {/each}
              </div>
            {:else if (playlistThumbnails.get(pl.id) ?? []).length === 1}
              {@const thumb = (playlistThumbnails.get(pl.id) ?? [])[0]}
              <div class="playlist-cover single">
                <img src={albumArtUrl(thumb)} alt="" class="cover-img" />
              </div>
            {:else if (playlistThumbnails.get(pl.id) ?? []).length >= 2}
              {@const thumbs = (playlistThumbnails.get(pl.id) ?? []).slice(0, 4)}
              <!-- Pad to 4 for grid layout, show only the available ones -->
              <div class="playlist-cover grid-2x2">
                {#each thumbs as thumb (thumb)}
                  <img src={albumArtUrl(thumb)} alt="" class="cover-img" />
                {/each}
                {#each { length: 4 - thumbs.length } as _}
                  <div class="cover-placeholder"><Music size={16} /></div>
                {/each}
              </div>
            {:else}
              <div class="playlist-icon">
                <Music size={32} />
              </div>
            {/if}
            <div class="playlist-info">
              <span class="playlist-title">{pl.title}</span>
              <span class="playlist-meta">
                {playlistTrackCounts.get(pl.id) ?? 0} {$t('playlists.tracks')}
              </span>
            </div>
            <button class="card-play-btn" on:click|stopPropagation={() => handlePlayPlaylist(pl.id)} title="Play" type="button">
              <Play size={14} fill="currentColor" />
            </button>
            <button class="card-delete-btn" on:click|stopPropagation={() => confirmDeletePlaylist(pl.id, pl.title)} title="Delete" type="button">
              <Trash2 size={14} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<!-- Create Dialog -->
{#if showCreateDialog}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-overlay" on:click={() => (showCreateDialog = false)} on:keydown={(e) => e.key === 'Escape' && (showCreateDialog = false)} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" on:click|stopPropagation on:keydown={(e) => e.key === 'Escape' && (showCreateDialog = false)}>
      <h3>{$t('playlists.create_new')}</h3>
      <input
        type="text"
        bind:value={newListTitle}
        placeholder={$t('playlists.title')}
        on:keydown={(e) => e.key === 'Enter' && handleCreateList()}
        bind:this={createInputEl}
      />
      <div class="dialog-actions">
        <button class="btn-secondary" on:click={() => (showCreateDialog = false)}>
          {$t('common.cancel')}
        </button>
        <button class="btn-primary" on:click={handleCreateList}>
          {$t('common.save')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Import from URL Dialog -->
{#if showImportDialog}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-overlay" on:click={() => !importing && (showImportDialog = false)} on:keydown={(e) => e.key === 'Escape' && !importing && (showImportDialog = false)} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" on:click|stopPropagation on:keydown={(e) => e.key === 'Escape' && !importing && (showImportDialog = false)}>
      <h3>{$t('playlists.import_url')}</h3>
      <p class="dialog-text">{$t('playlists.import_url_desc')}</p>
      <input
        type="text"
        bind:value={importUrl}
        placeholder="https://www.youtube.com/playlist?list=..."
        on:keydown={(e) => e.key === 'Enter' && !importing && handleImportPlaylist()}
        disabled={importing}
        bind:this={importInputEl}
      />
      {#if importing}
        <p class="dialog-text importing-hint">{$t('playlists.importing')}...</p>
      {/if}
      <div class="dialog-actions">
        <button class="btn-secondary" on:click={() => (showImportDialog = false)} disabled={importing} type="button">
          {$t('common.cancel')}
        </button>
        <button class="btn-primary" on:click={handleImportPlaylist} disabled={importing || !importUrl.trim()} type="button">
          {$t('playlists.import')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Dialog -->
{#if deleteTargetId}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-overlay" on:click={() => (deleteTargetId = null)} on:keydown={(e) => e.key === 'Escape' && (deleteTargetId = null)} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" on:click|stopPropagation on:keydown={(e) => e.key === 'Escape' && (deleteTargetId = null)}>
      <h3>Delete playlist?</h3>
      <p class="dialog-text">This will permanently delete <strong>{deleteTargetTitle}</strong> and all its tracks.</p>
      <div class="dialog-actions">
        <button class="btn-secondary" on:click={() => (deleteTargetId = null)} type="button">Cancel</button>
        <button class="btn-danger" on:click={handleDeletePlaylist} type="button">Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .page-playlists {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .page-title {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary, #e0e0e0);
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .import-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 8px;
    background: var(--bg-elevated, #1f2937);
    color: var(--text-primary, #e0e0e0);
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s;
  }

  .import-btn:hover {
    background: var(--bg-hover, #374151);
    border-color: var(--color-accent, #6366f1);
  }

  .create-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: var(--color-accent, #6366f1);
    color: white;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  .create-btn:hover {
    background: var(--color-accent-hover, #4f52d9);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
  }

  .empty-text {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .empty-state {
    padding: 3rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
    background: var(--bg-elevated, #1f2937);
    border-radius: 12px;
  }

  .artist-favorites-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 1rem;
  }

  .artist-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-elevated, #1f2937);
    border: none;
    border-radius: 12px;
    cursor: pointer;
    transition: background 0.2s;
    text-align: center;
  }

  .artist-card:hover {
    background: rgba(138, 92, 255, 0.1);
  }

  .artist-thumb {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    object-fit: cover;
  }

  .artist-thumb-placeholder {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-surface, #111827);
  }

  .artist-name {
    font-size: 0.85rem;
    color: var(--text-primary, #e0e0e0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .playlists-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 1rem;
  }

  .playlist-card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    background: var(--bg-elevated, #1f2937);
    border: none;
    border-radius: 12px;
    cursor: pointer;
    transition: background 0.2s;
    text-align: left;
  }

  .playlist-card:hover {
    background: rgba(138, 92, 255, 0.1);
  }

  .playlist-icon {
    width: 56px;
    height: 56px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(99, 102, 241, 0.15);
    color: var(--color-accent, #6366f1);
    flex-shrink: 0;
  }

  .playlist-cover {
    width: 56px;
    height: 56px;
    border-radius: 10px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .playlist-cover.single {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .playlist-cover.single .cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .playlist-cover.grid-2x2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 2px;
  }

  .playlist-cover.grid-2x2 .cover-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cover-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(99, 102, 241, 0.15);
    color: var(--color-accent, #6366f1);
  }

  .playlist-info {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .playlist-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .playlist-meta {
    font-size: 0.8rem;
    color: var(--text-secondary, #9ca3af);
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

  .dialog input {
    padding: 0.6rem 0.75rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 8px;
    color: var(--text-primary, #e0e0e0);
    font-size: 0.95rem;
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

  .btn-primary {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: var(--color-accent, #6366f1);
    color: white;
    cursor: pointer;
  }

  .card-play-btn {
    position: absolute;
    top: 0.5rem;
    right: 2.5rem;
    background: none;
    border: none;
    color: var(--color-accent, #6366f1);
    cursor: pointer;
    padding: 0.3rem;
    border-radius: 6px;
    opacity: 0;
    transition: opacity 0.2s, color 0.2s, background 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .playlist-card:hover .card-play-btn {
    opacity: 1;
  }

  .card-play-btn:hover {
    color: var(--color-accent-hover, #4f52d9);
    background: rgba(99, 102, 241, 0.15);
  }

  .card-delete-btn {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.3rem;
    border-radius: 6px;
    opacity: 0;
    transition: opacity 0.2s, color 0.2s, background 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .playlist-card:hover .card-delete-btn {
    opacity: 1;
  }

  .card-delete-btn:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }

  .dialog-text {
    margin: 0;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .importing-hint {
    color: var(--color-accent, #6366f1);
    font-style: italic;
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
