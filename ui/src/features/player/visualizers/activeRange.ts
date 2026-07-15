/**
 * Active-range detection for frequency data.
 *
 * Truncates the FFT bins to only the range where the track has meaningful
 * energy, so visualizers don't waste canvas width on bars that sit at zero.
 * The cutoff grows to accommodate higher frequencies but never shrinks within
 * the same track — it resets on track change.
 *
 * Each visualizer must create its own instance via `createActiveRange()` so
 * the cutoff state is independent per component.
 */
import type { FrequencyData } from '@shared/types/models';

/** Highest frequency to consider (Hz). */
const MAX_VISUAL_FREQ_HZ = 8_000;

/** Minimum fraction of the peak magnitude for a bin to count as "active". */
const ACTIVE_THRESHOLD = 0.015;

/** Extra bins to keep after the last active bin. */
const ACTIVE_BUFFER_BINS = 4;

/** Minimum bins to always keep. */
const MIN_BINS = 8;

export interface ActiveRangeState {
  stableCutoff: number;
  lastTrackId: string | null;
  initialized: boolean;
}

/**
 * Create an independent active-range instance with its own cutoff state.
 * Call once per visualizer component.
 */
export function createActiveRange(): ActiveRangeState {
  return { stableCutoff: 0, lastTrackId: null, initialized: false };
}

/**
 * Truncate frequency data to the active range of the current track.
 *
 * @param state    Mutable state for this instance (stableCutoff, lastTrackId).
 * @param data     Raw FFT frequency data.
 * @param trackId  Optional track identifier — passing a new value resets the cutoff.
 * @returns        FrequencyData with bins truncated to the active range.
 */
export function limitFrequencyRange(
  state: ActiveRangeState,
  data: FrequencyData,
  trackId?: string
): FrequencyData {
  // Reset cutoff on track change. On first call (initialized=false), always
  // calculate so the cutoff is set even if trackId is undefined.
  if (!state.initialized || trackId !== state.lastTrackId) {
    state.lastTrackId = trackId ?? null;
    state.stableCutoff = 0;
    state.initialized = true;
  }

  const nyquist = data.sampleRate > 0 ? data.sampleRate / 2 : 22_050;
  const cappedHz = Math.min(MAX_VISUAL_FREQ_HZ, nyquist);
  const maxIndex = Math.max(
    1,
    Math.min(
      data.bins.length,
      Math.ceil((cappedHz / nyquist) * data.bins.length)
    )
  );

  // Find peak and the last bin with meaningful energy.
  let peak = 0;
  let lastActiveBin = 0;
  for (let i = 0; i < maxIndex; i++) {
    if (data.bins[i] > peak) peak = data.bins[i];
  }
  const threshold = peak * ACTIVE_THRESHOLD;
  for (let i = 0; i < maxIndex; i++) {
    if (data.bins[i] >= threshold) lastActiveBin = i;
  }

  const target = Math.max(MIN_BINS, lastActiveBin + ACTIVE_BUFFER_BINS);
  const clamped = Math.min(maxIndex, target);

  // Grow only — never shrink within the same track.
  if (clamped > state.stableCutoff) {
    state.stableCutoff = clamped;
  }

  const bins = data.bins.subarray(0, state.stableCutoff);

  return {
    bins,
    sampleRate: data.sampleRate,
    peak,
  };
}
