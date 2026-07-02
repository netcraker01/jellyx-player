/**
 * Aurora visualizer renderer — soft horizontal bands of color sweeping with
 * the spectrum.
 *
 * Draws a few stacked translucent gradient bands that shift vertically with
 * low/mid/high frequency energy. Pure Canvas2D, single pass, no
 * allocations per frame beyond the gradient objects. Safe for the WebKitGTK
 * (JSC JIT disabled) dev runtime.
 *
 * Deliberately kept lighter than the old "aurora" mode that was implicated in
 * the WebKitGTK crash: no `shadowBlur`, no `globalCompositeOperation: 'lighter'`,
 * no particle system. Just stacked `createLinearGradient` + `fillRect` bands
 * with alpha modulation.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/** Split the spectrum into three bands (low / mid / high) by bin index. */
function bandEnergy(bins: Float32Array, start: number, end: number): number {
  let sum = 0;
  let count = 0;
  for (let i = start; i < end; i++) {
    sum += bins[i];
    count++;
  }
  return count > 0 ? sum / count : 0;
}

/**
 * Render stacked aurora bands.
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, min heights).
 */
export function renderAurora(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  if (!data || !data.bins.length) return;

  const { bins, peak } = data;
  const n = bins.length;
  // Three equal bands across the spectrum.
  const lowEnd = Math.max(1, Math.floor(n / 3));
  const midEnd = Math.max(lowEnd + 1, Math.floor((2 * n) / 3));
  const low = bandEnergy(bins, 0, lowEnd);
  const mid = bandEnergy(bins, lowEnd, midEnd);
  const high = bandEnergy(bins, midEnd, n);
  const norm = (v: number) => (peak > 0 ? v / peak : 0);
  const lowN = norm(low);
  const midN = norm(mid);
  const highN = norm(high);

  const accent = theme.accentColor;
  const bandHeight = height / 3;

  ctx.save();

  // Low band — anchored to the bottom, rises with bass.
  const lowH = Math.max(theme.barMinHeight, lowN * bandHeight * 0.9);
  const lowTop = height - lowH;
  const lowGrad = ctx.createLinearGradient(0, lowTop, 0, height);
  lowGrad.addColorStop(0, 'rgba(0, 0, 0, 0)');
  lowGrad.addColorStop(1, accent);
  ctx.globalAlpha = 0.35 + lowN * 0.4;
  ctx.fillStyle = lowGrad;
  ctx.fillRect(0, lowTop, width, lowH);

  // Mid band — centered vertically, breathes with mids.
  const midH = Math.max(theme.barMinHeight, midN * bandHeight * 0.9);
  const midTop = height / 2 - midH / 2;
  const midGrad = ctx.createLinearGradient(0, midTop, 0, midTop + midH);
  midGrad.addColorStop(0, 'rgba(0, 0, 0, 0)');
  midGrad.addColorStop(0.5, accent);
  midGrad.addColorStop(1, 'rgba(0, 0, 0, 0)');
  ctx.globalAlpha = 0.25 + midN * 0.4;
  ctx.fillStyle = midGrad;
  ctx.fillRect(0, midTop, width, midH);

  // High band — anchored to the top, descends with treble.
  const highH = Math.max(theme.barMinHeight, highN * bandHeight * 0.9);
  const highGrad = ctx.createLinearGradient(0, 0, 0, highH);
  highGrad.addColorStop(0, accent);
  highGrad.addColorStop(1, 'rgba(0, 0, 0, 0)');
  ctx.globalAlpha = 0.2 + highN * 0.4;
  ctx.fillStyle = highGrad;
  ctx.fillRect(0, 0, width, highH);

  ctx.globalAlpha = 1;
  ctx.restore();
}