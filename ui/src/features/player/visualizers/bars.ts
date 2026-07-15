/**
 * Bars visualizer renderer — classic spectrum bars.
 *
 * Pure Canvas2D function. Called once per animation frame from
 * `Visualizer.svelte`'s single requestAnimationFrame loop. Has no Svelte
 * reactivity, no DOM listeners, and no GPU/shader work — safe for the
 * WebKitGTK (JSC JIT disabled) dev runtime.
 *
 * Grouping: when the FFT bin count exceeds the available bar slots, bins are
 * averaged per group so the visual stays consistent across track/sample-rate
 * changes without reallocating buffers.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/**
 * Render a left-to-right bar spectrum.
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, gaps, min heights).
 */
export function renderBars(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  const barGap = theme.barGap;
  const barMinHeight = theme.barMinHeight;
  ctx.fillStyle = theme.accentColor;

  // Idle / no-data fallback: draw a small static bar pattern so the canvas
  // never looks completely empty.
  if (!data || !data.bins.length) {
    const maxBars = Math.max(1, Math.floor(width / 3));
    const barWidth = Math.max(1, (width - barGap * (maxBars - 1)) / maxBars);
    ctx.globalAlpha = 0.5;
    for (let i = 0; i < maxBars; i++) {
      const h = Math.max(barMinHeight, Math.min(2, height * 0.12));
      const x = i * (barWidth + barGap);
      ctx.fillRect(x, height - h, barWidth, h);
    }
    ctx.globalAlpha = 1;
    return;
  }

  const { bins, peak } = data;
  const maxBars = Math.min(bins.length, Math.max(1, Math.floor(width / 4)));
  const groupSize = Math.ceil(bins.length / maxBars);
  const barWidth = Math.max(1, (width - barGap * (maxBars - 1)) / maxBars);

  for (let i = 0; i < maxBars; i++) {
    let sum = 0;
    let count = 0;
    const groupStart = i * groupSize;
    const groupEnd = Math.min((i + 1) * groupSize, bins.length);
    for (let j = groupStart; j < groupEnd; j++) {
      sum += bins[j];
      count++;
    }
    const magnitude = count > 0 ? sum / count : 0;
    const normalizedHeight = peak > 0 ? magnitude / peak : 0;
    const shaped = Math.pow(normalizedHeight, 0.85);
    const barHeight = Math.max(barMinHeight, shaped * height * 0.9);

    const x = i * (barWidth + barGap);
    const y = height - barHeight;

    ctx.globalAlpha = 0.5 + shaped * 0.5;
    ctx.fillRect(x, y, barWidth, barHeight);
  }

  ctx.globalAlpha = 1;
}