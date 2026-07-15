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
  import { updateMiniWindowSize } from './nativeWindow';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import MiniVisualizer from './MiniVisualizer.svelte';

  $: skin = resolveMiniPlayerSkin($selectedMiniPlayerSkinId);
  $: skinSize = resolveMiniPlayerWindowSize(skin, $miniPlayerScale);
  $: skinScale = resolveMiniPlayerSkinScale(skin, $miniPlayerScale);
  $: progressPct = $progress.duration > 0 ? Math.min(100, Math.max(0, ($progress.position / $progress.duration) * 100)) : 0;

  let latestRequestedSize: typeof skinSize | null = null;
  let resizeInFlight: Promise<void> | null = null;

  function queueMiniWindowSize(size: typeof skinSize): void {
    latestRequestedSize = size;
    if (resizeInFlight) return;

    const drainResizeQueue = async (): Promise<void> => {
      while (latestRequestedSize) {
        const requestedSize = latestRequestedSize;
        latestRequestedSize = null;
        await updateMiniWindowSize(requestedSize).catch(() => undefined);
      }
    };

    resizeInFlight = drainResizeQueue().finally(() => {
      resizeInFlight = null;
      if (latestRequestedSize) queueMiniWindowSize(latestRequestedSize);
    });
  }

  // Resize the native window whenever scale or skin changes (preserves position).
  $: if (skinSize) {
    queueMiniWindowSize(skinSize);
  }

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  // Tauri's `data-tauri-drag-region` is unreliable on Windows with frameless
  // + transparent windows: the compositor has no opaque surface to hit-test,
  // and inline/img child elements intercept the mousedown. Calling
  // `startDragging()` directly issues the native SC_MOVE command, which works
  // reliably on all platforms. We keep `data-tauri-drag-region` as a fallback
  // for Linux/macOS.
  function startDrag(e: MouseEvent): void {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest('button')) return;
    getCurrentWindow().startDragging().catch(() => undefined);
  }
</script>

<main
  class="mini-player"
  style="--skin-card-width: {skin.window.width}px; --skin-card-height: {skin.window.height}px; --skin-window-width: {skinSize.width}px; --skin-window-height: {skinSize.height}px; --skin-scale: {skinScale}; --skin-shell: {skin.theme.shell}; --skin-shell-edge: {skin.theme.shellEdge}; --skin-screen: {skin.theme.screen}; --skin-screen-text: {skin.theme.screenText}; --skin-accent: {skin.theme.accent}; --skin-control-surface: {skin.theme.controlSurface}; --skin-control-text: {skin.theme.controlText};"
  aria-label="Mini player"
>
  <div class="device-wrapper">
    <section class="device" class:compact={skinScale < 0.5} data-skin={skin.id} data-kind={skin.kind} data-shape={skin.shape} aria-label={skin.name}>
      <div class="window-controls" aria-label="Mini player window controls">
        <button class="window-control restore-btn" type="button" on:click={exitMiniPlayer} aria-label="Return to full app">
          <Maximize2 size={14} />
          <span>Full app</span>
        </button>
        <button class="window-control icon-btn" type="button" on:click={minimizeMiniPlayer} aria-label="Minimize mini player">
          <Minus size={14} />
        </button>
        <button class="window-control icon-btn close-btn" type="button" on:click={quitFromMiniPlayer} aria-label="Quit Jellyx Player">
          <X size={14} />
        </button>
      </div>

      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="screen" data-tauri-drag-region on:mousedown={startDrag} role="region" aria-label="Mini player display">
        <div class="track-card" data-tauri-drag-region>
          {#if $currentTrack && albumArtUrl($currentTrack.thumbnail)}
            <img src={albumArtUrl($currentTrack.thumbnail)} alt="Album art" class="artwork" data-tauri-drag-region />
          {:else}
            <div class="artwork placeholder" data-tauri-drag-region>Jellyx</div>
          {/if}
          <div class="metadata" data-tauri-drag-region>
            <strong data-tauri-drag-region>{$currentTrack?.title ?? 'No track selected'}</strong>
            <span data-tauri-drag-region>{$currentTrack?.artist ?? skin.name}</span>
          </div>
        </div>
        <div class="progress" aria-label="Playback progress" data-tauri-drag-region>
          <span style="width: {progressPct}%" data-tauri-drag-region></span>
        </div>
        <div class="times" data-tauri-drag-region>
          <span data-tauri-drag-region>{formatTime($progress.position)}</span>
          <span data-tauri-drag-region>{formatTime($progress.duration)}</span>
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
  </div>
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
    background: transparent;
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
    transform-origin: top left;
  }

  /* The .device layout box is the unscaled skin base (e.g. 320x480), visually
   * scaled by `transform`. To make the *layout* box match the window's scaled
   * size so grid centering works, we wrap it in a container sized to the
   * scaled window dimension. Without this, `transform: scale()` doesn't
   * change the layout box, so the grid centers an oversized box → overflow.
   */
  .device-wrapper {
    width: var(--skin-window-width);
    height: var(--skin-window-height);
    overflow: hidden;
    display: grid;
    place-items: start;
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

</style>
