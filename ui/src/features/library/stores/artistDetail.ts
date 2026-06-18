/**
 * Artist detail store — IPC-backed Svelte store for artist detail views.
 */
import { writable, type Writable } from 'svelte/store';
import { getArtistDetail as getArtistDetailCommand } from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { ArtistDetail } from '@shared/types/models';

export interface ArtistDetailStore {
  subscribe: Writable<ArtistDetail | null>['subscribe'];
  load: (id: string) => Promise<void>;
  clear: () => void;
}

function createArtistDetailStore(): ArtistDetailStore {
  const { subscribe, set } = writable<ArtistDetail | null>(null);

  return {
    subscribe,

    /** Load full artist detail by ID from the Rust backend. */
    async load(id: string) {
      isLoadingArtistDetail.set(true);
      artistDetailError.set(null);
      try {
        const detail = await getArtistDetailCommand(id);
        set(detail);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        artistDetailError.set(msg);
        notifications.push({ type: 'error', title: 'Artist Error', message: msg, dismissible: true });
        set(null);
        throw e;
      } finally {
        isLoadingArtistDetail.set(false);
      }
    },

    /** Clear artist detail and error state. */
    clear() {
      set(null);
      artistDetailError.set(null);
    },
  };
}

/** Current artist detail (null if not loaded). */
export const artistDetail = createArtistDetailStore();

/** Whether an artist detail request is in flight. */
export const isLoadingArtistDetail = writable(false);

/** Error message from the last failed artist detail load (null if no error). */
export const artistDetailError = writable<string | null>(null);

/** Convenience action: load artist detail by ID. */
export const loadArtistDetail = artistDetail.load.bind(artistDetail);

/** Convenience action: clear artist detail state. */
export const clearArtistDetail = artistDetail.clear.bind(artistDetail);
