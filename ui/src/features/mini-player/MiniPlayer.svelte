<script lang="ts">
  import { Play, Pause, SkipBack, SkipForward, Maximize2, Minus, X } from 'lucide-svelte';
  import { exitMiniPlayer, minimizeMiniPlayer, quitFromMiniPlayer } from './mode';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import {
    currentTrack,
    isPlaying,
    progress,
    togglePlayPause,
    nextTrack,
    previousTrack,
  } from '@features/player/stores/player';
  import { miniPlayerScale, resolveMiniPlayerSkin, resolveMiniPlayerSkinScale, resolveMiniPlayerWindowSize, selectedMiniPlayerSkinId } from './skins';
  import MiniVisualizer from './MiniVisualizer.svelte';

  $: skin = resolveMiniPlayerSkin($selectedMiniPlayerSkinId);
  $: skinSize = resolveMiniPlayerWindowSize(skin, $miniPlayerScale);
  $: skinScale = resolveMiniPlayerSkinScale(skin, $miniPlayerScale);
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
  style="--skin-card-width: {skin.window.width}px; --skin-card-height: {skin.window.height}px; --skin-window-width: {skinSize.width}px; --skin-window-height: {skinSize.height}px; --skin-scale: {skinScale}; --skin-shell: {skin.theme.shell}; --skin-shell-edge: {skin.theme.shellEdge}; --skin-screen: {skin.theme.screen}; --skin-screen-text: {skin.theme.screenText}; --skin-accent: {skin.theme.accent}; --skin-control-surface: {skin.theme.controlSurface}; --skin-control-text: {skin.theme.controlText};"
  aria-label="Mini player"
