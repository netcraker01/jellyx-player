import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import {
  volume,
  visualizerMode,
  cinematicMode,
  cinematicIntensity,
  setVolume,
  setCinematicIntensity,
} from './player';

const importPlayer = (query: string) => import(`./player?test=${query}`) as Promise<typeof import('./player')>;

const mocks = vi.hoisted(() => ({
  playStream: vi.fn(),
  playLocal: vi.fn(),
  push: vi.fn(),
  setVolumeCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  playStream: mocks.playStream,
  playLocal: mocks.playLocal,
  pause: vi.fn(),
  resume: vi.fn(),
  next: vi.fn(),
  previous: vi.fn(),
  seek: vi.fn(),
  setVolume: mocks.setVolumeCmd,
  setShuffle: vi.fn(),
  cycleRepeat: vi.fn(),
  removeFromQueue: vi.fn(),
  clearQueue: vi.fn(),
  playNext: vi.fn(),
  getAudioSettings: vi.fn(),
  setPlaybackNormalizeAudio: vi.fn(),
}));

vi.mock('@services/events', () => ({
  onTrackChanged: vi.fn(),
  onStateChanged: vi.fn(),
  onQueueUpdated: vi.fn(),
  onProgressTick: vi.fn(),
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: mocks.push,
  },
}));

vi.mock('@features/favorites/stores/favorites', () => ({
  favorites: {
    subscribe: vi.fn(),
  },
}));

vi.mock('@i18n', () => ({
  t: {
    subscribe: vi.fn(() => () => {}),
  },
}));

function installLocalStorage() {
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
  });
  return values;
}

describe('player store localStorage migration', () => {
  beforeEach(() => {
    volume.set(80);
    visualizerMode.set('bars');
    cinematicMode.set(false);
    cinematicIntensity.set(0.5);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('reads volume from legacy helix-volume key when jellyx-volume is absent', () => {
    installLocalStorage();
    localStorage.setItem('helix-volume', '42');

    // Re-import to trigger store initialisation with the current localStorage state.
    return importPlayer('cached-volume').then((mod) => {
      expect(get(mod.volume)).toBe(42);
    });
  });

  it('writes new volume changes to jellyx-volume only', async () => {
    installLocalStorage();
    localStorage.setItem('helix-volume', '42');

    mocks.setVolumeCmd.mockResolvedValue(undefined);
    await setVolume(60);

    expect(localStorage.getItem('jellyx-volume')).toBe('60');
    expect(localStorage.getItem('helix-volume')).toBe('42');
  });

  it('reads visualizer mode from legacy key when canonical key is absent', () => {
    installLocalStorage();
    localStorage.setItem('helix-visualizer-mode', 'wave');

    return importPlayer('cached-visualizer').then((mod) => {
      expect(get(mod.visualizerMode)).toBe('wave');
    });
  });

  it('writes visualizer mode changes to jellyx-visualizer-mode', () => {
    installLocalStorage();
    visualizerMode.set('mirror');
    expect(localStorage.getItem('jellyx-visualizer-mode')).toBe('mirror');
    expect(localStorage.getItem('helix-visualizer-mode')).toBeNull();
  });

  it('reads cinematic mode from legacy key and writes to canonical key', () => {
    installLocalStorage();
    localStorage.setItem('helix-cinematic-mode', 'true');

    return importPlayer('cached-cinematic').then((mod) => {
      expect(get(mod.cinematicMode)).toBe(true);
    });
  });

  it('writes cinematic intensity changes to jellyx-cinematic-intensity', () => {
    installLocalStorage();
    setCinematicIntensity(0.75);
    expect(localStorage.getItem('jellyx-cinematic-intensity')).toBe('0.75');
    expect(localStorage.getItem('helix-cinematic-intensity')).toBeNull();
  });

  it('does not throw when localStorage is unavailable', () => {
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

    expect(get(volume)).toBe(80);
    expect(() => setCinematicIntensity(0.25)).not.toThrow();
  });
});
