import { describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  activateMiniPlayerSkin,
  clampMiniPlayerScale,
  DEFAULT_MINI_PLAYER_SKIN,
  miniPlayerScale,
  MINI_PLAYER_SKINS,
  resolveMiniPlayerSkin,
  resolveMiniPlayerWindowSize,
  selectedMiniPlayerSkinId,
  setMiniPlayerScale,
} from './skins';

function installLocalStorage() {
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
}

describe('mini player skins', () => {
  it('ships an iPod-like default skin', () => {
    const skin = resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN);
    expect(skin.id).toBe('ipod-classic');
    expect(skin.window).toMatchObject({ width: 320, height: 480, resizable: false });
    expect(skin.layout.controls).toEqual(['previous', 'playPause', 'next']);
  });

  it('ships only declarative skins with sizing contracts', () => {
    expect(MINI_PLAYER_SKINS.length).toBeGreaterThan(1);
    expect(MINI_PLAYER_SKINS.map((skin) => skin.id)).toEqual(expect.arrayContaining(['ipod-classic', 'winamp-classic']));
    expect(MINI_PLAYER_SKINS.every((skin) => skin.window.width > 0 && skin.window.height > 0)).toBe(true);
    expect(MINI_PLAYER_SKINS.every((skin) => !('script' in skin))).toBe(true);
  });

  it('ships a declarative Classic horizontal hi-fi skin contract', () => {
    const skin = resolveMiniPlayerSkin('winamp-classic');

    expect(skin).toMatchObject({
      id: 'winamp-classic',
      name: 'Classic',
      kind: 'classic',
      shape: 'rounded-rectangle',
      window: { width: 400, height: 100, resizable: false },
    });
    expect(skin.theme.screenText).toBe('#ffd166');
    expect(skin.layout.controls).toEqual(['previous', 'playPause', 'next']);
  });

  it('falls back to the default skin for unknown ids', () => {
    expect(resolveMiniPlayerSkin('community-scripted-skin').id).toBe(DEFAULT_MINI_PLAYER_SKIN);
  });

  it('clamps declarative window dimensions before native resize', () => {
    const skin = {
      ...resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN),
      window: { width: 9999, height: Number.NaN, resizable: false },
    };

    expect(resolveMiniPlayerWindowSize(skin)).toEqual({ width: 320, height: 480 });
  });

  it('applies mini-player scale to resolved window dimensions', () => {
    expect(resolveMiniPlayerWindowSize(resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN), 0.35)).toEqual({ width: 112, height: 168 });
  });

  it('preserves each skin aspect ratio at the smallest scale', () => {
    for (const skin of MINI_PLAYER_SKINS) {
      const size = resolveMiniPlayerWindowSize(skin, 0.3);

      expect(size.width / size.height).toBeCloseTo(skin.window.width / skin.window.height, 2);
    }
  });

  it('scales the Classic skin proportionally below 100%', () => {
    const skin = resolveMiniPlayerSkin('winamp-classic');
    const smallest = resolveMiniPlayerWindowSize(skin, 0.3);
    const largest = resolveMiniPlayerWindowSize(skin, 1);

    expect(smallest).toEqual({ width: 120, height: 30 });
    expect(smallest.width).toBeLessThan(largest.width);
    expect(smallest.height).toBeLessThan(largest.height);
    expect(smallest.width / smallest.height).toBeCloseTo(skin.window.width / skin.window.height, 1);
  });

  it('floors tiny positive malformed skin dimensions before native resize', () => {
    const skin = {
      ...resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN),
      window: { width: 0.1, height: 0.1, resizable: false },
    };

    expect(resolveMiniPlayerWindowSize(skin)).toEqual({ width: 1, height: 1 });
  });

  it('activates the Classic skin from the registry', () => {
    installLocalStorage();

    activateMiniPlayerSkin('winamp-classic');

    expect(resolveMiniPlayerSkin(get(selectedMiniPlayerSkinId)).id).toBe('winamp-classic');
    expect(localStorage.getItem('helix-mini-player-skin')).toBe('winamp-classic');
  });

  it('clamps and persists the mini-player scale preference', () => {
    installLocalStorage();

    setMiniPlayerScale(0.1);
    expect(get(miniPlayerScale)).toBe(0.3);
    expect(localStorage.getItem('helix-mini-player-scale')).toBe('0.3');

    setMiniPlayerScale(2);
    expect(get(miniPlayerScale)).toBe(1);
    expect(localStorage.getItem('helix-mini-player-scale')).toBe('1');

    expect(clampMiniPlayerScale(0.33)).toBe(0.35);
  });
});
