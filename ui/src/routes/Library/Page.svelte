<script lang="ts">
  import { onMount } from 'svelte';
  import {
    watchedFolders,
    localTracks,
    isScanning,
    scanStatus,
    scanError,
    loadWatchedFolders,
    loadLocalTracks,
    scanNewFolder,
    removeFolder,
  } from '@features/library/stores/library';
  import type { WatchedFolder, LocalTrackEntry } from '@shared/types/models';

  let selectedFolder: string | null = null;

  onMount(() => {
    loadWatchedFolders();
    loadLocalTracks();
  });

  async function openFolderPicker() {
    try {
      // Use tauri-plugin-dialog to open folder picker
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Music Folder',
      });

      if (selected) {
        const path = typeof selected === 'string' ? selected : (selected as unknown as { path: string }).path;
        await scanNewFolder(path);
      }
    } catch (e) {
      console.error('Folder picker error:', e);
    }
  }

  function selectFolder(path: string | null) {
    selectedFolder = path;
    if (path) {
      loadLocalTracks(path);
    } else {
      loadLocalTracks();
    }
  }

  async function handleRemoveFolder(path: string) {
    await removeFolder(path);
    if (selectedFolder === path) {
      selectedFolder = null;
      loadLocalTracks();
    }
  }

  function formatDuration(seconds: number | undefined): string {
    if (!seconds) return '--:--';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }
</script>

<div class="page-library">
  <header class="library-header">
    <h1>Local Library</h1>
    <button class="btn-add-folder" on:click={openFolderPicker} disabled={$isScanning}>
      {#if $isScanning}
        Scanning...
      {:else}
        + Add Folder
      {/if}
    </button>
  </header>

  {#if $scanError}
    <div class="scan-error">Error: {$scanError}</div>
  {/if}

  {#if $scanStatus}
    <div class="scan-result">
      Scanned {$scanStatus.filesScanned} files:
      {$scanStatus.filesAdded} added, {$scanStatus.filesUpdated} updated,
      {$scanStatus.filesSkipped} unchanged, {$scanStatus.errors} errors
    </div>
  {/if}

  <section class="watched-folders">
    <h2>Watched Folders</h2>
    {#if $watchedFolders.length === 0}
      <p class="empty-state">No folders added yet. Click "Add Folder" to scan your music.</p>
    {:else}
      <ul class="folder-list">
        {#each $watchedFolders as folder}
          <li class:active={selectedFolder === folder.path}>
            <button class="folder-name" on:click={() => selectFolder(folder.path)}>
              📁 {folder.path}
            </button>
            <button class="btn-remove" on:click={() => handleRemoveFolder(folder.path)} title="Remove folder">
              ✕
            </button>
          </li>
        {/each}
        {#if selectedFolder}
          <li>
            <button class="folder-name show-all" on:click={() => selectFolder(null)}>
              Show all tracks
            </button>
          </li>
        {/if}
      </ul>
    {/if}
  </section>

  <section class="local-tracks">
    <h2>{selectedFolder ? 'Tracks in folder' : 'All Local Tracks'} ({$localTracks.length})</h2>
    {#if $localTracks.length === 0}
      <p class="empty-state">
        {#if $watchedFolders.length === 0}
          Add a folder to start scanning your music library.
        {:else}
          No tracks found. Try scanning a folder with audio files.
        {/if}
      </p>
    {:else}
      <table class="track-table">
        <thead>
          <tr>
            <th>Title</th>
            <th>Artist</th>
            <th>Album</th>
            <th>Duration</th>
          </tr>
        </thead>
        <tbody>
          {#each $localTracks as entry}
            <tr>
              <td>{entry.track.title}</td>
              <td>{entry.track.artist}</td>
              <td>{entry.track.album ?? '—'}</td>
              <td>{formatDuration(entry.track.duration)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>

<style>
  .page-library {
    padding: 1rem;
    color: var(--text-primary, #e0e0e0);
  }

  .library-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .library-header h1 {
    font-size: 1.5rem;
    margin: 0;
  }

  .btn-add-folder {
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent, #6366f1);
    border-radius: 6px;
    background: transparent;
    color: var(--accent, #6366f1);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-add-folder:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-add-folder:hover:not(:disabled) {
    background: var(--accent, #6366f1);
    color: white;
  }

  .scan-error {
    padding: 0.75rem;
    margin-bottom: 1rem;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .scan-result {
    padding: 0.75rem;
    margin-bottom: 1rem;
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
    border-radius: 6px;
    color: #86efac;
    font-size: 0.9rem;
  }

  .watched-folders h2,
  .local-tracks h2 {
    font-size: 1.1rem;
    margin-bottom: 0.75rem;
    color: var(--text-secondary, #a0a0a0);
  }

  .folder-list {
    list-style: none;
    padding: 0;
    margin: 0 0 1.5rem 0;
  }

  .folder-list li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    margin-bottom: 2px;
  }

  .folder-list li.active {
    background: rgba(99, 102, 241, 0.15);
  }

  .folder-name {
    flex: 1;
    background: none;
    border: none;
    color: var(--text-primary, #e0e0e0);
    cursor: pointer;
    text-align: left;
    font-size: 0.9rem;
    padding: 0;
  }

  .folder-name:hover {
    text-decoration: underline;
  }

  .folder-name.show-all {
    color: var(--accent, #6366f1);
    font-style: italic;
  }

  .btn-remove {
    background: none;
    border: none;
    color: var(--text-secondary, #a0a0a0);
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }

  .btn-remove:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .empty-state {
    color: var(--text-secondary, #a0a0a0);
    font-style: italic;
  }

  .track-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
  }

  .track-table th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border, #333);
    color: var(--text-secondary, #a0a0a0);
    font-weight: 500;
  }

  .track-table td {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid rgba(51, 51, 51, 0.5);
  }

  .track-table tr:hover {
    background: rgba(255, 255, 255, 0.03);
  }
</style>