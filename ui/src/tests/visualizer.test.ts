/**
 * Visualizer data layer tests.
 *
 * Tests the FrequencyData type, binary FFT channel, and store layer
 * for the visualizer frontend (binary IPC migration).
 */
import { describe, it, expect } from 'vitest';
import type { FrequencyData } from '@shared/types/models';
import { createFftChannel } from '@services/events';
import { frequencyData, modoCineActive } from '@features/player/stores/player';

// ── FrequencyData type shape (binary mode) ────────────────

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

// ── Binary FFT channel ───────────────────────────────────

describe('createFftChannel', () => {
  it('is a function that returns Promise<UnlistenFn>', async () => {
    // Browser fallback: returns no-op when Tauri unavailable
    const result = await createFftChannel(() => {});
    expect(typeof result).toBe('function');
  });

  it('is exported as the replacement for onFrequencyData', () => {
    expect(typeof createFftChannel).toBe('function');
  });
});

// ── Binary frame decoding ────────────────────────────────

describe('binary frame decoding', () => {
  it('decodes a valid binary frame into FrequencyData', () => {
    // Simulate a binary frame: [sample_rate u32 LE][peak f32 LE][bins f32 LE]
    const sampleRate = 44100;
    const peak = 0.5;
    const bins = [0.1, 0.2, 0.3, 0.4, 0.5];
    const binCount = bins.length;

    const buffer = new ArrayBuffer(8 + binCount * 4);
    const view = new DataView(buffer);
    view.setUint32(0, sampleRate, true);   // little-endian
    view.setFloat32(4, peak, true);         // little-endian

    const binsArray = new Float32Array(buffer, 8);
    for (let i = 0; i < binCount; i++) {
      binsArray[i] = bins[i];
    }

    // Decode
    const decoded: FrequencyData = {
      bins: new Float32Array(buffer, 8),
      sampleRate: view.getUint32(0, true),
      peak: view.getFloat32(4, true),
    };

    expect(decoded.sampleRate).toBe(44100);
    expect(decoded.peak).toBeCloseTo(0.5);
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