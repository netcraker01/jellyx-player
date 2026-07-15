<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '@i18n';
  import { navigate } from '@app/router/navigation';
  import {
    watchedFolders,
    localTracks,
    isScanning,
    scanStatus,
    scanError,
    tracksByFolder,
    loadWatchedFolders,
    loadLocalTracks,
    scanNewFolder,
    removeFolder,
  } from '@features/library/stores/library';
  import { Folder } from 'lucide-svelte';

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

  async function handleRemoveFolder(path: string) {
    await removeFolder(path);
  }

  function openFolder(path: string) {
    navigate(`/library/folder/${encodeURIComponent(path)}`);
  }

  $: folderCount = (folderPath: string) => $tracksByFolder.get(folderPath)?.length ?? 0;

  /** Extract the final folder name from a path. Falls back to the full
   *  path if no separator is found (e.g. bare folder name on some platforms). */
  function folderName(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/').filter(Boolean);
    return parts.length > 0 ? parts[parts.length - 1] : path;
  }
</script>

<div class="page-library">
  <header class="library-header">
    <h1>{$t('library.local_files')}</h1>
    <button class="btn-add-folder" on:click={openFolderPicker} disabled={$isScanning}>
      {#if $isScanning}
        {$t('library.scanning')}
      {:else}
        + {$t('library.add_folder')}
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
    <h2>{$t('library.watched_folders')}</h2>
    {#if $watchedFolders.length === 0}
      <p class="empty-state">{$t('library.empty_folders')}</p>
    {:else}
      <div class="folder-cards">
        {#each $watchedFolders as folder (folder.path)}
          <div
            class="folder-card"
            on:click={() => openFolder(folder.path)}
            on:keydown={(e) => e.key === 'Enter' && openFolder(folder.path)}
            role="button"
            tabindex="0"
            aria-label="{$t('library.open_folder')} {folderName(folder.path)}"
          >
            <div class="folder-icon">
              <Folder size={28} />
            </div>
            <div class="folder-info">
              <span class="folder-path" title={folder.path}>{folderName(folder.path)}</span>
              <span class="folder-count">{folderCount(folder.path)} {$t('library.folder_tracks')}</span>
            </div>
            <button
              class="btn-remove"
              on:click|stopPropagation={() => handleRemoveFolder(folder.path)}
              title="Remove folder"
              aria-label="Remove folder {folderName(folder.path)}"
            >
              ✕
            </button>
          </div>
        {/each}
      </div>
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

  .watched-folders h2 {
    font-size: 1.1rem;
    margin-bottom: 0.75rem;
    color: var(--text-secondary, #a0a0a0);
  }

  .empty-state {
    color: var(--text-secondary, #a0a0a0);
    font-style: italic;
  }

  .folder-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 1rem;
  }

  .folder-card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 12px;
    cursor: pointer;
    text-align: left;
    transition: background 0.2s, border-color 0.2s, transform 0.15s;
  }

  .folder-card:hover {
    background: rgba(138, 92, 255, 0.1);
    border-color: var(--color-accent, #6366f1);
    transform: translateY(-2px);
  }

  .folder-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 10px;
    background: rgba(99, 102, 241, 0.15);
    color: var(--color-accent, #6366f1);
    flex-shrink: 0;
  }

  .folder-info {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
    flex: 1;
  }

  .folder-path {
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text-primary, #e0e0e0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .folder-count {
    font-size: 0.8rem;
    color: var(--text-secondary, #a0a0a0);
  }

  .btn-remove {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    background: none;
    border: none;
    color: var(--text-secondary, #a0a0a0);
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
    opacity: 0;
    transition: opacity 0.2s, color 0.2s, background 0.2s;
  }

  .folder-card:hover .btn-remove {
    opacity: 1;
  }

  .btn-remove:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }
</style>