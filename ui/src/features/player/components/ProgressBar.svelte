<script lang="ts">
  import { t } from '@i18n';
  import { progress, seekTo } from '../stores/player';

  function formatTime(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  function handleSeek(e: MouseEvent): void {
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const duration = $progress.duration;
    if (duration > 0) {
      seekTo(ratio * duration);
    }
  }
</script>

<div class="progress-bar">
  <span class="time-label">{formatTime($progress.position)}</span>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="bar-track" on:click={handleSeek}>
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
  }

  .bar-track:hover {
    height: 8px;
  }

  .bar-fill {
    height: 100%;
    background: var(--color-accent, #6366f1);
    border-radius: 3px;
    transition: width 0.1s linear;
  }
</style>