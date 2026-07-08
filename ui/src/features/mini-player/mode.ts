import { get } from 'svelte/store';
import { navigate, currentPath } from '@app/router/navigation';
import { openMiniPlayer, restoreFullPlayer } from '@services/commands';
import { enterNativeMiniWindow, restoreNativeFullWindow, type SavedNativeWindowState } from './nativeWindow';
import { resolveMiniPlayerSkin, resolveMiniPlayerWindowSize, selectedMiniPlayerSkinId } from './skins';

const MINI_PLAYER_PATH = '/mini-player';
let previousFullPath = '/';
let savedWindowState: SavedNativeWindowState | null = null;

export async function enterMiniPlayer(): Promise<void> {
  const path = get(currentPath);
  if (path === MINI_PLAYER_PATH) return;

  previousFullPath = path;

  const skin = resolveMiniPlayerSkin(get(selectedMiniPlayerSkinId));
  savedWindowState = await enterNativeMiniWindow(resolveMiniPlayerWindowSize(skin));
  await openMiniPlayer();
  navigate(MINI_PLAYER_PATH);
}

export async function exitMiniPlayer(): Promise<void> {
  await restoreNativeFullWindow(savedWindowState);
  savedWindowState = null;
  await restoreFullPlayer();
  navigate(previousFullPath === MINI_PLAYER_PATH ? '/' : previousFullPath, { replace: true });
}
