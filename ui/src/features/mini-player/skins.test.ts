import { describe, expect, it } from 'vitest';
import { DEFAULT_MINI_PLAYER_SKIN, MINI_PLAYER_SKINS, resolveMiniPlayerSkin, resolveMiniPlayerWindowSize } from './skins';

describe('mini player skins', () => {
  it('ships an iPod-like default skin', () => {
    const skin = resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN);
    expect(skin.id).toBe('ipod-classic');
    expect(skin.window).toMatchObject({ width: 320, height: 480, resizable: false });
    expect(skin.layout.controls).toEqual(['previous', 'playPause', 'next']);
  });

  it('ships only declarative skins with sizing contracts', () => {
    expect(MINI_PLAYER_SKINS.length).toBeGreaterThan(1);
    expect(MINI_PLAYER_SKINS.every((skin) => skin.window.width > 0 && skin.window.height > 0)).toBe(true);
    expect(MINI_PLAYER_SKINS.every((skin) => !('script' in skin))).toBe(true);
  });

  it('falls back to the default skin for unknown ids', () => {
    expect(resolveMiniPlayerSkin('community-scripted-skin').id).toBe(DEFAULT_MINI_PLAYER_SKIN);
  });

  it('clamps declarative window dimensions before native resize', () => {
    const skin = {
      ...resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN),
      window: { width: 9999, height: Number.NaN, resizable: false },
    };

    expect(resolveMiniPlayerWindowSize(skin)).toEqual({ width: 640, height: 240 });
  });
});