>
  <section class="device" class:compact={skinScale < 0.5} data-skin={skin.id} data-kind={skin.kind} data-shape={skin.shape} aria-label={skin.name}>
    <div class="window-controls" aria-label="Mini player window controls">
      <button class="window-control restore-btn" type="button" on:click={exitMiniPlayer} aria-label="Return to full app">
        <Maximize2 size={14} />
        <span>Full app</span>
      </button>
      <button class="window-control icon-btn" type="button" on:click={minimizeMiniPlayer} aria-label="Minimize mini player">
        <Minus size={14} />
      </button>
      <button class="window-control icon-btn close-btn" type="button" on:click={quitFromMiniPlayer} aria-label="Quit Helix">
        <X size={14} />
      </button>
    </div>

    <div class="screen">
      <div class="track-card">
        {#if $currentTrack && albumArtUrl($currentTrack.thumbnail)}
          <img src={albumArtUrl($currentTrack.thumbnail)} alt="Album art" class="artwork" />
        {:else}
          <div class="artwork placeholder">Helix</div>
        {/if}
        <div class="metadata">
          <strong>{$currentTrack?.title ?? 'No track selected'}</strong>
          <span>{$currentTrack?.artist ?? skin.name}</span>
        </div>
      </div>
      <div class="progress" aria-label="Playback progress">
        <span style="width: {progressPct}%"></span>
      </div>
      <div class="times">
        <span>{formatTime($progress.position)}</span>
        <span>{formatTime($progress.duration)}</span>
      </div>
      <MiniVisualizer />
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
    width: var(--skin-card-width);
    height: var(--skin-card-height);
    padding: 18px 20px 26px;
    border-radius: 34px;
    background: linear-gradient(145deg, #fff, var(--skin-shell) 48%, #dde1e7);
    border: 1px solid var(--skin-shell-edge);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.9), 0 24px 70px rgba(0, 0, 0, 0.45);
    color: var(--skin-control-text);
    box-sizing: border-box;
    transform: scale(var(--skin-scale));
    transform-origin: center;
  }

  .window-controls {
    width: 100%;
    margin-bottom: 10px;
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 6px;
  }

  .window-control {
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

  .icon-btn {
    width: 32px;
    padding-inline: 0;
  }

  .window-control:hover { color: var(--skin-accent); }
  .close-btn:hover { color: #dc2626; }

  .device.compact .restore-btn span {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .screen {
    position: relative;
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

  .device[data-kind='classic'] {
    padding: 4px 6px;
    border-radius: 8px;
    display: flex;
    flex-direction: row;
    align-items: stretch;
    gap: 6px;
    width: 100%;
    height: auto;
    min-height: 0;
    background:
      radial-gradient(circle at 18% 0%, rgba(255, 255, 255, 0.22), transparent 32%),
      linear-gradient(180deg, #303844 0%, var(--skin-shell) 42%, #080a0d 100%);
    border: 2px solid var(--skin-shell-edge);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.28),
      inset 0 -2px 0 rgba(0, 0, 0, 0.72),
      0 18px 50px rgba(0, 0, 0, 0.58);
    color: var(--skin-control-text);
    box-sizing: border-box;
  }

  .device[data-kind='classic'] .window-controls {
    display: none;
  }

  .device[data-kind='classic'] .screen {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    align-self: stretch;
    min-height: 0;
    padding: 0;
    border-radius: 0;
    border: 0;
    background: transparent;
    color: var(--skin-screen-text);
    box-shadow: none;
    font-family: 'Courier New', ui-monospace, monospace;
    text-shadow: 0 0 6px rgba(255, 209, 102, 0.58);
    overflow: hidden;
  }

  .device[data-kind='classic'] .track-card {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1 1 auto;
    min-height: 0;
  }

  .device[data-kind='classic'] .artwork {
    flex-shrink: 0;
    width: 34px;
    height: 34px;
    border-radius: 5px;
    margin: 0;
    background: rgba(255, 159, 28, 0.12);
    box-shadow: 0 0 0 1px rgba(255, 209, 102, 0.18);
  }

  .device[data-kind='classic'] .metadata {
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    gap: 0.12rem;
    overflow: hidden;
  }

  .device[data-kind='classic'] .metadata strong,
  .device[data-kind='classic'] .metadata span {
    display: inline-block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .device[data-kind='classic'] .metadata strong { font-size: 0.76rem; letter-spacing: 0.02em; line-height: 1.1; }
  .device[data-kind='classic'] .metadata span { font-size: 0.6rem; color: #fbbf24; line-height: 1.1; }

  .device[data-kind='classic'] .progress {
    flex-shrink: 0;
    height: 6px;
    margin-top: 6px;
    clear: none;
    border-radius: 999px;
    background: #070402;
    border: 1px solid #020100;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.8);
  }

  .device[data-kind='classic'] .progress span {
    background: linear-gradient(90deg, #f97316, #ffd166);
    box-shadow: 0 0 8px rgba(255, 159, 28, 0.58);
  }

  .device[data-kind='classic'] .times {
    margin-top: 2px;
    font-size: 0.6rem;
  }

  .device[data-kind='classic'] .click-wheel {
    flex: 0 0 auto;
    align-self: stretch;
    width: auto;
    height: auto;
    min-height: 0;
    margin: 0;
    border-radius: 8px;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 5px;
    padding: 4px 6px;
    background:
      linear-gradient(90deg, rgba(255, 255, 255, 0.04) 0 1px, transparent 1px 5px),
      linear-gradient(180deg, #252b35, #080a0d);
    border: 1px solid #05070a;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.16), inset 0 -1px 0 rgba(0, 0, 0, 0.8);
    box-sizing: border-box;
  }

  .device[data-kind='classic'] .wheel-btn {
    position: static;
    width: 26px;
    height: 26px;
    min-width: 0;
    border-radius: 5px;
    background: linear-gradient(180deg, #4f5b6c, var(--skin-control-surface) 48%, #11151b);
    border: 1px solid #06080b;
    color: var(--skin-control-text);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), inset 0 -2px 0 rgba(0, 0, 0, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .device[data-kind='classic'] .center {
    inset: auto;
    width: 30px;
    height: 30px;
    border-radius: 5px;
    background: radial-gradient(circle at 50% 30%, #ffd166, var(--skin-accent) 58%, #7c2d12 100%);
    color: #140b03;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.58), inset 0 -2px 0 rgba(0, 0, 0, 0.35), 0 0 10px rgba(255, 159, 28, 0.18);
  }
</style>
