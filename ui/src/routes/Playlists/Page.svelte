<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { navigate } from '@app/router/navigation';
  import { t } from '@i18n';
  import { Plus, Music, Heart } from 'lucide-svelte';
  import { playlists } from '@features/playlists/stores/playlists';
  import { artistFavorites } from '@features/artist-favorites/stores/artistFavorites';
  import { countPlaylistTracks } from '@services/commands';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import type { UserPlaylist, ArtistFavorite } from '@shared/types/models';

  let showCreateDialog = false;
  let newListTitle = '';
  let playlistTrackCounts: Map<string, number> = new Map();
  let cancelled = false;

  onMount(async () => {
    await playlists.load();
    await artistFavorites.load();
    if (!cancelled) loadTrackCounts();
  });

  onDestroy(() => {
    cancelled = true;
  });

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

  async function handleCreateList() {
    if (!newListTitle.trim()) return;
    await playlists.create(newListTitle.trim());
    newListTitle = '';
    showCreateDialog = false;
    await loadTrackCounts();
  }

  function openPlaylist(id: string) {
    navigate(`/playlists/${encodeURIComponent(id)}`);
  }

  function goToArtist(artistId: string) {
    navigate(`/artist/${encodeURIComponent(artistId)}`);
  }
</script>

<div class="page-playlists">
  <div class="page-header">
    <h1 class="page-title">{$t('playlists.title')}</h1>
    <button class="create-btn" on:click={() => (showCreateDialog = true)}>
      <Plus size={18} />
      <span>{$t('playlists.create')}</span>
    </button>
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
          <button class="playlist-card" on:click={() => openPlaylist(pl.id)} type="button">
            <div class="playlist-icon">
              <Music size={32} />
            </div>
            <div class="playlist-info">
              <span class="playlist-title">{pl.title}</span>
              <span class="playlist-meta">
                {playlistTrackCounts.get(pl.id) ?? 0} {$t('playlists.tracks')}
              </span>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </section>
</div>

<!-- Create Dialog -->
{#if showCreateDialog}
  <div class="dialog-overlay" on:click={() => (showCreateDialog = false)} role="dialog" aria-modal="true">
    <div class="dialog" on:click|stopPropagation>
      <h3>{$t('playlists.create_new')}</h3>
      <input
        type="text"
        bind:value={newListTitle}
        placeholder={$t('playlists.title')}
        on:keydown={(e) => e.key === 'Enter' && handleCreateList()}
        autofocus
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
</style>
