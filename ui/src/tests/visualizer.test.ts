/**
 * Visualizer data layer tests.
 *
 * Tests the FrequencyData type, FFT event payload conversion, and store layer.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { FrequencyData } from '@shared/types/models';
import { frequencyDataFromFftPayload, onFftFrame } from '@services/events';
import { frequencyData, modoCineActive } from '@features/player/stores/player';

// ── FrequencyData type shape ──────────────────────────────

describe('FrequencyData type', () => {
  it('has bins as Float32Array, sampleRate, and peak fields', () => {
    const data: FrequencyData = {
      bins: new Float32Array([0.1, 0.2, 0.3, 0.4, 0.5]),
      sampleRate: 44100,
      peak: 0.5,
    };

    expect(data.bins).toBeInstanceOf(Float32Array);
    expect(data.bins.length).toBe(5);
    expect(data.sampleRate).toBe(44100);
    expect(data.peak).toBe(0.5);
  });

  it('bins can be empty (no audio data)', () => {
    const data: FrequencyData = {
      bins: new Float32Array(0),
      sampleRate: 48000,
      peak: 0.0,
    };

    expect(data.bins).toHaveLength(0);
    expect(data.peak).toBe(0.0);
  });

  it('supports large bin arrays (FFT size 1024 = 512 bins)', () => {
    const bins = new Float32Array(512);
    for (let i = 0; i < 512; i++) {
      bins[i] = i / 512;
    }
    const data: FrequencyData = {
      bins,
      sampleRate: 44100,
      peak: 0.999,
    };

    expect(data.bins.length).toBe(512);
    expect(data.peak).toBeCloseTo(0.999);
  });

  it('bins values can be iterated directly without conversion', () => {
    const data: FrequencyData = {
      bins: new Float32Array([0.1, 0.5, 0.3]),
      sampleRate: 44100,
      peak: 0.5,
    };

    let sum = 0;
    for (let i = 0; i < data.bins.length; i++) {
      sum += data.bins[i];
    }
    expect(sum).toBeCloseTo(0.9, 5);
  });
});

// ── FFT event listener ───────────────────────────────────

describe('onFftFrame', () => {
  it('is a function that returns Promise<UnlistenFn>', async () => {
    // Browser fallback: returns no-op when Tauri unavailable
    const result = await onFftFrame(() => {});
    expect(typeof result).toBe('function');
  });

  it('is exported as the replacement for onFrequencyData', () => {
    expect(typeof onFftFrame).toBe('function');
  });
});

// ── FFT event payload conversion ──────────────────────────

describe('FFT event payload conversion', () => {
  it('converts the JSON event payload into FrequencyData', () => {
    const decoded = frequencyDataFromFftPayload({
      bins: [0.1, 0.2, 0.3, 0.4, 0.5],
      sampleRate: 44100,
      peak: 0.5,
    });

    expect(decoded.sampleRate).toBe(44100);
    expect(decoded.peak).toBeCloseTo(0.5);
    expect(decoded.bins).toBeInstanceOf(Float32Array);
    expect(decoded.bins.length).toBe(5);
    expect(decoded.bins[0]).toBeCloseTo(0.1, 5);
    expect(decoded.bins[4]).toBeCloseTo(0.5, 5);
  });
});

// ── FrequencyData store ──────────────────────────────────

describe('frequencyData store', () => {
  it('initializes as null', () => {
    let value: FrequencyData | null = 'not-null' as any;
    const unsub = frequencyData.subscribe((v) => { value = v; });
    expect(value).toBeNull();
    unsub();
  });

  it('updates when set with FrequencyData (Float32Array bins)', () => {
    const testData: FrequencyData = {
      bins: new Float32Array([0.1, 0.5, 0.3]),
      sampleRate: 44100,
      peak: 0.5,
    };
    frequencyData.set(testData);

    let value: FrequencyData | null = null;
    const unsub = frequencyData.subscribe((v) => { value = v; });
    expect(value).not.toBeNull();
    expect(value!.bins).toBeInstanceOf(Float32Array);
    expect(value!.bins.length).toBe(3);
    expect(value!.sampleRate).toBe(44100);
    unsub();
  });

  it('can be set back to null', () => {
    frequencyData.set({ bins: new Float32Array([0.1]), sampleRate: 44100, peak: 0.1 });
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

describe('visualizer FFT ownership', () => {
  it('keeps both visualizer components as pure frequencyData consumers', () => {
    const visualizer = readFileSync(resolve(process.cwd(), 'src/features/player/components/Visualizer.svelte'), 'utf8');
    const miniVisualizer = readFileSync(resolve(process.cwd(), 'src/features/mini-player/MiniVisualizer.svelte'), 'utf8');

    expect(visualizer).not.toContain('onFftFrame');
    expect(miniVisualizer).not.toContain('onFftFrame');
    expect(visualizer).not.toContain('start_fft_stream');
    expect(miniVisualizer).not.toContain('start_fft_stream');
  });
});
