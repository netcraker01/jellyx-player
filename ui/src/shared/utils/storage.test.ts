import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  canonicalKey,
  legacyKey,
  getMigratedItem,
  setMigratedItem,
  removeMigratedItem,
} from './storage';

function installLocalStorage() {
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
  });
  return values;
}

describe('storage migration helpers', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('builds canonical and legacy key names', () => {
    expect(canonicalKey('volume')).toBe('jellyx-volume');
    expect(legacyKey('volume')).toBe('helix-volume');
  });

  it('prefers the canonical key over the legacy key', () => {
    installLocalStorage();
    setMigratedItem('volume', '80');
    localStorage.setItem('helix-volume', '40');

    expect(getMigratedItem('volume')).toBe('80');
  });

  it('falls back to the legacy key when canonical is missing', () => {
    installLocalStorage();
    localStorage.setItem('helix-volume', '55');

    expect(getMigratedItem('volume')).toBe('55');
  });

  it('returns null when both keys are missing', () => {
    installLocalStorage();
    expect(getMigratedItem('visualizer-mode')).toBeNull();
  });

  it('writes new values to the canonical key only', () => {
    installLocalStorage();
    setMigratedItem('locale', 'es');

    expect(localStorage.getItem('jellyx-locale')).toBe('es');
    expect(localStorage.getItem('helix-locale')).toBeNull();
  });

  it('removes only the canonical key, leaving legacy intact', () => {
    installLocalStorage();
    setMigratedItem('volume', '80');
    localStorage.setItem('helix-volume', '40');

    removeMigratedItem('volume');

    expect(localStorage.getItem('jellyx-volume')).toBeNull();
    expect(localStorage.getItem('helix-volume')).toBe('40');
  });

  it('returns null and does not throw when localStorage is unavailable', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('Storage disabled');
      },
      setItem: () => {
        throw new Error('Storage disabled');
      },
      removeItem: () => {
        throw new Error('Storage disabled');
      },
    });

    expect(getMigratedItem('volume')).toBeNull();
    expect(() => setMigratedItem('volume', '80')).not.toThrow();
    expect(() => removeMigratedItem('volume')).not.toThrow();
  });

  it('supports all PR 3 persisted preference suffixes', () => {
    installLocalStorage();
    const suffixes = [
      'volume',
      'locale',
      'visualizer-mode',
      'cinematic-mode',
      'cinematic-intensity',
      'mini-player-skin',
      'mini-player-scale',
      'hide-title-bar',
    ];

    for (const suffix of suffixes) {
      setMigratedItem(suffix, String(Math.random()));
      expect(localStorage.getItem(`jellyx-${suffix}`)).not.toBeNull();
    }
  });
});
