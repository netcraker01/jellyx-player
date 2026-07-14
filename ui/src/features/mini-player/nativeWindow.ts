import { LogicalPosition, LogicalSize, getCurrentWindow } from '@tauri-apps/api/window';
import type { MiniPlayerWindowSize } from './skins';

export const NORMAL_APP_MIN_SIZE: MiniPlayerWindowSize = { width: 900, height: 600 };

export interface SavedNativeWindowState {
  size: MiniPlayerWindowSize | null;
  position: { x: number; y: number } | null;
  decorated: boolean | null;
  resizable: boolean | null;
}

type OptionalResizableWindowApi = {
  isResizable?: () => Promise<boolean>;
  setResizable?: (resizable: boolean) => Promise<void>;
};

async function getResizableState(window: OptionalResizableWindowApi): Promise<boolean | null> {
  if (typeof window.isResizable !== 'function') return null;
  return window.isResizable().catch(() => null);
}

async function setResizableState(window: OptionalResizableWindowApi, resizable: boolean): Promise<void> {
  if (typeof window.setResizable !== 'function') return;
  await window.setResizable(resizable).catch(() => undefined);
}

/**
 * Enter mini-player mode: save current state, then apply decorations/size.
 *
 * On Windows the client area can desync when `setSize` runs before
 * `setDecorations(false)`. The safe order is:
 *   1. Decorations off (transparent + chromeless)
 *   2. Min-size + size to the final skin dimensions
 *   3. Lock resizable
 */
export async function enterNativeMiniWindow(size: MiniPlayerWindowSize): Promise<SavedNativeWindowState> {
  const window = getCurrentWindow();
  const scaleFactor = await window.scaleFactor();

  const [outerSize, outerPosition] = await Promise.all([
    window.outerSize().catch(() => null),
    window.outerPosition().catch(() => null),
  ]);
  const decorated = await window.isDecorated().catch(() => null);
  const resizable = await getResizableState(window);

  // 1. Chrome off first — avoids Windows client-area desync.
  await window.setDecorations(false).catch(() => undefined);

  // 2. Final size + min-size.
  await window.setMinSize(new LogicalSize(size.width, size.height));
  await window.setSize(new LogicalSize(size.width, size.height));

  // 3. Lock resize.
  await setResizableState(window, false);
  await window.setFocus().catch(() => undefined);

  const logicalSize = outerSize?.toLogical(scaleFactor) ?? null;
  const logicalPosition = outerPosition?.toLogical(scaleFactor) ?? null;

  return {
    size: logicalSize ? { width: Math.round(logicalSize.width), height: Math.round(logicalSize.height) } : null,
    position: logicalPosition ? { x: Math.round(logicalPosition.x), y: Math.round(logicalPosition.y) } : null,
    decorated,
    resizable,
  };
}

/**
 * Resize the mini-player window to match a new scale / skin without
 * re-centering. Preserves the user-chosen position.
 */
export async function updateMiniWindowSize(size: MiniPlayerWindowSize): Promise<void> {
  const window = getCurrentWindow();
  await window.setMinSize(new LogicalSize(size.width, size.height));
  await window.setSize(new LogicalSize(size.width, size.height));
}

/**
 * Restore the window to normal mode.
 *
 * Safe order for Windows: decorations on *before* the final size so the
 * OS can re-measure the client area correctly.
 */
export async function restoreNativeFullWindow(state: SavedNativeWindowState | null): Promise<void> {
  const window = getCurrentWindow();
  const restoredSize = state?.size ?? NORMAL_APP_MIN_SIZE;

  // 1. Min-size first so the OS won't reject a smaller-to-larger transition.
  await window.setMinSize(new LogicalSize(NORMAL_APP_MIN_SIZE.width, NORMAL_APP_MIN_SIZE.height));

  // 2. Decorations on before final size (Windows client-area safety).
  if (state?.decorated !== null && state?.decorated !== undefined) {
    await window.setDecorations(state.decorated).catch(() => undefined);
  }

  if (state?.resizable !== null && state?.resizable !== undefined) {
    await setResizableState(window, state.resizable);
  }

  // 3. Final size + position.
  await window.setSize(new LogicalSize(restoredSize.width, restoredSize.height));

  if (state?.position) {
    await window.setPosition(new LogicalPosition(state.position.x, state.position.y)).catch(() => undefined);
  }

  await window.setFocus().catch(() => undefined);
}

export async function minimizeNativeWindow(): Promise<void> {
  await getCurrentWindow().minimize();
}

export async function closeNativeWindow(): Promise<void> {
  await getCurrentWindow().close();
}
