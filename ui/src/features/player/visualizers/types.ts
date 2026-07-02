/**
 * Shared types for visualizer renderers and the registry.
 *
 * Renderers are pure Canvas2D functions — they receive everything they need
 * as arguments and hold no state. This keeps them safe to hot-swap from the
 * single requestAnimationFrame loop in `Visualizer.svelte` and easy to unit
 * test with a stub 2D context.
 */
import type { FrequencyData } from '@shared/types/models';

/** Theme tokens resolved from CSS custom properties by the host. */
export interface VisualizerTheme {
  /** Accent color (CSS var `--viz-color-accent`). */
  accentColor: string;
  /** Gap between bars in pixels (CSS var `--viz-bar-gap`). */
  barGap: number;
  /** Minimum bar height in pixels (CSS var `--viz-bar-min-height`). */
  barMinHeight: number;
}

/** Signature every renderer must implement. */
export type VisualizerRenderer = (
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: FrequencyData | null,
  theme: VisualizerTheme
) => void;