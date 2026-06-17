<script>
  import { Play, Pause, SkipForward, SkipBack, Volume2, Maximize, Minimize } from 'lucide-svelte';
  import { t } from '../../i18n';
  import { modoCineActive } from '@features/player/stores/player';
</script>

<div class="bottom-bar">
  <div class="track-info">
    <div class="track-placeholder"></div>
    <div class="track-text">
      <span class="track-title">{$t('player.no_track')}</span>
    </div>
  </div>

  <div class="controls">
    <button class="control-btn" aria-label="Previous">
      <SkipBack size={18} />
    </button>
    <button class="control-btn play-btn" aria-label="Play">
      <Play size={22} />
    </button>
    <button class="control-btn" aria-label="Next">
      <SkipForward size={18} />
    </button>
  </div>

  <div class="volume">
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <button
      class="control-btn modo-cine-btn"
      aria-label={$modoCineActive ? $t('visualizer.fullscreen') : $t('visualizer.spectrum')}
      on:click={() => modoCineActive.set(!$modoCineActive)}
    >
      {#if $modoCineActive}
        <Minimize size={16} />
      {:else}
        <Maximize size={16} />
      {/if}
    </button>
    <Volume2 size={18} />
    <div class="volume-bar"></div>
  </div>
</div>

<style>
  .bottom-bar {
    grid-area: bottombar;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-surface, #111827);
    border-top: 1px solid var(--border-color, #1f2937);
    padding: 0.5rem 1.5rem;
    height: 72px;
  }

  .track-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 200px;
  }

  .track-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 4px;
    background: var(--bg-elevated, #1f2937);
  }

  .track-text {
    display: flex;
    flex-direction: column;
  }

  .track-title {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .control-btn {
    background: none;
    border: none;
    color: var(--text-primary, #e0e0e0);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s;
  }

  .control-btn:hover {
    color: var(--color-accent, #6366f1);
  }

  .play-btn {
    background: var(--color-accent, #6366f1);
    color: white;
    width: 36px;
    height: 36px;
    border-radius: 50%;
  }

  .play-btn:hover {
    opacity: 0.9;
    color: white;
  }

  .volume {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 200px;
    justify-content: flex-end;
    color: var(--text-secondary, #9ca3af);
  }

  .volume-bar {
    width: 80px;
    height: 4px;
    background: var(--bg-elevated, #1f2937);
    border-radius: 2px;
  }

  .modo-cine-btn {
    opacity: 0.6;
    transition: opacity 0.2s;
  }

  .modo-cine-btn:hover {
    opacity: 1;
  }
</style>