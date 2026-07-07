<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '@i18n';
  import { navigate } from '@app/router/navigation';
  import { getLocalTracks } from '@services/commands';
  import TrackRow from '@shared/components/TrackRow.svelte';
  import { ArrowLeft, Folder } from 'lucide-svelte';
  import type { LocalTrackEntry } from '@shared/types/models';

  // folderPath is the URL-decoded absolute path passed by the router.
  export let folderPath: string;

  let entries: LocalTrackEntry[] = [];
  let loading = true;
  let error: string | null = null;

  onMount(async () => {
    loading = true;
    error = null;
    try {
      entries = await getLocalTracks(folderPath);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  function goBack() {
    navigate('/library');
  }
</script>

<div class="page-folder-detail">
  <header class="folder-header">
    <button class="back-btn" on:click={goBack} aria-label={$t('common.back')}>
      <ArrowLeft size={20} />
    </button>
    <div class="folder-title-wrap">
      <Folder size={22} class="folder-icon" />
      <h1 class="folder-title">{folderPath}</h1>
    </div>
    <span class="track-count">{entries.length} {$t('library.folder_tracks')}</span>
  </header>

  {#if loading}
    <p class="status">{$t('common.loading')}</p>
  {:else if error}
    <div class="error-state">{$t('common.error')}: {error}</div>
  {:else if entries.length === 0}
    <p class="empty-state">{$t('library.empty_tracks')}</p>
  {:else}
    <div class="track-list">
      {#each entries as entry (entry.filePath)}
        <TrackRow track={entry.track} showSource={false} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .page-folder-detail {
    padding: 1rem;
    color: var(--text-primary, #e0e0e0);
  }

  .folder-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  .back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.4rem;
    border-radius: 6px;
    transition: color 0.15s, background 0.15s;
  }

  .back-btn:hover {
    color: var(--text-primary, #e0e0e0);
    background: var(--bg-elevated, #1f2937);
  }

  .folder-title-wrap {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
  }

  :global(.folder-icon) {
    color: var(--color-accent, #6366f1);
    flex-shrink: 0;
  }

  .folder-title {
    font-size: 1.25rem;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-count {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    flex-shrink: 0;
  }

  .status,
  .empty-state {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .error-state {
    padding: 0.75rem;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #fca5a5;
    font-size: 0.9rem;
  }

  .track-list {
    display: flex;
    flex-direction: column;
  }
</style>