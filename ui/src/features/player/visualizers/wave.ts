/**
 * Wave visualizer renderer — oscilloscope-style line across the canvas.
 *
 * Treats the FFT bins as a time-domain-ish waveform and strokes a smooth
 * horizontal line that deflects vertically with magnitude. Pure Canvas2D,
 * single pass, no allocations per frame beyond the path commands. Safe for
 * the WebKitGTK (JSC JIT disabled) dev runtime.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/**
 * Render a horizontal oscilloscope line.
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, stroke width).
 */
export function renderWave(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  if (!data || !data.bins.length) return;

  const { bins, peak } = data;
  const midY = height / 2;
  const n = bins.length;
  const stepX = width / Math.max(1, n - 1);

  // Soft glow underlay
  ctx.save();
  ctx.lineWidth = Math.max(1, theme.barMinHeight);
  ctx.strokeStyle = theme.accentColor;
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.globalAlpha = 0.25;
  ctx.shadowBlur = 12;
  ctx.shadowColor = theme.accentColor;

  ctx.beginPath();
  ctx.moveTo(0, midY);
  for (let i = 0; i < n; i++) {
    const magnitude = bins[i];
    const normalized = peak > 0 ? magnitude / peak : 0;
    const shaped = Math.pow(normalized, 0.85);
    // Map shaped 0..1 to ±0.45 * height around the midline.
    const y = midY - (shaped - 0.5) * 2 * (height * 0.45);
    ctx.lineTo(i * stepX, y);
  }
  ctx.lineTo(width, midY);
  ctx.stroke();

  // Crisp main line on top
  ctx.globalAlpha = 1;
  ctx.shadowBlur = 0;
  ctx.lineWidth = Math.max(1, theme.barMinHeight);
  ctx.beginPath();
  ctx.moveTo(0, midY);
  for (let i = 0; i < n; i++) {
    const magnitude = bins[i];
    const normalized = peak > 0 ? magnitude / peak : 0;
    const shaped = Math.pow(normalized, 0.85);
    const y = midY - (shaped - 0.5) * 2 * (height * 0.45);
    ctx.lineTo(i * stepX, y);
  }
  ctx.lineTo(width, midY);
  ctx.stroke();
  ctx.restore();
}