/**
 * Tunnel visualizer renderer — concentric rings expanding from the center.
 *
 * Draws a set of rings that grow outward from the canvas center, driven by
 * overall spectrum energy and a read-only time term (`performance.now()`).
 * No spawn queue, no particle list, no per-frame state machine: each ring's
 * radius is a pure function of `(energy, time, ringIndex)`, so the "tunnel"
 * illusion is produced by math alone and is fully deterministic. Safe for
 * the WebKitGTK (JSC JIT disabled) dev runtime.
 *
 * Deliberately avoids the patterns previously implicated in the WebKitGTK
 * crash: no `shadowBlur`, no `globalCompositeOperation: 'lighter'`, no
 * WeakMap state, no per-frame allocations beyond the stroke paths.
 */
import type { FrequencyData } from '@shared/types/models';
import type { VisualizerTheme } from './types';

/** Number of concurrent rings in the tunnel. Bounded for cheap frames. */
const RING_COUNT = 8;
/** Expansion cycle duration in ms. Controls how fast rings travel outward. */
const RING_CYCLE_MS = 1400;
/** Base ring thickness in pixels (scaled by energy). */
const BASE_STROKE = 2;

/**
 * Render concentric expanding rings (tunnel effect).
 *
 * @param ctx      Canvas 2D context (already cleared by the host).
 * @param width    Canvas pixel width.
 * @param height   Canvas pixel height.
 * @param data     Latest frequency data (bins + peak), may be empty while idle.
 * @param theme    Resolved theme tokens (accent color, gaps, min heights).
 */
export function renderTunnel(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
): void {
  // Overall energy: average of the low and mid bands for a "musical" drive.
  let energy = 0;
  if (data && data.bins.length) {
    const { bins, peak } = data;
    const n = bins.length;
    const lowEnd = Math.max(1, Math.floor(n / 3));
    const midEnd = Math.max(lowEnd + 1, Math.floor((2 * n) / 3));
    let sum = 0;
    for (let i = 0; i < midEnd; i++) sum += bins[i];
    energy = peak > 0 ? sum / midEnd / peak : 0;
    if (energy > 1) energy = 1;
  }

  const cx = width / 2;
  const cy = height / 2;
  // Max radius reaches just past the nearest corner so rings fade off-canvas.
  const maxRadius = Math.hypot(width, height) / 2;
  if (maxRadius <= 0) return;

  const now = performance.now();
  const phase = (now % RING_CYCLE_MS) / RING_CYCLE_MS; // 0..1

  ctx.save();
  ctx.strokeStyle = theme.accentColor;
  ctx.lineWidth = Math.max(1, BASE_STROKE + energy * 4);

  for (let i = 0; i < RING_COUNT; i++) {
    // Each ring is offset within the cycle so they form a continuous tunnel.
    const ringPhase = (phase + i / RING_COUNT) % 1;
    // Radius grows non-linearly for a perspective-like deepening.
    const radius = ringPhase * ringPhase * maxRadius;
    if (radius < 1) continue;

    // Alpha fades as the ring approaches the edge; newest ring is brightest.
    const alpha = (1 - ringPhase) * (0.25 + energy * 0.6);
    if (alpha <= 0) continue;

    ctx.globalAlpha = alpha;
    ctx.beginPath();
    ctx.arc(cx, cy, radius, 0, Math.PI * 2);
    ctx.stroke();
  }

  // Center pulse: a filled disc whose radius tracks energy, anchoring the
  // tunnel origin. Kept small and alpha-capped to avoid a hotspot.
  const coreRadius = Math.max(theme.barMinHeight, energy * Math.min(width, height) * 0.08);
  ctx.globalAlpha = 0.35 + energy * 0.4;
  ctx.fillStyle = theme.accentColor;
  ctx.beginPath();
  ctx.arc(cx, cy, coreRadius, 0, Math.PI * 2);
  ctx.fill();

  ctx.globalAlpha = 1;
  ctx.restore();
}