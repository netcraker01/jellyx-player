<script lang="ts">
  import { Play, Pause, SkipForward, SkipBack, Shuffle, Repeat, Repeat1 } from 'lucide-svelte';
  import { t } from '@i18n';
  import { isPlaying, shuffle, repeatMode, togglePlayPause, nextTrack, previousTrack, toggleShuffle, cycleRepeat } from '../stores/player';

  export let disabled = false;

  $: repeatLabel = $repeatMode === 'One' ? $t('player.repeat_one') : $repeatMode === 'All' ? $t('player.repeat_all') : $t('player.repeat_off');
</script>

<div class="controls">
  <button
    class="control-btn mode-btn"
    class:active={$shuffle}
    aria-label={$t('player.shuffle')}
    on:click={toggleShuffle}
    disabled={disabled}
  >
    <Shuffle size={18} />
  </button>

  <button class="control-btn" aria-label="Previous" on:click={previousTrack} disabled={disabled}>
    <SkipBack size={20} />
  </button>
  <button class="control-btn play-btn" aria-label={$isPlaying ? $t('player.pause') : $t('player.play')} on:click={togglePlayPause} disabled={disabled}>
    {#if $isPlaying}
      <Pause size={24} />
    {:else}
      <Play size={24} />
    {/if}
  </button>
  <button class="control-btn" aria-label="Next" on:click={nextTrack} disabled={disabled}>
    <SkipForward size={20} />
  </button>

  <button
    class="control-btn mode-btn"
    class:active={$repeatMode !== 'Off'}
    aria-label={repeatLabel}
    on:click={cycleRepeat}
    disabled={disabled}
  >
    {#if $repeatMode === 'One'}
      <Repeat1 size={18} />
    {:else}
      <Repeat size={18} />
    {/if}
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1.5rem;
  }

  .control-btn {
    background: none;
    border: none;
    color: var(--text-primary, #e0e0e0);
    cursor: pointer;
    padding: 0.5rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, transform 0.1s, background 0.2s;
  }

  .control-btn:hover:not(:disabled) {
    color: var(--color-helix-cyan, #00E5FF);
    background: rgba(0, 229, 255, 0.08);
  }

  .control-btn:active:not(:disabled) {
    transform: scale(0.95);
  }

  .control-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .play-btn {
    background: var(--helix-gradient-primary);
    color: #0a0a0f;
    width: 52px;
    height: 52px;
    border-radius: 50%;
    box-shadow: 0 4px 16px rgba(138, 92, 255, 0.35);
  }

  .play-btn:hover:not(:disabled) {
    opacity: 0.92;
    color: #0a0a0f;
    box-shadow: 0 6px 20px rgba(138, 92, 255, 0.45);
  }

  .mode-btn {
    color: var(--text-secondary, #9ca3af);
  }

  .mode-btn.active {
    color: var(--color-helix-cyan, #00E5FF);
    background: rgba(0, 229, 255, 0.08);
  }

  .mode-btn:hover:not(:disabled) {
    color: var(--color-helix-cyan, #00E5FF);
  }
</style>
