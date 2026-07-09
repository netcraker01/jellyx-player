import { get } from 'svelte/store';
import { navigate, currentPath } from '@app/router/navigation';
import { openMiniPlayer, restoreFullPlayer } from '@services/commands';
import { closeNativeWindow, enterNativeMiniWindow, minimizeNativeWindow, restoreNativeFullWindow, type SavedNativeWindowState } from './nativeWindow';
import { miniPlayerScale, resolveMiniPlayerSkin, resolveMiniPlayerWindowSize, selectedMiniPlayerSkinId } from './skins';

const MINI_PLAYER_PATH = '/mini-player';
let previousFullPath = '/';
let savedWindowState: SavedNativeWindowState | null = null;

export async function enterMiniPlayer(): Promise<void> {
  const path = get(currentPath);
  if (path === MINI_PLAYER_PATH) return;

  previousFullPath = path;

  const skin = resolveMiniPlayerSkin(get(selectedMiniPlayerSkinId));
  const nativeWindowState = await enterNativeMiniWindow(resolveMiniPlayerWindowSize(skin, get(miniPlayerScale)));

  try {
    await openMiniPlayer();
    navigate(MINI_PLAYER_PATH);
    savedWindowState = nativeWindowState;
  } catch (error) {
    savedWindowState = null;
    await restoreNativeFullWindow(nativeWindowState);
    throw error;
  }
}

export async function exitMiniPlayer(): Promise<void> {
  await restoreNativeFullWindow(savedWindowState);
  savedWindowState = null;
  await restoreFullPlayer();
  navigate(previousFullPath === MINI_PLAYER_PATH ? '/' : previousFullPath, { replace: true });
}

export async function minimizeMiniPlayer(): Promise<void> {
  await minimizeNativeWindow();
}

export async function quitFromMiniPlayer(): Promise<void> {
  await closeNativeWindow();
}
