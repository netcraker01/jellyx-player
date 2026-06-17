/**
 * Favorites store — IPC-backed Svelte store for favorite tracks.
 *
 * Loads favorites from the Rust backend on init.
 * Provides add/remove actions that update both backend and local state.
 */
import { writable } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { FavoriteEntry, Track } from '@shared/types/models';

export interface FavoritesStore {
  subscribe: typeof writable<FavoriteEntry[]>['subscribe'];
  load: () => Promise<void>;
  add: (track: Track) => Promise<void>;
  remove: (trackId: string) => Promise<void>;
  toggle: (trackId: string) => Promise<boolean>;
  isFavorite: (trackId: string) => boolean;
}

function createFavoritesStore(): FavoritesStore {
  const { subscribe, set, update } = writable<FavoriteEntry[]>([]);

  return {
    subscribe,

    /** Load favorites from the Rust backend. */
    async load() {
      try {
        const entries = await commands.getFavorites();
        set(entries);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Add a track to favorites (backend + local state). */
    async add(track: Track) {
      try {
        await commands.addFavorite(track);
        // Optimistically add to local state
        update((entries) => {
          const entry: FavoriteEntry = {
            track,
            addedAt: new Date().toISOString(),
          };
          return [entry, ...entries];
        });
        notifications.push({ type: 'success', title: 'Favorites', message: 'Added to favorites', dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Remove a track from favorites (backend + local state). */
    async remove(trackId: string) {
      try {
        await commands.removeFavorite(trackId);
        // Optimistically remove from local state
        update((entries) => entries.filter((e) => e.track.id !== trackId));
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Toggle favorite state for a track via atomic IPC command.
     *  Returns true if now favorited, false if removed.
     */
    async toggle(trackId: string) {
      try {
        const nowFavorited = await commands.toggleFavorite(trackId);
        update((entries) => {
          if (nowFavorited) {
            // We may not have the full track here; the backend owns truth.
            // Refresh the list to ensure consistency.
            return entries;
          }
          return entries.filter((e) => e.track.id !== trackId);
        });
        if (nowFavorited) {
          // Reload favorites so the new entry has full track metadata.
          await this.load();
        }
        return nowFavorited;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Favorites Error', message: msg, dismissible: true });
        throw e;
      }
    },

    /** Check if a track is in favorites (local state only — fast). */
    isFavorite(trackId: string): boolean {
      let found = false;
      subscribe((entries) => {
        found = entries.some((e) => e.track.id === trackId);
      })();
      return found;
    },
  };
}

export const favorites = createFavoritesStore();