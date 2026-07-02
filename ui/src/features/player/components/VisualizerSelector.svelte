<script lang="ts">
  /**
   * Visualizer selector — renders a compact mode switcher inside the
   * fullscreen overlay.
   *
   * Only mounted while `modoCineActive` is true (the host conditionally
   * includes it), so it has no cost when the overlay is closed. It reads the
   * persisted `visualizerMode` store and the registry's mode list; selecting
   * a mode just sets the store, and the host's rAF loop picks up the new
   * renderer on the next frame.
   *
   * Styling is intentionally minimal and pointer-events:auto so clicks land on
   * the buttons and not on the canvas underneath.
   */
  import { t } from '@i18n';
  import { visualizerMode } from '@features/player/stores/player';
  import { VISUALIZER_MODES } from '../visualizers/registry';
</script>

<div class="viz-selector" role="tablist" aria-label={$t('visualizer.spectrum')}>
  {#each VISUALIZER_MODES as mode (mode.id)}
    <button
      class="viz-chip"
      class:active={$visualizerMode === mode.id}
      role="tab"
      aria-selected={$visualizerMode === mode.id}
      aria-label={$t(mode.labelKey)}
      on:click={() => visualizerMode.set(mode.id)}
    >
      {$t(mode.labelKey)}
    </button>
  {/each}
</div>

<style>
  .viz-selector {
    position: absolute;
    bottom: 1.25rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 101;
    display: flex;
    gap: 0.4rem;
    padding: 0.35rem;
    background: rgba(10, 10, 15, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 999px;
    backdrop-filter: blur(8px);
    pointer-events: auto;
  }

  .viz-chip {
    background: transparent;
    border: none;
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    font-weight: 500;
    letter-spacing: 0.02em;
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .viz-chip:hover {
    color: var(--text-primary, #e0e0e0);
    background: rgba(255, 255, 255, 0.06);
  }

  .viz-chip.active {
    color: #fff;
    background: var(--color-accent, #6366f1);
  }
</style>