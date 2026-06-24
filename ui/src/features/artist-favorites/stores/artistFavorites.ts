/**
 * Artist Favorites store — IPC-backed Svelte store for favorite artists.
 *
 * Loads artist favorites from the Rust backend on init.
 * Provides add/remove actions that update both backend and local state.
 */
import { writable, type Writable } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { ArtistFavorite } from '@shared/types/models';

export interface ArtistFavoritesStore {
  subscribe: Writable<ArtistFavorite[]>['subscribe'];
  load: () => Promise<void>;
  add: (artistId: string, artistName: string, thumbnail?: string) => Promise<void>;
  remove: (artistId: string) => Promise<void>;
  isFavorite: (artistId: string) => Promise<boolean>;
}

function createArtistFavoritesStore(): ArtistFavoritesStore {
  const { subscribe, set, update } = writable<ArtistFavorite[]>([]);

  return {
    subscribe,

    /** Load artist favorites from the Rust backend. */
    async load() {
      try {
        const entries = await commands.getAllArtistFavorites();
        set(entries ?? []);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Add an artist to favorites. */
    async add(artistId: string, artistName: string, thumbnail?: string) {
      try {
        await commands.addArtistFavorite(artistId, artistName, thumbnail);
        const now = new Date().toISOString();
        update((entries) => {
          // Replace if already exists
          const filtered = entries.filter((a) => a.artistId !== artistId);
          return [
            { artistId, artistName, thumbnail, addedAt: now },
            ...filtered,
          ];
        });
        notifications.push({ type: 'success', title: 'Artist Favorites', message: `Added ${artistName}`, dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Remove an artist from favorites. */
    async remove(artistId: string) {
      try {
        await commands.removeArtistFavorite(artistId);
        update((entries) => entries.filter((a) => a.artistId !== artistId));
        notifications.push({ type: 'success', title: 'Artist Favorites', message: 'Removed from favorites', dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Check if an artist is favorited (queries backend for truth). */
    async isFavorite(artistId: string): Promise<boolean> {
      try {
        return await commands.isArtistFavorite(artistId);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
        return false;
      }
    },
  };
}

export const artistFavorites = createArtistFavoritesStore();
