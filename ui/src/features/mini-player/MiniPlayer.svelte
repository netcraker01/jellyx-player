<script lang="ts">
  import { Play, Pause, SkipBack, SkipForward, Maximize2 } from 'lucide-svelte';
  import { exitMiniPlayer } from './mode';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import {
    currentTrack,
    isPlaying,
    progress,
    togglePlayPause,
    nextTrack,
    previousTrack,
  } from '@features/player/stores/player';
  import { resolveMiniPlayerSkin, resolveMiniPlayerWindowSize, selectedMiniPlayerSkinId } from './skins';

  $: skin = resolveMiniPlayerSkin($selectedMiniPlayerSkinId);
  $: skinSize = resolveMiniPlayerWindowSize(skin);
  $: progressPct = $progress.duration > 0 ? Math.min(100, Math.max(0, ($progress.position / $progress.duration) * 100)) : 0;

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }
</script>

<main
  class="mini-player"
  style="--skin-card-width: {skinSize.width}px; --skin-card-height: {skinSize.height}px; --skin-shell: {skin.theme.shell}; --skin-shell-edge: {skin.theme.shellEdge}; --skin-screen: {skin.theme.screen}; --skin-screen-text: {skin.theme.screenText}; --skin-accent: {skin.theme.accent}; --skin-control-surface: {skin.theme.controlSurface}; --skin-control-text: {skin.theme.controlText};"
  aria-label="Mini player"
>
  <section class="device" data-skin={skin.id} aria-label={skin.name}>
    <button class="restore-btn" type="button" on:click={exitMiniPlayer} aria-label="Return to full app">
      <Maximize2 size={14} />
      <span>Full app</span>
    </button>

    <div class="screen">
      {#if $currentTrack && albumArtUrl($currentTrack.thumbnail)}
        <img src={albumArtUrl($currentTrack.thumbnail)} alt="Album art" class="artwork" />
      {:else}
        <div class="artwork placeholder">Helix</div>
      {/if}
      <div class="metadata">
        <strong>{$currentTrack?.title ?? 'No track selected'}</strong>
        <span>{$currentTrack?.artist ?? skin.name}</span>
      </div>
      <div class="progress" aria-label="Playback progress">
        <span style="width: {progressPct}%"></span>
      </div>
      <div class="times">
        <span>{formatTime($progress.position)}</span>
        <span>{formatTime($progress.duration)}</span>
      </div>
    </div>

    <div class="click-wheel" aria-label="Playback controls">
      <button class="wheel-btn prev" aria-label="Previous" on:click={previousTrack}><SkipBack size={18} /></button>
      <button class="wheel-btn next" aria-label="Next" on:click={nextTrack}><SkipForward size={18} /></button>
      <button class="wheel-btn center" aria-label={$isPlaying ? 'Pause' : 'Play'} on:click={togglePlayPause}>
        {#if $isPlaying}
          <Pause size={28} />
        {:else}
          <Play size={28} />
        {/if}
      </button>
    </div>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
  }

  .mini-player {
    width: 100vw;
    height: 100vh;
    display: grid;
    place-items: center;
    background: radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 0.3), transparent 45%), #111827;
    font-family: Inter, system-ui, sans-serif;
  }

  .device {
    width: min(100vw, var(--skin-card-width));
    height: min(100vh, var(--skin-card-height));
    padding: 18px 20px 26px;
    border-radius: 34px;
    background: linear-gradient(145deg, #fff, var(--skin-shell) 48%, #dde1e7);
    border: 1px solid var(--skin-shell-edge);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.9), 0 24px 70px rgba(0, 0, 0, 0.45);
    color: var(--skin-control-text);
    box-sizing: border-box;
  }

  .restore-btn {
    width: 100%;
    margin-bottom: 10px;
    border: 1px solid var(--skin-shell-edge);
    border-radius: 999px;
    padding: 7px 10px;
    background: rgba(255, 255, 255, 0.62);
    color: var(--skin-control-text);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font: inherit;
    font-size: 0.76rem;
    font-weight: 700;
  }

  .restore-btn:hover { color: var(--skin-accent); }

  .screen {
    height: 172px;
    padding: 12px;
    border-radius: 12px;
    background: var(--skin-screen);
    color: var(--skin-screen-text);
    border: 3px solid #2f3742;
    box-shadow: inset 0 2px 10px rgba(0, 0, 0, 0.25);
    box-sizing: border-box;
  }

  .artwork {
    width: 76px;
    height: 76px;
    border-radius: 6px;
    object-fit: cover;
    float: left;
    margin-right: 12px;
    background: rgba(255, 255, 255, 0.45);
  }

  .placeholder {
    display: grid;
    place-items: center;
    font-size: 0.8rem;
    font-weight: 700;
  }

  .metadata {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-height: 76px;
  }

  .metadata strong,
  .metadata span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .metadata strong { font-size: 0.95rem; }
  .metadata span { font-size: 0.8rem; opacity: 0.8; }

  .progress {
    clear: both;
    height: 6px;
    margin-top: 18px;
    border-radius: 999px;
    background: rgba(31, 41, 51, 0.18);
    overflow: hidden;
  }

  .progress span {
    display: block;
    height: 100%;
    background: var(--skin-accent);
  }

  .times {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
  }

  .click-wheel {
    position: relative;
    width: 190px;
    height: 190px;
    margin: 34px auto 0;
    border-radius: 50%;
    background: var(--skin-control-surface);
    box-shadow: inset 0 -8px 20px rgba(0, 0, 0, 0.08), inset 0 4px 14px rgba(255, 255, 255, 0.9);
  }

  .wheel-btn {
    position: absolute;
    border: 0;
    background: transparent;
    color: var(--skin-control-text);
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .wheel-btn:hover { color: var(--skin-accent); }
  .prev { left: 18px; top: 82px; }
  .next { right: 18px; top: 82px; }

  .center {
    inset: 54px;
    border-radius: 50%;
    background: var(--skin-shell);
    box-shadow: inset 0 3px 10px rgba(0, 0, 0, 0.14);
  }
</style>
