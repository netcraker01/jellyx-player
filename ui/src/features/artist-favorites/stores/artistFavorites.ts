/**
 * Artist Favorites store — IPC-backed Svelte store for favorite artists.
 *
 * Favorites are keyed by the composite key `artist_id + source` so the same
 * artist name from different sources (e.g. "local" vs "youtube") can coexist
 * without overwriting each other. Existing favorites default to source
 * "local" for backward compatibility.
 */
import { writable, type Writable } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { ArtistFavorite } from '@shared/types/models';

/** Default source dimension used when callers don't specify one. */
const DEFAULT_SOURCE = 'local';

/** Build the composite key used to dedupe favorites in the local store. */
function favoriteKey(artistId: string, source?: string): string {
  return `${artistId}::${source ?? DEFAULT_SOURCE}`;
}

export interface ArtistFavoritesStore {
  subscribe: Writable<ArtistFavorite[]>['subscribe'];
  load: () => Promise<void>;
  add: (
    artistId: string,
    artistName: string,
    thumbnail?: string,
    source?: string,
    sourceArtistRef?: string,
  ) => Promise<void>;
  remove: (artistId: string, source?: string) => Promise<void>;
  isFavorite: (artistId: string, source?: string) => Promise<boolean>;
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

    /** Add an artist to favorites with an optional source dimension. */
    async add(
      artistId: string,
      artistName: string,
      thumbnail?: string,
      source?: string,
      sourceArtistRef?: string,
    ) {
      const src = source ?? DEFAULT_SOURCE;
      try {
        await commands.addArtistFavorite(artistId, artistName, thumbnail, src, sourceArtistRef);
        const now = new Date().toISOString();
        update((entries) => {
          // Replace if already exists for this (artistId, source) pair.
          const filtered = entries.filter((a) => favoriteKey(a.artistId, a.source) !== favoriteKey(artistId, src));
          return [
            { artistId, source: src, artistName, thumbnail, sourceArtistRef, addedAt: now },
            ...filtered,
          ];
        });
        notifications.push({ type: 'success', title: 'Artist Favorites', message: `Added ${artistName}`, dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Remove an artist from favorites for a given source (or all sources). */
    async remove(artistId: string, source?: string) {
      const src = source ?? DEFAULT_SOURCE;
      try {
        await commands.removeArtistFavorite(artistId, src);
        update((entries) => entries.filter((a) => favoriteKey(a.artistId, a.source) !== favoriteKey(artistId, src)));
        notifications.push({ type: 'success', title: 'Artist Favorites', message: 'Removed from favorites', dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
      }
    },

    /** Check if an artist is favorited for a given source (queries backend). */
    async isFavorite(artistId: string, source?: string): Promise<boolean> {
      const src = source ?? DEFAULT_SOURCE;
      try {
        return await commands.isArtistFavorite(artistId, src);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Artist Favorites Error', message: msg, dismissible: true });
        return false;
      }
    },
  };
}

export const artistFavorites = createArtistFavoritesStore();