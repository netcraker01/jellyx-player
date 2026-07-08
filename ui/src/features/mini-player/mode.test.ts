import { beforeEach, describe, expect, it, vi } from 'vitest';
import { enterMiniPlayer, exitMiniPlayer } from './mode';
import { enterNativeMiniWindow, restoreNativeFullWindow } from './nativeWindow';
import { activateMiniPlayerSkin } from './skins';
import { openMiniPlayer, restoreFullPlayer } from '@services/commands';
import { navigate } from '@app/router/navigation';

const mocks = vi.hoisted(() => ({
  path: (() => {
    let value = '/library';
    const subscribers = new Set<(value: string) => void>();
    return {
      subscribe(run: (value: string) => void) {
        run(value);
        subscribers.add(run);
        return () => subscribers.delete(run);
      },
      set(next: string) {
        value = next;
        subscribers.forEach((run) => run(value));
      },
    };
  })(),
  navigate: vi.fn(),
  openMiniPlayer: vi.fn().mockResolvedValue(undefined),
  restoreFullPlayer: vi.fn().mockResolvedValue(undefined),
  enterNativeMiniWindow: vi.fn().mockResolvedValue({
    size: { width: 1200, height: 800 },
    position: { x: 20, y: 30 },
  }),
  restoreNativeFullWindow: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@app/router/navigation', () => ({
  currentPath: mocks.path,
  navigate: mocks.navigate,
}));

vi.mock('@services/commands', () => ({
  openMiniPlayer: mocks.openMiniPlayer,
  restoreFullPlayer: mocks.restoreFullPlayer,
}));

vi.mock('./nativeWindow', () => ({
  enterNativeMiniWindow: mocks.enterNativeMiniWindow,
  restoreNativeFullWindow: mocks.restoreNativeFullWindow,
}));

describe('mini player mode', () => {
  beforeEach(() => {
    mocks.path.set('/library');
    mocks.navigate.mockReset();
    mocks.openMiniPlayer.mockClear();
    mocks.restoreFullPlayer.mockClear();
    mocks.enterNativeMiniWindow.mockClear();
    mocks.restoreNativeFullWindow.mockClear();
    activateMiniPlayerSkin('ipod-classic');
  });

  it('opens mini mode inside the existing app route', async () => {
    await enterMiniPlayer();

    expect(enterNativeMiniWindow).toHaveBeenCalledWith({ width: 320, height: 480 });
    expect(openMiniPlayer).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith('/mini-player');
  });

  it('uses the selected skin dimensions for native mini mode', async () => {
    activateMiniPlayerSkin('graphite-pocket');

    await enterMiniPlayer();

    expect(enterNativeMiniWindow).toHaveBeenCalledWith({ width: 300, height: 480 });
  });

  it('does not overwrite saved native bounds when already in mini mode', async () => {
    mocks.path.set('/mini-player');

    await enterMiniPlayer();

    expect(enterNativeMiniWindow).not.toHaveBeenCalled();
    expect(openMiniPlayer).not.toHaveBeenCalled();
  });

  it('restores the full app to the previous route', async () => {
    await enterMiniPlayer();
    await exitMiniPlayer();

    expect(restoreNativeFullWindow).toHaveBeenCalledWith({
      size: { width: 1200, height: 800 },
      position: { x: 20, y: 30 },
    });
    expect(restoreFullPlayer).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenLastCalledWith('/library', { replace: true });
  });
});
