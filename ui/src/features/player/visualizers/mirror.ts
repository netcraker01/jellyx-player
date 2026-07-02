/**
 * Mirror visualizer renderer — bars mirrored from the horizontal center.
 *
 * Visually distinct from `bars` (which grow from the bottom): bars here grow
 * up and down symmetrically from the vertical midline, giving an
 * Winamp-style "reflection" look. Pure Canvas2D, single pass, no
 * allocations per frame. Safe for the WebKitGTK (JSC JIT disabled) dev runtime.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/**
 * Render bars mirrored from the horizontal center.
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, gaps, min heights).
 */
export function renderMirror(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  if (!data || !data.bins.length) return;

  const { bins, peak } = data;
  const maxBars = Math.min(bins.length, Math.max(1, Math.floor(width / 4)));
  const groupSize = Math.ceil(bins.length / maxBars);
  const barGap = theme.barGap;
  const barMinHeight = theme.barMinHeight;
  const barWidth = Math.max(1, (width - barGap * (maxBars - 1)) / maxBars);
  const midY = height / 2;

  ctx.fillStyle = theme.accentColor;

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
    const halfBar = Math.max(barMinHeight / 2, (normalizedHeight * height * 0.42));

    const x = i * (barWidth + barGap);

    ctx.globalAlpha = 0.6 + normalizedHeight * 0.4;
    // Upper half (grows upward from the center)
    ctx.fillRect(x, midY - halfBar, barWidth, halfBar);
    // Lower half (mirrored, grows downward)
    ctx.fillRect(x, midY, barWidth, halfBar);
  }

  ctx.globalAlpha = 1;
}