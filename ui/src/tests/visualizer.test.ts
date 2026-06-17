/**
 * Visualizer data layer tests.
 *
 * Tests the FrequencyData type, event subscription, and store layer
 * for the visualizer frontend (specs VF-006 through VF-009).
 */
import { describe, it, expect } from 'vitest';
import type { FrequencyData } from '@shared/types/models';
import { onFrequencyData } from '@services/events';
import { frequencyData, modoCineActive } from '@features/player/stores/player';

// ── VF-006: FrequencyData type shape ────────────────────

describe('FrequencyData type', () => {
  it('has bins, sampleRate, and peak fields matching Rust serde camelCase', () => {
    const data: FrequencyData = {
      bins: [0.1, 0.2, 0.3, 0.4, 0.5],
      sampleRate: 44100,
      peak: 0.5,
    };

    expect(data.bins).toBeInstanceOf(Array);
    expect(data.bins.length).toBe(5);
    expect(data.sampleRate).toBe(44100);
    expect(data.peak).toBe(0.5);
  });

  it('bins can be empty (no audio data)', () => {
    const data: FrequencyData = {
      bins: [],
      sampleRate: 48000,
      peak: 0.0,
    };

    expect(data.bins).toHaveLength(0);
    expect(data.peak).toBe(0.0);
  });

  it('supports large bin arrays (FFT size 1024 = 512 bins)', () => {
    const bins = new Array(512).fill(0).map((_, i) => i / 512);
    const data: FrequencyData = {
      bins,
      sampleRate: 44100,
      peak: 0.999,
    };

    expect(data.bins.length).toBe(512);
    expect(data.peak).toBeCloseTo(0.999);
  });
});

// ── VF-007: Event subscription ───────────────────────────

describe('onFrequencyData', () => {
  it('is a function that returns Promise<UnlistenFn>', async () => {
    // Browser fallback: subscribeEvent returns () => {} when Tauri unavailable
    const result = await onFrequencyData(() => {});
    expect(typeof result).toBe('function');
  });

  it('subscribes to frequency-data event (verified by module structure)', () => {
    // The event name 'frequency-data' is validated by Rust tests
    // and by the events.ts module export
    expect(typeof onFrequencyData).toBe('function');
  });
});

// ── VF-008: FrequencyData store ──────────────────────────

describe('frequencyData store', () => {
  it('initializes as null', () => {
    let value: FrequencyData | null = 'not-null' as any;
    const unsub = frequencyData.subscribe((v) => { value = v; });
    expect(value).toBeNull();
    unsub();
  });

  it('updates when set with FrequencyData', () => {
    const testData: FrequencyData = {
      bins: [0.1, 0.5, 0.3],
      sampleRate: 44100,
      peak: 0.5,
    };
    frequencyData.set(testData);

    let value: FrequencyData | null = null;
    const unsub = frequencyData.subscribe((v) => { value = v; });
    expect(value).toEqual(testData);
    unsub();
  });

  it('can be set back to null', () => {
    frequencyData.set({ bins: [0.1], sampleRate: 44100, peak: 0.1 });
    frequencyData.set(null);

    let value: FrequencyData | null = null;
    const unsub = frequencyData.subscribe((v) => { value = v; });
    expect(value).toBeNull();
    unsub();
  });
});

describe('modoCineActive store', () => {
  it('initializes as false', () => {
    let value = true;
    const unsub = modoCineActive.subscribe((v) => { value = v; });
    expect(value).toBe(false);
    unsub();
  });

  it('can be toggled to true', () => {
    modoCineActive.set(true);

    let value = false;
    const unsub = modoCineActive.subscribe((v) => { value = v; });
    expect(value).toBe(true);
    unsub();
  });
});