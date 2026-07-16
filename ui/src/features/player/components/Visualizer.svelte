<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { frequencyData, modoCineActive, visualizerMode, currentTrack } from '../stores/player';
  import type { FrequencyData } from '@shared/types/models';
  import { resolveVisualizerMode, type VisualizerModeEntry } from '../visualizers/registry';
  import { limitFrequencyRange, createActiveRange, type ActiveRangeState } from '../visualizers/activeRange';
  import type { VisualizerTheme } from '../visualizers/types';
  import VisualizerSelector from './VisualizerSelector.svelte';

  let canvas: HTMLCanvasElement;
  let overlayEl: HTMLDivElement | null = null;
  let rafId: number | null = null;

  const activeRange: ActiveRangeState = createActiveRange();

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
  $: { $visualizerMode; refreshTheme(); }

  onMount(() => {
    // Initial canvas sizing and theme cache
    handleResize();
    refreshTheme();

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

  /** Cache theme tokens so we don't call getComputedStyle every frame.
   *  Refreshed on mode change and on resize (theme may change with route). */
  let cachedTheme: VisualizerTheme = { accentColor: '#6366f1', barGap: 2, barMinHeight: 2 };

  function refreshTheme(): VisualizerTheme {
    const styles = getComputedStyle(document.documentElement);
    const accentColor = styles.getPropertyValue('--viz-color-accent').trim() || '#6366f1';
    const barGap = parseInt(styles.getPropertyValue('--viz-bar-gap')) || 2;
    const barMinHeight = parseInt(styles.getPropertyValue('--viz-bar-min-height')) || 2;
    cachedTheme = { accentColor, barGap, barMinHeight };
    return cachedTheme;
  }

  /** Max backing-store dimensions for the canvas. The visualizer is a background
   *  effect — rendering at full monitor resolution (e.g. 4K) is visually
   *  indistinguishable from 1280×720 but 4-9× slower on CPU Canvas2D. CSS scales
   *  the bitmap up to fill the viewport. */
  const MAX_CANVAS_WIDTH = 1280;
  const MAX_CANVAS_HEIGHT = 720;

  /** Cached 2D context. Requested once with GPU-friendly options.
   *  Modo cine needs alpha:true so the CSS gradient background of
   *  .visualizer.modo-cine shows through the clearRect'd canvas.
   *  desynchronized:true decouples paint from the event loop for lower latency.
   *  willReadFrequently:false hints the browser to use hardware-accelerated Canvas2D. */
  let cachedCtx: CanvasRenderingContext2D | null = null;

  function getCtx(): CanvasRenderingContext2D | null {
    if (cachedCtx) return cachedCtx;
    cachedCtx = canvas.getContext('2d', {
      alpha: true,
      desynchronized: true,
      willReadFrequently: false,
    }) as CanvasRenderingContext2D | null;
    return cachedCtx;
  }

  function renderFrame(): void {
    if (!canvas) return;

    const ctx = getCtx();
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Render only the musically-useful part of the spectrum. The raw FFT spans
    // the full Nyquist range; trimming the upper tail avoids bars/cells that
    // sit at zero most of the time and makes all visualizers feel more alive.
    const displayData = currentData ? limitFrequencyRange(activeRange, currentData, $currentTrack?.id) : null;

    // Dispatch to the active renderer (pure Canvas2D, no state).
    activeMode.render(ctx, width, height, displayData, cachedTheme);
  }

  function handleResize(): void {
    if (!canvas) return;
    const parent = canvas.parentElement;
    if (parent) {
      let w = parent.clientWidth;
      let h = parent.clientHeight;
      // Cap the backing store so CPU Canvas2D doesn't paint millions of
      // pixels per frame on high-DPI monitors. CSS scales the bitmap up.
      w = Math.min(w, MAX_CANVAS_WIDTH);
      h = Math.min(h, MAX_CANVAS_HEIGHT);
      canvas.width = Math.max(1, w);
      canvas.height = Math.max(1, h);
      // Setting canvas.width/height resets the context — invalidate cache.
      cachedCtx = null;
    }
    refreshTheme();
  }

  onDestroy(() => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
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
      // Resize canvas for modo cine fullscreen — the visualizer is in a
      // fixed full-viewport overlay (.visualizer-embed) inside .app-shell.
      setTimeout(() => {
        if (canvas) {
          canvas.width = Math.min(window.innerWidth, MAX_CANVAS_WIDTH);
          canvas.height = Math.min(window.innerHeight, MAX_CANVAS_HEIGHT);
          cachedCtx = null; // canvas resize resets the context
          refreshTheme();
        }
      }, 0);
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
      </div>
      <button class="modo-cine-close" aria-label="Exit fullscreen" on:click={() => modoCineActive.set(false)}>
        ✕
      </button>
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

  /* Modo cine: fills the full viewport as a fullscreen overlay. The parent
     .visualizer-embed is fixed with z-index 99. */
  .visualizer.modo-cine {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
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
  /* Track title and chrome controls must paint above the sidebar/content/bottombar
     (z-index: 1). They live inside .visualizer-embed (z-index: 99) so they
     are already above all app content. */
  .track-title {
    position: fixed;
    top: 1.5rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 50;
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
    position: fixed;
    bottom: 90px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 50;
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
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 50;
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
