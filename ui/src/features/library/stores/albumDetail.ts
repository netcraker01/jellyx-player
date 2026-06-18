/**
 * Album detail store — IPC-backed Svelte store for album detail views.
 */
import { writable, type Writable } from 'svelte/store';
import { getAlbumDetail as getAlbumDetailCommand } from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { AlbumDetail } from '@shared/types/models';

export interface AlbumDetailStore {
  subscribe: Writable<AlbumDetail | null>['subscribe'];
  load: (id: string) => Promise<void>;
  clear: () => void;
}

function createAlbumDetailStore(): AlbumDetailStore {
  const { subscribe, set } = writable<AlbumDetail | null>(null);

  return {
    subscribe,

    /** Load full album detail by ID from the Rust backend. */
    async load(id: string) {
      isLoadingAlbumDetail.set(true);
      albumDetailError.set(null);
      try {
        const detail = await getAlbumDetailCommand(id);
        set(detail);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        albumDetailError.set(msg);
        notifications.push({ type: 'error', title: 'Album Error', message: msg, dismissible: true });
        set(null);
        throw e;
      } finally {
        isLoadingAlbumDetail.set(false);
      }
    },

    /** Clear album detail and error state. */
    clear() {
      set(null);
      albumDetailError.set(null);
    },
  };
}

/** Current album detail (null if not loaded). */
export const albumDetail = createAlbumDetailStore();

/** Whether an album detail request is in flight. */
export const isLoadingAlbumDetail = writable(false);

/** Error message from the last failed album detail load (null if no error). */
export const albumDetailError = writable<string | null>(null);

/** Convenience action: load album detail by ID. */
export const loadAlbumDetail = albumDetail.load.bind(albumDetail);

/** Convenience action: clear album detail state. */
export const clearAlbumDetail = albumDetail.clear.bind(albumDetail);
