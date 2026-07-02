/**
 * Radial visualizer renderer — spectrum bars arranged around a circle.
 *
 * Bins are grouped into a fixed number of radial segments and drawn as bars
 * extending outward from the center. Pure Canvas2D, single pass, no
 * allocations per frame beyond the path commands. Safe for the WebKitGTK
 * (JSC JIT disabled) dev runtime.
 *
 * Deliberately kept lighter than the old "radial geometry" mode that was
 * implicated in the WebKitGTK crash: no `shadowBlur`, no
 * `globalCompositeOperation: 'lighter'`, no per-frame WeakMap state. Just
 * `rotate` + `fillRect` in a loop.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/**
 * Render a radial spectrum.
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, gaps, min heights).
 */
export function renderRadial(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  if (!data || !data.bins.length) return;

  const { bins, peak } = data;
  const cx = width / 2;
  const cy = height / 2;
  // Inner radius is a fraction of the smaller dimension, leaving room for bars.
  const base = Math.min(width, height);
  const innerRadius = Math.max(8, base * 0.18);
  const maxBarLength = Math.max(4, base * 0.32);

  // Cap the segment count so very large FFT sizes don't draw hairline bars.
  const segments = Math.min(bins.length, 96);
  const groupSize = Math.ceil(bins.length / segments);
  const barGap = theme.barGap;
  const barMinHeight = theme.barMinHeight;
  // Angular width of each bar, leaving a small gap between bars.
  const angleStep = (Math.PI * 2) / segments;
  const barAngle = Math.max(0.02, angleStep - (barGap / innerRadius));

  ctx.save();
  ctx.translate(cx, cy);
  ctx.fillStyle = theme.accentColor;

  for (let i = 0; i < segments; i++) {
    let sum = 0;
    let count = 0;
    const groupStart = i * groupSize;
    const groupEnd = Math.min((i + 1) * groupSize, bins.length);
    for (let j = groupStart; j < groupEnd; j++) {
      sum += bins[j];
      count++;
    }
    const magnitude = count > 0 ? sum / count : 0;
    const normalized = peak > 0 ? magnitude / peak : 0;
    const barLength = Math.max(barMinHeight, normalized * maxBarLength);

    // Rotate to the bar's angle. Bars start at the top (-PI/2) and go clockwise.
    const angle = -Math.PI / 2 + i * angleStep;
    ctx.save();
    ctx.rotate(angle);
    ctx.globalAlpha = 0.6 + normalized * 0.4;
    // Draw a thin radial rectangle: x is along the radius, y straddles 0.
    const barThickness = Math.max(1, innerRadius * barAngle);
    ctx.fillRect(innerRadius, -barThickness / 2, barLength, barThickness);
    ctx.restore();
  }

  ctx.globalAlpha = 1;
  ctx.restore();
}