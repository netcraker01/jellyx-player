/**
 * Grid visualizer renderer — a matrix of cells lit by spectrum energy.
 *
 * Maps the spectrum onto a 2D grid: columns correspond to frequency bin
 * groups (low → high, left → right) and rows correspond to amplitude tiers.
 * Each cell's brightness reflects how strongly its bin group exceeds the
 * row's amplitude threshold, producing a heatmap-style "equalizer grid"
 * that is visually distinct from the linear bar / radial / aurora modes.
 *
 * Pure Canvas2D, single pass, no allocations per frame beyond the cell
 * coordinates. No `shadowBlur`, no `globalCompositeOperation`, no particle
 * system, no per-frame state machine. Safe for the WebKitGTK (JSC JIT
 * disabled) dev runtime.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/** Fixed grid geometry. Kept small so cell count stays bounded on any canvas. */
const GRID_COLUMNS = 24;
const GRID_ROWS = 12;

/**
 * Render a spectrum heatmap grid.
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, gaps, min heights).
 */
export function renderGrid(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  if (!data || !data.bins.length) return;

  const { bins, peak } = data;
  const columns = Math.min(GRID_COLUMNS, bins.length);
  const groupSize = Math.ceil(bins.length / columns);

  const gap = Math.max(1, theme.barGap);
  const cellW = Math.max(1, (width - gap * (columns + 1)) / columns);
  const cellH = Math.max(1, (height - gap * (GRID_ROWS + 1)) / GRID_ROWS);

  ctx.save();
  ctx.fillStyle = theme.accentColor;

  for (let col = 0; col < columns; col++) {
    // Average magnitude for this column's bin group.
    let sum = 0;
    let count = 0;
    const groupStart = col * groupSize;
    const groupEnd = Math.min((col + 1) * groupSize, bins.length);
    for (let j = groupStart; j < groupEnd; j++) {
      sum += bins[j];
      count++;
    }
    const magnitude = count > 0 ? sum / count : 0;
    const normalized = peak > 0 ? magnitude / peak : 0;
    const shaped = Math.pow(normalized, 0.85);

    // Number of lit rows from the bottom, based on shaped energy.
    const litRows = Math.min(GRID_ROWS, Math.floor(shaped * GRID_ROWS + 0.5));
    const x = gap + col * (cellW + gap);

    for (let row = 0; row < GRID_ROWS; row++) {
      // Row 0 is the top; convert to "rows from the bottom".
      const fromBottom = GRID_ROWS - 1 - row;
      const lit = fromBottom < litRows;
      // Distance of this row from the lit front, used for falloff alpha.
      const distance = fromBottom - (litRows - 1);
      const y = gap + row * (cellH + gap);

      if (lit) {
        // Brightest at the lit front (distance 0), fading upward.
        const alpha = distance === 0 ? 0.95 : Math.max(0.15, 0.95 - distance * 0.25);
        ctx.globalAlpha = alpha;
        ctx.fillRect(x, y, cellW, cellH);
      } else {
        // Dim base grid so the matrix is always visible (idle state too).
        ctx.globalAlpha = 0.06;
        ctx.fillRect(x, y, cellW, cellH);
      }
    }
  }

  ctx.globalAlpha = 1;
  ctx.restore();
}