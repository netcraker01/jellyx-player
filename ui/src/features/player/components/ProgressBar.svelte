<script lang="ts">
  import { t } from '@i18n';
  import { progress, seekTo } from '../stores/player';

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  function handleSeek(e: MouseEvent): void {
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const duration = $progress.duration;
    // Guard against Infinity/NaN — YouTube m4a streams may report Infinity
    // as duration until metadata is fully parsed, which would make seek
    // compute NaN positions and break playback entirely.
    if (Number.isFinite(duration) && duration > 0) {
      seekTo(ratio * duration);
    }
  }
</script>

<div class="progress-bar">
  <span class="time-label">{formatTime($progress.position)}</span>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="bar-track" on:click={handleSeek}>
    <div class="bar-glow"></div>
    <div
      class="bar-fill"
      style="width: {$progress.duration > 0 ? ($progress.position / $progress.duration) * 100 : 0}%"
    ></div>
  </div>
  <span class="time-label">{formatTime($progress.duration)}</span>
</div>

<style>
  .progress-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    max-width: 500px;
  }

  .time-label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: center;
  }

  .bar-track {
    flex: 1;
    height: 6px;
    background: var(--bg-elevated, #1f2937);
    border-radius: 3px;
    cursor: pointer;
    position: relative;
    overflow: hidden;
  }

  .bar-track:hover {
    height: 8px;
  }

  .bar-glow {
    position: absolute;
    inset: 0;
    border-radius: 3px;
    background: var(--jellyx-gradient-progress);
    opacity: 0.12;
    pointer-events: none;
  }

  .bar-fill {
    height: 100%;
    background: var(--jellyx-gradient-progress);
    border-radius: 3px;
    transition: width 0.1s linear;
    position: relative;
  }

  .bar-fill::after {
    content: '';
    position: absolute;
    right: 0;
    top: 50%;
    transform: translate(50%, -50%);
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-jellyx-cyan, #00E5FF);
    opacity: 0;
    transition: opacity 0.15s;
    box-shadow: 0 0 8px rgba(0, 229, 255, 0.5);
  }

  .bar-track:hover .bar-fill::after {
    opacity: 1;
  }
</style>
