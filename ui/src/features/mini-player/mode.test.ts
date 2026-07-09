import { beforeEach, describe, expect, it, vi } from 'vitest';
import { enterMiniPlayer, exitMiniPlayer, minimizeMiniPlayer, quitFromMiniPlayer } from './mode';
import { closeNativeWindow, enterNativeMiniWindow, minimizeNativeWindow, restoreNativeFullWindow } from './nativeWindow';
import { activateMiniPlayerSkin, setMiniPlayerScale } from './skins';
import { openMiniPlayer, restoreFullPlayer } from '@services/commands';
import { navigate } from '@app/router/navigation';

const mocks = vi.hoisted(() => ({
    savedNativeWindowState: {
      size: { width: 1200, height: 800 },
      position: { x: 20, y: 30 },
      decorated: true,
  },
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
  openMiniPlayer: vi.fn(),
  restoreFullPlayer: vi.fn(),
  enterNativeMiniWindow: vi.fn(),
  restoreNativeFullWindow: vi.fn(),
  minimizeNativeWindow: vi.fn(),
  closeNativeWindow: vi.fn(),
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
  minimizeNativeWindow: mocks.minimizeNativeWindow,
  closeNativeWindow: mocks.closeNativeWindow,
}));

describe('mini player mode', () => {
  beforeEach(() => {
    mocks.path.set('/library');
    mocks.navigate.mockReset();
    mocks.openMiniPlayer.mockReset().mockResolvedValue(undefined);
    mocks.restoreFullPlayer.mockReset().mockResolvedValue(undefined);
    mocks.enterNativeMiniWindow.mockReset().mockResolvedValue(mocks.savedNativeWindowState);
    mocks.restoreNativeFullWindow.mockReset().mockResolvedValue(undefined);
    mocks.minimizeNativeWindow.mockReset().mockResolvedValue(undefined);
    mocks.closeNativeWindow.mockReset().mockResolvedValue(undefined);
    activateMiniPlayerSkin('ipod-classic');
    setMiniPlayerScale(1);
  });

  it('opens mini mode inside the existing app route', async () => {
    await enterMiniPlayer();

    expect(enterNativeMiniWindow).toHaveBeenCalledWith({ width: 320, height: 480 });
    expect(openMiniPlayer).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith('/mini-player');
  });

  it('uses the selected skin dimensions for native mini mode', async () => {
    activateMiniPlayerSkin('winamp-classic');

    await enterMiniPlayer();

    expect(enterNativeMiniWindow).toHaveBeenCalledWith({ width: 400, height: 100 });
  });

  it('uses the persisted mini-player scale for native mini mode', async () => {
    setMiniPlayerScale(0.3);

    await enterMiniPlayer();

    expect(enterNativeMiniWindow).toHaveBeenCalledWith({ width: 96, height: 144 });
  });

  it('does not overwrite saved native bounds when already in mini mode', async () => {
    mocks.path.set('/mini-player');

    await enterMiniPlayer();

    expect(enterNativeMiniWindow).not.toHaveBeenCalled();
    expect(openMiniPlayer).not.toHaveBeenCalled();
  });

  it('restores native full window state when mini-player entry fails after native changes', async () => {
    const failure = new Error('open failed');
    mocks.openMiniPlayer.mockRejectedValue(failure);

    await expect(enterMiniPlayer()).rejects.toThrow(failure);

    expect(enterNativeMiniWindow).toHaveBeenCalledWith({ width: 320, height: 480 });
    expect(restoreNativeFullWindow).toHaveBeenCalledWith(mocks.savedNativeWindowState);
    expect(navigate).not.toHaveBeenCalled();
  });

  it('restores the full app to the previous route', async () => {
    await enterMiniPlayer();
    await exitMiniPlayer();

    expect(restoreNativeFullWindow).toHaveBeenCalledWith({
      size: { width: 1200, height: 800 },
      position: { x: 20, y: 30 },
      decorated: true,
    });
    expect(restoreFullPlayer).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenLastCalledWith('/library', { replace: true });
  });

  it('forwards mini-player minimize and quit controls to the native window', async () => {
    await minimizeMiniPlayer();
    await quitFromMiniPlayer();

    expect(minimizeNativeWindow).toHaveBeenCalledTimes(1);
    expect(closeNativeWindow).toHaveBeenCalledTimes(1);
  });
});
