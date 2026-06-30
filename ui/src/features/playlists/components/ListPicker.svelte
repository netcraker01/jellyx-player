<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte';
  import { Search, Plus, Check } from 'lucide-svelte';
  import { playlists, recentPlaylists } from '@features/playlists/stores/playlists';
  import { searchUserPlaylists, getPlaylistTracks } from '@services/commands';
  import type { Track, UserPlaylist } from '@shared/types/models';

  export let track: Track;
  export let visible: boolean = false;
  export let anchorX: number = 0;
  export let anchorY: number = 0;

  const dispatch = createEventDispatcher();

  let searchQuery = '';
  let searchResults: UserPlaylist[] = [];
  let trackInListIds: Set<string> = new Set();
  let creating = false;
  let newListTitle = '';
  let searchInputEl: HTMLInputElement | undefined;
  let newListInputEl: HTMLInputElement | undefined;

  onMount(() => {
    playlists.load();
    checkTrackInLists();
  });

  async function checkTrackInLists() {
    const all = await new Promise<UserPlaylist[]>((resolve) => {
      const unsub = playlists.subscribe((p) => {
        resolve(p);
        unsub();
      });
    });
    for (const pl of all.slice(0, 10)) {
      try {
        const tracks = await getPlaylistTracks(pl.id);
        const found = tracks.some((t) => t.track.id === track.id || t.track.sourceId === track.sourceId);
        if (found) {
          trackInListIds.add(pl.id);
        }
      } catch {
        // ignore
      }
    }
    trackInListIds = new Set(trackInListIds);
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      searchResults = [];
      return;
    }
    searchResults = await searchUserPlaylists(searchQuery.trim());
  }

  function clearSearch() {
    searchQuery = '';
    searchResults = [];
  }

  async function selectList(playlistId: string) {
    await playlists.addTrack(playlistId, track);
    dispatch('close');
  }

  async function createAndAdd() {
    if (!newListTitle.trim()) return;
    const pl = await playlists.create(newListTitle.trim());
    if (pl && typeof pl === 'object' && 'id' in pl) {
      await playlists.addTrack(pl.id, track);
      dispatch('close');
    }
  }

  function handleBackdropClick() {
    dispatch('close');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      dispatch('close');
    }
  }

  $: displayLists = searchQuery.trim() ? searchResults : $recentPlaylists;

  $: if (visible) {
    tick().then(() => searchInputEl?.focus());
  }

  $: if (creating) {
    tick().then(() => newListInputEl?.focus());
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="picker-backdrop" on:click={handleBackdropClick} on:keydown={handleKeydown} role="presentation">
    <div
      class="picker-popup"
      on:click|stopPropagation
      on:keydown={handleKeydown}
      style="left: {Math.min(anchorX, window.innerWidth - 260)}px; top: {Math.min(anchorY, window.innerHeight - 300)}px;"
      role="dialog"
      aria-label="Add to list"
      tabindex="-1"
    >
      <div class="picker-header">
        <div class="search-row">
          <Search size={14} />
          <input
            type="text"
            bind:value={searchQuery}
            on:input={handleSearch}
            placeholder="Search lists..."
            bind:this={searchInputEl}
          />
        </div>
      </div>

      <div class="picker-body">
        {#if displayLists.length === 0 && !searchQuery.trim()}
          <p class="empty-text">No lists yet.</p>
        {:else if displayLists.length === 0 && searchQuery.trim()}
          <p class="empty-text">No results.</p>
        {:else}
          <div class="list-options">
            {#each displayLists as pl (pl.id)}
              <button class="list-option" on:click={() => selectList(pl.id)} type="button">
                <span class="list-title">{pl.title}</span>
                {#if trackInListIds.has(pl.id)}
                  <Check size={14} class="check-icon" />
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="picker-footer">
        {#if creating}
          <div class="create-row">
            <input
              type="text"
              bind:value={newListTitle}
              on:keydown={(e) => e.key === 'Enter' && createAndAdd()}
              placeholder="New list name"
              bind:this={newListInputEl}
            />
            <button class="create-btn" on:click={createAndAdd} type="button">Add</button>
          </div>
        {:else}
          <button class="create-new-btn" on:click={() => (creating = true)} type="button">
            <Plus size={14} />
            <span>Create new list</span>
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
  }

  .picker-popup {
    position: absolute;
    width: 240px;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface, #111827);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 10px;
    box-shadow:
      0 10px 40px rgba(0, 0, 0, 0.45),
      0 0 0 1px rgba(0, 0, 0, 0.2);
    overflow: hidden;
  }

  .picker-header {
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
  }

  .search-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 6px;
    color: var(--text-secondary, #9ca3af);
  }

  .search-row input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text-primary, #e0e0e0);
    font-size: 0.85rem;
    outline: none;
  }

  .picker-body {
    max-height: 180px;
    overflow-y: auto;
    padding: 0.25rem 0;
  }

  .empty-text {
    padding: 1rem;
    text-align: center;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    margin: 0;
  }

  .list-options {
    display: flex;
    flex-direction: column;
  }

  .list-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    color: var(--text-primary, #e0e0e0);
    font-size: 0.85rem;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
  }

  .list-option:hover {
    background: var(--bg-elevated, #1f2937);
  }

  .list-title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .picker-footer {
    padding: 0.5rem;
    border-top: 1px solid var(--border-color, #1f2937);
  }

  .create-new-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    padding: 0.4rem 0.5rem;
    background: none;
    border: none;
    border-radius: 6px;
    color: var(--color-accent, #6366f1);
    font-size: 0.85rem;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
  }

  .create-new-btn:hover {
    background: rgba(99, 102, 241, 0.1);
  }

  .create-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .create-row input {
    flex: 1;
    padding: 0.35rem 0.5rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 6px;
    color: var(--text-primary, #e0e0e0);
    font-size: 0.85rem;
  }

  .create-btn {
    padding: 0.35rem 0.75rem;
    border: none;
    border-radius: 6px;
    background: var(--color-accent, #6366f1);
    color: white;
    font-size: 0.85rem;
    cursor: pointer;
  }

  :global(.check-icon) {
    color: #22c55e;
  }
</style>
