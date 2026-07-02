<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { frequencyData, modoCineActive, visualizerMode, currentTrack } from '../stores/player';
  import { createFftChannel } from '@services/events';
  import type { FrequencyData } from '@shared/types/models';
  import { resolveVisualizerMode, type VisualizerModeEntry } from '../visualizers/registry';
  import type { VisualizerTheme } from '../visualizers/types';
  import VisualizerSelector from './VisualizerSelector.svelte';

  let canvas: HTMLCanvasElement;
  let overlayEl: HTMLDivElement | null = null;
  let rafId: number | null = null;
  let unlisten: (() => void) | null = null;

  // ── Auto-hide overlay chrome ────────────────────────────────────
  /** Whether the overlay chrome (close button + selector) is currently visible.
   *  The track title is always visible; only chrome auto-hides on mouse idle so
   *  the fullscreen view stays clean with just the visual effect and the title. */
  let controlsVisible = true;
  let lastMouseX = -1;
  let lastMouseY = -1;
  let wasModoCineActive = false;

  /** Idle timer handle for auto-hiding chrome after the mouse stops moving. */
  let idleTimer: ReturnType<typeof setTimeout> | null = null;

  /** Delay before hiding chrome after the mouse stops moving (ms).
   *  Long enough that the user can read/aim the controls, short enough that the
   *  view returns to a clean state quickly once idle. */
  const CHROME_IDLE_DELAY = 2500;
  /** Highest frequency we render in the visualizer.
   *  The raw FFT covers the whole Nyquist range, but the upper tail is often
   *  visually dead in real-world tracks. Capping the rendered spectrum keeps
   *  the visualization denser and avoids long zones that look stuck at zero. */
  const MAX_VISUAL_FREQ_HZ = 12_000;

  /** Arm/re-arm the idle timer that hides the overlay chrome. */
  function scheduleChromeHide(): void {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(() => {
      controlsVisible = false;
    }, CHROME_IDLE_DELAY);
  }

  /** Reveal chrome and restart the idle countdown. Called on pointer activity
   *  inside the fullscreen overlay. The instant the pointer moves, controls
   *  reappear; if it stays still for CHROME_IDLE_DELAY, they fade out again. */
  function revealChrome(): void {
    controlsVisible = true;
    scheduleChromeHide();
  }

  /** Pointer movement handler — only acts while the fullscreen overlay is active. */
  function handlePointerMove(e: PointerEvent): void {
    if (!$modoCineActive) return;
    // Ignore synthetic / zero-delta moves so the idle timer is not constantly
    // restarted by noisy pointer events on some WebKit/desktop setups.
    if (e.clientX === lastMouseX && e.clientY === lastMouseY) return;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
    revealChrome();
  }

  /** Any pointer-down inside the overlay should also reveal chrome. */
  function handlePointerDown(): void {
    if (!$modoCineActive) return;
    revealChrome();
  }

  /** Clear the idle timer and reset chrome state (used on exit/cleanup). */
  function clearIdleTimer(): void {
    if (idleTimer) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
    controlsVisible = true;
    lastMouseX = -1;
    lastMouseY = -1;
  }

  // Local reference to frequency data for rAF loop (avoids reactive dependency in animation)
  let currentData: FrequencyData | null = null;

  // Subscribe to frequency data and keep local reference for rAF loop
  $: if ($frequencyData) {
    currentData = $frequencyData;
  }

  // Resolve the active renderer from the persisted mode id. Re-evaluated only
  // when the mode id changes — the rAF loop reads `activeMode` directly, so a
  // mode switch takes effect on the next frame with no effect churn.
  let activeMode: VisualizerModeEntry = resolveVisualizerMode($visualizerMode);
  $: activeMode = resolveVisualizerMode($visualizerMode);

  onMount(async () => {
    // Subscribe to binary FFT stream via Tauri Channel
    unlisten = await createFftChannel((data: FrequencyData) => {
      frequencyData.set(data);
    });

    // Initial canvas sizing
    handleResize();

    // Start rAF loop
    startRenderLoop();

    // Escape key exits Modo Cine
    function handleKeydown(e: KeyboardEvent): void {
      if (e.key === 'Escape' && $modoCineActive) {
        modoCineActive.set(false);
      }
    }
    window.addEventListener('keydown', handleKeydown);

    // Start the idle countdown immediately on mount (chrome visible, then hides
    // if the mouse never moves — which is the common clean view).
    revealChrome();

    // Store cleanup for the keydown listener
    keydownHandler = handleKeydown;
  });

  let keydownHandler: ((e: KeyboardEvent) => void) | null = null;

  function startRenderLoop(): void {
    function frame(): void {
      renderFrame();
      rafId = requestAnimationFrame(frame);
    }
    rafId = requestAnimationFrame(frame);
  }

  /** Resolve theme tokens from CSS custom properties once per frame.
   *  Reading computed style every frame is cheap (no layout/reflow) and keeps
   *  the renderers stateless. */
  function resolveTheme(): VisualizerTheme {
    const styles = getComputedStyle(document.documentElement);
    const accentColor = styles.getPropertyValue('--viz-color-accent').trim() || '#6366f1';
    const barGap = parseInt(styles.getPropertyValue('--viz-bar-gap')) || 2;
    const barMinHeight = parseInt(styles.getPropertyValue('--viz-bar-min-height')) || 2;
    return { accentColor, barGap, barMinHeight };
  }

  function renderFrame(): void {
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Render only the musically-useful part of the spectrum. The raw FFT spans
    // the full Nyquist range; trimming the upper tail avoids bars/cells that
    // sit at zero most of the time and makes all visualizers feel more alive.
    const displayData = currentData ? limitFrequencyRange(currentData) : null;

    // Dispatch to the active renderer (pure Canvas2D, no state).
    activeMode.render(ctx, width, height, displayData, resolveTheme());
  }

  function limitFrequencyRange(data: FrequencyData): FrequencyData {
    const nyquist = data.sampleRate > 0 ? data.sampleRate / 2 : 22_050;
    const cappedHz = Math.min(MAX_VISUAL_FREQ_HZ, nyquist);
    const maxIndex = Math.max(1, Math.min(
      data.bins.length,
      Math.ceil((cappedHz / nyquist) * data.bins.length)
    ));

    const bins = data.bins.subarray(0, maxIndex);
    let peak = 0;
    for (let i = 0; i < bins.length; i++) {
      if (bins[i] > peak) peak = bins[i];
    }

    return {
      bins,
      sampleRate: data.sampleRate,
      peak,
    };
  }

  function handleResize(): void {
    if (!canvas) return;
    const parent = canvas.parentElement;
    if (parent) {
      canvas.width = parent.clientWidth;
      canvas.height = parent.clientHeight;
    }
  }

  onDestroy(() => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    if (keydownHandler) {
      window.removeEventListener('keydown', keydownHandler);
      keydownHandler = null;
    }
    clearIdleTimer();
  });

  $: if ($modoCineActive !== wasModoCineActive) {
    wasModoCineActive = $modoCineActive;
    if (!$modoCineActive) {
      // Reset chrome state when exiting fullscreen so the next entry is clean.
      clearIdleTimer();
      // Resize to parent container
      setTimeout(handleResize, 0);
    } else {
      // Resize canvas for fullscreen cinema mode
      setTimeout(() => {
        if (canvas) {
          canvas.width = window.innerWidth;
          canvas.height = window.innerHeight;
        }
      }, 0);
      // Reveal chrome only ON ENTRY, not on every unrelated component update.
      revealChrome();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="visualizer"
  class:modo-cine={$modoCineActive}
  bind:this={overlayEl}
  on:pointermove={handlePointerMove}
  on:pointerdown={handlePointerDown}
>
  <canvas bind:this={canvas} class="visualizer-canvas"></canvas>
  {#if $modoCineActive}
    <div class="track-title">
      {$currentTrack?.title ?? ''}
    </div>
    {#if controlsVisible}
      <div class="chrome-controls visible">
        <VisualizerSelector />
        <button class="modo-cine-close" aria-label="Exit fullscreen" on:click={() => modoCineActive.set(false)}>
          ✕
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .visualizer {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .visualizer.modo-cine {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    z-index: 100;
    background:
      radial-gradient(ellipse 80% 60% at 50% -20%, rgba(138, 92, 255, 0.12), transparent),
      radial-gradient(ellipse 60% 50% at 80% 100%, rgba(0, 229, 255, 0.08), transparent),
      var(--bg-base, #0a0a0f);
  }

  .visualizer-canvas {
    display: block;
    width: 100%;
    height: 100%;
  }

  /* ── Track title ─────────────────────────────────────────────── */
  .track-title {
    position: absolute;
    top: 1.5rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 101;
    max-width: 70vw;
    color: var(--text-primary, #e0e0e0);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    text-align: center;
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.6);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    /* The title stays visible even when chrome hides — the user explicitly
       asked for the title to remain. */
    opacity: 1;
    transition: opacity 0.3s ease;
    pointer-events: none;
    /* Subtle fade-in on enter so the title doesn't pop. */
  }

  /* ── Chrome controls (close button + selector) ────────────────── */
  /* Wraps the auto-hiding controls. They fade out together when the mouse
     is idle and reappear the instant it moves. */
  .chrome-controls {
    transition: opacity 0.3s ease;
    pointer-events: none; /* allow mouse events to reach the window handler */
    visibility: visible;
  }

  .chrome-controls.visible {
    opacity: 1;
    visibility: visible;
    pointer-events: none; /* children re-enable their own pointer events */
  }

  /* The selector and close button live inside .chrome-controls; they keep
     pointer-events: auto so they remain clickable while visible. */
  .chrome-controls :global(.viz-selector) {
    pointer-events: auto;
  }

  .modo-cine-close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    z-index: 101;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--text-primary, #e0e0e0);
    font-size: 1.25rem;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s, box-shadow 0.2s;
    backdrop-filter: blur(4px);
    pointer-events: auto;
  }

  .modo-cine-close:hover {
    background: rgba(255, 255, 255, 0.15);
    box-shadow: 0 0 12px rgba(138, 92, 255, 0.3);
  }
</style>
