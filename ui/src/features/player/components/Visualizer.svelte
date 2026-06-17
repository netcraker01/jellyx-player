<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { frequencyData, modoCineActive } from '../stores/player';
  import { createFftChannel } from '@services/events';
  import type { FrequencyData } from '@shared/types/models';

  let canvas: HTMLCanvasElement;
  let rafId: number | null = null;
  let unlisten: (() => void) | null = null;

  // Local reference to frequency data for rAF loop (avoids reactive dependency in animation)
  let currentData: FrequencyData | null = null;

  // Subscribe to frequency data and keep local reference for rAF loop
  $: if ($frequencyData) {
    currentData = $frequencyData;
  }

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

  function renderFrame(): void {
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    if (!currentData || !currentData.bins.length) {
      return;
    }

    const { bins, peak } = currentData;
    const barGap = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--viz-bar-gap')) || 2;
    const barMinHeight = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--viz-bar-min-height')) || 2;
    const accentColor = getComputedStyle(document.documentElement).getPropertyValue('--viz-color-accent').trim() || '#6366f1';

    // Use a reasonable number of bars by grouping bins if needed
    const maxBars = Math.min(bins.length, Math.floor(width / 4));
    const groupSize = Math.ceil(bins.length / maxBars);
    const barWidth = Math.max(1, (width - barGap * (maxBars - 1)) / maxBars);

    for (let i = 0; i < maxBars; i++) {
      // Average the bins in this group — iterate Float32Array directly
      let sum = 0;
      let count = 0;
      const groupStart = i * groupSize;
      const groupEnd = Math.min((i + 1) * groupSize, bins.length);
      for (let j = groupStart; j < groupEnd; j++) {
        sum += bins[j];
        count++;
      }
      const magnitude = count > 0 ? sum / count : 0;

      // Normalize bar height relative to peak
      const normalizedHeight = peak > 0 ? magnitude / peak : 0;
      const barHeight = Math.max(barMinHeight, normalizedHeight * height * 0.85);

      const x = i * (barWidth + barGap);
      const y = height - barHeight;

      ctx.fillStyle = accentColor;
      ctx.globalAlpha = 0.6 + normalizedHeight * 0.4;
      ctx.fillRect(x, y, barWidth, barHeight);
    }

    ctx.globalAlpha = 1;
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
  });

  $: if ($modoCineActive) {
    // Resize canvas for fullscreen cinema mode
    setTimeout(() => {
      if (canvas) {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
      }
    }, 0);
  } else {
    // Resize to parent container
    setTimeout(handleResize, 0);
  }
</script>

<div class="visualizer" class:modo-cine={$modoCineActive}>
  <canvas bind:this={canvas} class="visualizer-canvas"></canvas>
  {#if $modoCineActive}
    <button class="modo-cine-close" aria-label="Exit fullscreen" on:click={() => modoCineActive.set(false)}>
      ✕
    </button>
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
    background: var(--bg-base, #0a0a0f);
  }

  .visualizer-canvas {
    display: block;
    width: 100%;
    height: 100%;
  }

  .modo-cine-close {
    position: absolute;
    top: 1rem;
    right: 1rem;
    z-index: 101;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: var(--text-primary, #e0e0e0);
    font-size: 1.25rem;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;
  }

  .modo-cine-close:hover {
    background: rgba(255, 255, 255, 0.2);
  }
</style>