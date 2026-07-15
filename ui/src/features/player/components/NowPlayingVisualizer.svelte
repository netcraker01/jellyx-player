<script lang="ts">
  /**
   * Bars visualizer for the NowPlaying page.
   *
   * Uses a custom renderer that skips near-zero energy bins — only bars
   * with meaningful signal fill the width. ResizeObserver for sizing,
   * same pattern as MiniVisualizer.
   */
  import { onMount, onDestroy } from 'svelte';
  import { frequencyData, currentTrack } from '@features/player/stores/player';
  import { limitFrequencyRange, createActiveRange, type ActiveRangeState } from '@features/player/visualizers/activeRange';
  import type { FrequencyData } from '@shared/types/models';

  let canvas: HTMLCanvasElement;
  let rafId: number | null = null;
  let ro: ResizeObserver | null = null;

  const activeRange: ActiveRangeState = createActiveRange();

  let currentData: FrequencyData | null = null;
  $: if ($frequencyData) currentData = $frequencyData;

  const BAR_GAP = 2;
  const BAR_MIN_HEIGHT = 6;
  const ACCENT_COLOR = '#6366f1';
  /** Only draw bars whose magnitude exceeds this fraction of peak. */
  const ENERGY_THRESHOLD = 0.03;

  let cachedCtx: CanvasRenderingContext2D | null = null;

  function getCtx(): CanvasRenderingContext2D | null {
    if (cachedCtx) return cachedCtx;
    cachedCtx = canvas.getContext('2d', {
      alpha: false,
      desynchronized: true,
      willReadFrequently: false,
    }) as CanvasRenderingContext2D | null;
    return cachedCtx;
  }

  function handleResize(): void {
    if (!canvas) return;
    const parent = canvas.parentElement;
    if (!parent) return;
    let w = Math.floor(parent.clientWidth);
    let h = Math.floor(parent.clientHeight);
    if (w === 0 || h === 0) {
      const rect = parent.getBoundingClientRect();
      w = Math.floor(rect.width);
      h = Math.floor(rect.height);
    }
    if (w > 0 && h > 0) {
      canvas.width = w;
      canvas.height = h;
      cachedCtx = null;
    }
  }

  /**
   * Render bars — only bins with meaningful energy get drawn.
   * Active bars fill the entire width; silent bins are skipped entirely.
   */
  function renderActiveBars(
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    data: FrequencyData
  ): void {
    const { bins, peak } = data;
    if (!bins.length || peak <= 0 || width <= 0 || height <= 0) return;

    // Collect only bins with energy above threshold.
    const threshold = peak * ENERGY_THRESHOLD;
    const active: number[] = [];
    for (let i = 0; i < bins.length; i++) {
      if (bins[i] >= threshold) active.push(bins[i]);
    }
    if (!active.length) return;

    const barWidth = Math.max(1, (width - BAR_GAP * (active.length - 1)) / active.length);

    ctx.fillStyle = ACCENT_COLOR;
    for (let i = 0; i < active.length; i++) {
      const normalizedHeight = active[i] / peak;
      const shaped = Math.pow(normalizedHeight, 0.85);
      const barHeight = Math.max(BAR_MIN_HEIGHT, shaped * height * 0.9);
      const x = i * (barWidth + BAR_GAP);
      const y = height - barHeight;

      ctx.globalAlpha = 0.5 + shaped * 0.5;
      ctx.fillRect(x, y, barWidth, barHeight);
    }
    ctx.globalAlpha = 1;
  }

  function renderFrame(): void {
    if (!canvas) return;
    if (canvas.width === 0 || canvas.height === 0) {
      handleResize();
      if (canvas.width === 0 || canvas.height === 0) return;
    }
    const ctx = getCtx();
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const raw = currentData ?? { bins: new Float32Array(0), sampleRate: 44100, peak: 0 };
    const data = limitFrequencyRange(activeRange, raw, $currentTrack?.id);
    renderActiveBars(ctx, canvas.width, canvas.height, data);
  }

  onMount(() => {
    handleResize();
    const frame = (): void => {
      renderFrame();
      rafId = requestAnimationFrame(frame);
    };
    rafId = requestAnimationFrame(frame);

    const parent = canvas?.parentElement;
    if (parent && typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(() => handleResize());
      ro.observe(parent);
    }
  });

  onDestroy(() => {
    if (rafId !== null) cancelAnimationFrame(rafId);
    rafId = null;
    if (ro) {
      ro.disconnect();
      ro = null;
    }
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="nowplaying-viz" aria-hidden="true">
  <canvas bind:this={canvas} class="nowplaying-viz-canvas"></canvas>
</div>

<style>
  .nowplaying-viz {
    width: 100%;
    height: 160px;
    overflow: hidden;
  }

  .nowplaying-viz-canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
