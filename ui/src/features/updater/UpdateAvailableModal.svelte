<script lang="ts">
  /**
   * UpdateAvailableModal — global modal shown when a newer Jellyx Player version is available.
   *
   * Phase 1: "Update now" opens the release page in the system default browser
   * via the backend `open_release_page` command. No in-place installation.
   *
   * Modal state is driven by `updaterStore` (`modalOpen`, `info`). Actions call
   * the store's `updateNow`, `remindLater`, and `skipVersion` helpers.
   *
   * The dialog markup follows the existing Playlists page dialog pattern
   * (overlay + dialog + dialog-actions) so styling stays consistent.
   */
  import { updaterStore, updateNow, remindLater, skipVersion, dismissModal } from './updater.store';
  import { Download, Clock, X, ExternalLink } from 'lucide-svelte';

  $: info = $updaterStore.info;
  $: open = $updaterStore.modalOpen && info != null;
  $: checking = $updaterStore.checking;
  $: error = $updaterStore.error;

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') dismissModal();
  }
</script>

{#if open && info}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="dialog-overlay"
    on:click={dismissModal}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="update-modal-title"
    tabindex="-1"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" on:click|stopPropagation on:keydown={handleKeydown}>
      <header class="dialog-header">
        <div class="title-row">
          <Download size={20} />
          <h3 id="update-modal-title">A new version of Jellyx Player is available</h3>
        </div>
        <button class="close-btn" on:click={dismissModal} title="Close" type="button" aria-label="Close">
          <X size={16} />
        </button>
      </header>

      <div class="version-row">
        <div class="version-cell">
          <span class="version-label">Current</span>
          <span class="version-value">{info.currentVersion}</span>
        </div>
        <span class="version-arrow">→</span>
        <div class="version-cell latest">
          <span class="version-label">Latest</span>
          <span class="version-value">{info.latestVersion}</span>
        </div>
      </div>

      {#if info.channel && info.channel !== 'unknown'}
        <p class="channel-line">Install channel: <span class="channel-badge">{info.channel}</span></p>
      {/if}

      {#if info.body}
        <div class="release-notes">
          <h4>Release notes</h4>
          <pre class="notes-body">{info.body}</pre>
        </div>
      {/if}

      {#if error}
        <p class="error-line" role="alert">{error}</p>
      {/if}

      <div class="dialog-actions">
        <button class="btn-secondary" on:click={() => skipVersion()} type="button" disabled={checking}>
          Skip this version
        </button>
        <button class="btn-secondary" on:click={() => remindLater(24)} type="button" disabled={checking}>
          <Clock size={14} />
          Remind me later
        </button>
        <button class="btn-primary" on:click={() => updateNow()} type="button" disabled={checking}>
          <ExternalLink size={14} />
          Update now
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    z-index: 200;
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.5rem;
    background: var(--bg-surface, #111827);
    border-radius: 12px;
    border: 1px solid var(--border-color, #1f2937);
    min-width: 380px;
    max-width: 560px;
    max-height: 80vh;
    overflow-y: auto;
  }

  .dialog-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary, #e0e0e0);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    color: var(--text-primary, #e0e0e0);
    background: var(--bg-hover, #374151);
  }

  .version-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 8px;
  }

  .version-cell {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .version-cell.latest .version-value {
    color: var(--color-accent, #6366f1);
    font-weight: 700;
  }

  .version-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary, #9ca3af);
  }

  .version-value {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
  }

  .version-arrow {
    color: var(--text-secondary, #9ca3af);
    font-size: 1.2rem;
  }

  .channel-line {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-secondary, #9ca3af);
  }

  .channel-badge {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.75rem;
    color: var(--color-accent, #6366f1);
    background: color-mix(in srgb, var(--color-accent, #6366f1) 15%, transparent);
  }

  .release-notes {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .release-notes h4 {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-primary, #e0e0e0);
  }

  .notes-body {
    margin: 0;
    padding: 0.75rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 8px;
    font-family: 'Inter', sans-serif;
    font-size: 0.85rem;
    color: var(--text-secondary, #9ca3af);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow-y: auto;
  }

  .error-line {
    margin: 0;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    background: rgba(239, 68, 68, 0.15);
    color: #fca5a5;
    font-size: 0.85rem;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .btn-secondary {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-secondary:hover:not(:disabled) {
    color: var(--text-primary, #e0e0e0);
    background: var(--bg-hover, #374151);
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    background: var(--color-accent, #6366f1);
    color: white;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-accent-hover, #4f52d9);
  }

  .btn-secondary:disabled,
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>