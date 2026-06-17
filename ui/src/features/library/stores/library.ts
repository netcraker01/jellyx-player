/**
 * Library store — manages local music library state.
 *
 * Provides watched folders, local tracks, and scan status.
 */
import { writable, derived } from 'svelte/store';
import {
  scanFolder,
  getLocalTracks,
  getWatchedFolders,
  removeWatchedFolder,
} from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { WatchedFolder, LocalTrackEntry, ScanResult } from '@shared/types/models';

// ── Stores ────────────────────────────────────────────────────────

export const watchedFolders = writable<WatchedFolder[]>([]);
export const localTracks = writable<LocalTrackEntry[]>([]);
export const scanStatus = writable<ScanResult | null>(null);
export const isScanning = writable(false);
export const scanError = writable<string | null>(null);

// ── Derived ────────────────────────────────────────────────────────

/** Total number of local tracks across all folders. */
export const localTrackCount = derived(localTracks, ($tracks) => $tracks.length);

/** Group local tracks by folder path. */
export const tracksByFolder = derived(localTracks, ($tracks) => {
  const map = new Map<string, LocalTrackEntry[]>();
  for (const entry of $tracks) {
    const list = map.get(entry.folderPath) ?? [];
    list.push(entry);
    map.set(entry.folderPath, list);
  }
  return map;
});

// ── Actions ─────────────────────────────────────────────────────────

/** Load watched folders from backend. */
export async function loadWatchedFolders(): Promise<void> {
  try {
    const folders = await getWatchedFolders();
    watchedFolders.set(folders);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Library Error', message: msg, dismissible: true });
  }
}

/** Load local tracks from backend. */
export async function loadLocalTracks(folderPath?: string): Promise<void> {
  try {
    const tracks = await getLocalTracks(folderPath);
    localTracks.set(tracks);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Library Error', message: msg, dismissible: true });
  }
}

/** Scan a folder and refresh state. */
export async function scanNewFolder(folderPath: string): Promise<void> {
  isScanning.set(true);
  scanError.set(null);

  try {
    const result = await scanFolder(folderPath);
    scanStatus.set(result);
    // Refresh both folders and tracks
    await loadWatchedFolders();
    await loadLocalTracks();
    // Success toast
    notifications.push({
      type: 'success',
      title: 'Scan Complete',
      message: `Added ${result.filesAdded} file${result.filesAdded === 1 ? '' : 's'}`,
      dismissible: true,
    });
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    scanError.set(msg);
    notifications.push({ type: 'error', title: 'Scan Failed', message: msg, dismissible: true });
  } finally {
    isScanning.set(false);
  }
}

/** Remove a watched folder and refresh state. */
export async function removeFolder(path: string): Promise<void> {
  try {
    await removeWatchedFolder(path);
    await loadWatchedFolders();
    await loadLocalTracks();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Library Error', message: msg, dismissible: true });
  }
}