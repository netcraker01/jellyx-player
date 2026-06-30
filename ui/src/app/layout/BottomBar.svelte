<script lang="ts">
  import { Play, Pause, SkipForward, SkipBack, Volume2, VolumeX, Maximize, Minimize, Shuffle, Repeat, Repeat1 } from 'lucide-svelte';
  import { t } from '@i18n';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import {
    currentTrack,
    isPlaying,
    progress,
    volume,
    modoCineActive,
    shuffle,
    repeatMode,
    togglePlayPause,
    nextTrack,
    previousTrack,
    toggleShuffle,
    cycleRepeat,
    seekTo,
    setVolume,
  } from '@features/player/stores/player';

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  $: repeatLabel = $repeatMode === 'One' ? $t('player.repeat_one') : $repeatMode === 'All' ? $t('player.repeat_all') : $t('player.repeat_off');

  function handleProgressClick(e: MouseEvent): void {
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const duration = $progress.duration;
    if (Number.isFinite(duration) && duration > 0) {
      seekTo(ratio * duration);
    }
  }

  function handleVolumeChange(e: Event): void {
    const input = e.target as HTMLInputElement;
    setVolume(parseInt(input.value, 10));
  }

  function toggleMute(): void {
    if ($volume > 0) {
      previousVolume = $volume;
      setVolume(0);
    } else {
      setVolume(previousVolume || 80);
    }
  }

  let previousVolume = 80;

  // ── Marquee detection ───────────────────────────────────────────────
  let titleEl: HTMLSpanElement | null = null;
  let needsMarquee = false;

  function checkMarquee() {
    if (titleEl) {
      needsMarquee = titleEl.scrollWidth > titleEl.clientWidth + 1;
    }
  }

  // Check on track change and after DOM update
  $: if ($currentTrack?.title) {
    needsMarquee = false;
    // Use requestAnimationFrame to measure after render
    requestAnimationFrame(() => {
      requestAnimationFrame(checkMarquee);
    });
  }
</script>

<div class="bottom-bar">
  <div class="track-info">
    {#if $currentTrack && albumArtUrl($currentTrack.thumbnail)}
      <img class="track-thumbnail" src={albumArtUrl($currentTrack.thumbnail)} alt="Album art" />
    {:else}
      <div class="track-placeholder">
        <HelixLogo size={20} monochrome={true} />
      </div>
    {/if}
    <div class="track-text">
      <span class="track-title" class:marquee={needsMarquee} bind:this={titleEl}>{$currentTrack?.title ?? $t('player.no_track')}</span>
      {#if $currentTrack?.artist}
        <span class="track-artist">{$currentTrack.artist}</span>
      {/if}
    </div>
  </div>

  <div class="controls-center">
    <div class="controls">
      <button
        class="control-btn mode-btn"
        class:active={$shuffle}
        aria-label={$t('player.shuffle')}
        on:click={toggleShuffle}
      >
        <Shuffle size={16} />
      </button>
      <button class="control-btn" aria-label="Previous" on:click={previousTrack}>
        <SkipBack size={18} />
      </button>
      <button class="control-btn play-btn" aria-label={$isPlaying ? $t('player.pause') : $t('player.play')} on:click={togglePlayPause}>
        {#if $isPlaying}
          <Pause size={22} />
        {:else}
          <Play size={22} />
        {/if}
      </button>
      <button class="control-btn" aria-label="Next" on:click={nextTrack}>
        <SkipForward size={18} />
      </button>
      <button
        class="control-btn mode-btn"
        class:active={$repeatMode !== 'Off'}
        aria-label={repeatLabel}
        on:click={cycleRepeat}
      >
        {#if $repeatMode === 'One'}
          <Repeat1 size={16} />
        {:else}
          <Repeat size={16} />
        {/if}
      </button>
    </div>
    <div class="progress-section">
      <span class="time-label">{formatTime($progress.position)}</span>
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <div class="progress-bar" on:click={handleProgressClick}>
        <div
          class="progress-fill"
          style="width: {$progress.duration > 0 ? ($progress.position / $progress.duration) * 100 : 0}%"
        ></div>
      </div>
      <span class="time-label">{formatTime($progress.duration)}</span>
    </div>
  </div>

  <div class="volume">
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
    <button class="control-btn" on:click={toggleMute} aria-label="Mute">
      {#if $volume === 0}
        <VolumeX size={18} />
      {:else}
        <Volume2 size={18} />
      {/if}
    </button>
    <input
      type="range"
      class="volume-slider"
      min="0"
      max="100"
      bind:value={$volume}
      on:change={handleVolumeChange}
      aria-label={$t('player.volume')}
    />
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
    --marquee-visible: 200px;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
    max-width: 280px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .track-thumbnail {
    width: 48px;
    height: 48px;
    border-radius: 4px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .track-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 4px;
    background: var(--bg-elevated, #1f2937);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .track-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .track-title {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.85rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: inline-block;
  }

  .track-title.marquee {
    text-overflow: clip;
    animation: marquee-scroll 10s linear infinite;
    padding-right: 2rem;
  }

  @keyframes marquee-scroll {
    0% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(0);
    }
    80% {
      transform: translateX(calc(-100% + var(--marquee-visible, 200px)));
    }
    100% {
      transform: translateX(calc(-100% + var(--marquee-visible, 200px)));
    }
  }

  .track-artist {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .controls-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    flex: 1;
    max-width: 600px;
    min-width: 0;
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

  .mode-btn {
    color: var(--text-secondary, #9ca3af);
  }

  .mode-btn.active {
    color: var(--color-accent, #6366f1);
  }

  .progress-section {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
  }

  .time-label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    min-width: 32px;
    text-align: center;
  }

  .progress-bar {
    flex: 1;
    height: 4px;
    background: var(--bg-elevated, #1f2937);
    border-radius: 2px;
    cursor: pointer;
    position: relative;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-accent, #6366f1);
    border-radius: 2px;
    transition: width 0.1s linear;
  }

  .volume {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 200px;
    justify-content: flex-end;
    color: var(--text-secondary, #9ca3af);
  }

  .volume-slider {
    width: 80px;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--bg-elevated, #1f2937);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .volume-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-accent, #6366f1);
    cursor: pointer;
  }

  .volume-slider::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-accent, #6366f1);
    border: none;
    cursor: pointer;
  }

  .modo-cine-btn {
    opacity: 0.6;
    transition: opacity 0.2s;
  }

  .modo-cine-btn:hover {
    opacity: 1;
  }
</style>
