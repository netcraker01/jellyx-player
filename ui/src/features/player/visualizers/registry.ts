/**
 * Visualizer registry — the single source of truth for available modes.
 *
 * Each entry pairs a stable mode id with its renderer function and i18n key.
 * The host (`Visualizer.svelte`) reads `currentMode` from the persisted store,
 * looks up the renderer here, and calls it from its one requestAnimationFrame
 * loop. This keeps mode switching a pure dispatch — no Svelte effects per
 * mode, no rAF churn, no DOM mutation beyond redrawing the same canvas.
 *
 * Adding a mode later:
 *   1. Drop a renderer file in `visualizers/` implementing `VisualizerRenderer`.
 *   2. Register it here with a stable `id`, `labelKey`, and `icon` (optional).
 *   3. Add the i18n label under `visualizer.modes.*` in en.json / es.json.
 *
 * The initial safe batch is deliberately conservative: pure Canvas2D
 * renderers. The heavier modes that were previously implicated in WebKitGTK
 * JSC stack overflows (particles, blobs, streaks, constellation, pulseRing,
 * lavaLamp) still stay out until re-introduced one at a time in a follow-up
 * batch. `radial` and `aurora` were re-introduced as stripped-down,
 * allocation-light versions of their old selves: no `shadowBlur`, no
 * `globalCompositeOperation: 'lighter'`, no per-frame WeakMap state machines.
 *
 * `grid` and `tunnel` were added in the second safe batch as stateless
 * pure-Canvas2D renderers: `grid` is a spectrum heatmap matrix, `tunnel` is
 * concentric rings driven by a read-only time term (no spawn queue / particle
 * list). Neither holds per-frame state, avoiding the WeakMap state-machine
 * pattern that contributed to the earlier WebKitGTK crash.
 */
import type { VisualizerRenderer } from './types';
import { renderBars } from './bars';
import { renderWave } from './wave';
import { renderMirror } from './mirror';
import { renderRadial } from './radial';
import { renderAurora } from './aurora';
import { renderGrid } from './grid';
import { renderTunnel } from './tunnel';

/** Stable mode ids. Persisted to localStorage, so never reuse a removed id. */
export const VisualizerMode = {
  Bars: 'bars',
  Wave: 'wave',
  Mirror: 'mirror',
  Radial: 'radial',
  Aurora: 'aurora',
  Grid: 'grid',
  Tunnel: 'tunnel',
} as const;

export type VisualizerModeId = (typeof VisualizerMode)[keyof typeof VisualizerMode];

/** A registry entry describing one selectable visualizer. */
export interface VisualizerModeEntry {
  /** Stable id persisted to localStorage. */
  id: VisualizerModeId;
  /** i18n key under `visualizer.modes.*`. */
  labelKey: string;
  /** Renderer called once per animation frame by the host. */
  render: VisualizerRenderer;
}

/** Ordered list of registered modes (drives selector order). */
export const VISUALIZER_MODES: readonly VisualizerModeEntry[] = [
  { id: VisualizerMode.Bars, labelKey: 'visualizer.modes.bars', render: renderBars },
  { id: VisualizerMode.Wave, labelKey: 'visualizer.modes.wave', render: renderWave },
  { id: VisualizerMode.Mirror, labelKey: 'visualizer.modes.mirror', render: renderMirror },
  { id: VisualizerMode.Radial, labelKey: 'visualizer.modes.radial', render: renderRadial },
  { id: VisualizerMode.Aurora, labelKey: 'visualizer.modes.aurora', render: renderAurora },
  { id: VisualizerMode.Grid, labelKey: 'visualizer.modes.grid', render: renderGrid },
  { id: VisualizerMode.Tunnel, labelKey: 'visualizer.modes.tunnel', render: renderTunnel },
];

/** Default mode when no persisted preference exists or the stored one is unknown. */
export const DEFAULT_VISUALIZER_MODE: VisualizerModeId = VisualizerMode.Bars;

/**
 * Resolve a persisted mode id to a registry entry, falling back to the
 * default mode if the id is unknown. Keeps the persisted store small and
 * forward-compatible: deleted modes degrade gracefully to bars.
 */
export function resolveVisualizerMode(id: string | null | undefined): VisualizerModeEntry {
  if (id) {
    const found = VISUALIZER_MODES.find((m) => m.id === id);
    if (found) return found;
  }
  return VISUALIZER_MODES[0];
}