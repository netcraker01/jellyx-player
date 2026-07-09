import { beforeEach, describe, expect, it, vi } from 'vitest';
import { closeNativeWindow, enterNativeMiniWindow, minimizeNativeWindow, NORMAL_APP_MIN_SIZE, restoreNativeFullWindow } from './nativeWindow';

const mocks = vi.hoisted(() => ({
  window: {
    scaleFactor: vi.fn().mockResolvedValue(2),
    outerSize: vi.fn().mockResolvedValue({ toLogical: () => ({ width: 1200, height: 800 }) }),
    outerPosition: vi.fn().mockResolvedValue({ toLogical: () => ({ x: 40, y: 50 }) }),
    isDecorated: vi.fn().mockResolvedValue(true),
    setMinSize: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    setDecorations: vi.fn().mockResolvedValue(undefined),
    setFocus: vi.fn().mockResolvedValue(undefined),
    minimize: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  },
  LogicalSize: vi.fn(function (this: { width: number; height: number }, width: number, height: number) {
    this.width = width;
    this.height = height;
  }),
  LogicalPosition: vi.fn(function (this: { x: number; y: number }, x: number, y: number) {
    this.x = x;
    this.y = y;
  }),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => mocks.window,
  LogicalSize: mocks.LogicalSize,
  LogicalPosition: mocks.LogicalPosition,
}));

describe('native mini-player window helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.window.scaleFactor.mockResolvedValue(2);
    mocks.window.outerSize.mockResolvedValue({ toLogical: () => ({ width: 1200, height: 800 }) });
    mocks.window.outerPosition.mockResolvedValue({ toLogical: () => ({ x: 40, y: 50 }) });
    mocks.window.isDecorated.mockResolvedValue(true);
    mocks.window.setDecorations.mockResolvedValue(undefined);
  });

  it('saves the current outer bounds and resizes to mini skin size', async () => {
    const state = await enterNativeMiniWindow({ width: 320, height: 480 });

    expect(state).toEqual({ size: { width: 1200, height: 800 }, position: { x: 40, y: 50 }, decorated: true });
    expect(mocks.window.setMinSize).toHaveBeenCalledWith({ width: 320, height: 480 });
    expect(mocks.window.setSize).toHaveBeenCalledWith({ width: 320, height: 480 });
    expect(mocks.window.setDecorations).toHaveBeenCalledWith(false);
  });

  it('records the previous hidden decoration state before entering mini mode', async () => {
    mocks.window.isDecorated.mockResolvedValueOnce(false);

    const state = await enterNativeMiniWindow({ width: 320, height: 480 });

    expect(state.decorated).toBe(false);
  });

  it('records an unknown decoration state when it cannot be queried', async () => {
    mocks.window.isDecorated.mockRejectedValueOnce(new Error('unsupported'));

    const state = await enterNativeMiniWindow({ width: 320, height: 480 });

    expect(state.decorated).toBeNull();
  });

  it('restores normal minimum size before saved bounds', async () => {
    await restoreNativeFullWindow({ size: { width: 1200, height: 800 }, position: { x: 40, y: 50 }, decorated: true });

    expect(mocks.window.setMinSize).toHaveBeenCalledWith(NORMAL_APP_MIN_SIZE);
    expect(mocks.window.setSize).toHaveBeenCalledWith({ width: 1200, height: 800 });
    expect(mocks.window.setPosition).toHaveBeenCalledWith({ x: 40, y: 50 });
    expect(mocks.window.setDecorations).toHaveBeenLastCalledWith(true);
  });

  it('restores a pre-existing hidden native title bar after mini mode', async () => {
    await restoreNativeFullWindow({ size: { width: 1200, height: 800 }, position: { x: 40, y: 50 }, decorated: false });

    expect(mocks.window.setDecorations).toHaveBeenLastCalledWith(false);
  });

  it('does not change decorations when the previous decoration state is unknown', async () => {
    await restoreNativeFullWindow({ size: { width: 1200, height: 800 }, position: null, decorated: null });

    expect(mocks.window.setDecorations).not.toHaveBeenCalled();
  });

  it('falls back to the normal app size when saved bounds are missing', async () => {
    await restoreNativeFullWindow(null);

    expect(mocks.window.setMinSize).toHaveBeenCalledWith(NORMAL_APP_MIN_SIZE);
    expect(mocks.window.setSize).toHaveBeenCalledWith(NORMAL_APP_MIN_SIZE);
    expect(mocks.window.setPosition).not.toHaveBeenCalled();
  });

  it('exposes native minimize and close helpers for mini controls', async () => {
    await minimizeNativeWindow();
    await closeNativeWindow();

    expect(mocks.window.minimize).toHaveBeenCalledTimes(1);
    expect(mocks.window.close).toHaveBeenCalledTimes(1);
  });
});
