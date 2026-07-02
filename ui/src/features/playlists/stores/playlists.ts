/**
 * Playlist store — IPC-backed Svelte store for user-created local playlists.
 *
 * Loads playlists from the Rust backend on init.
 * Provides CRUD actions that update both backend and local state.
 */
import { writable, derived, type Writable } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { Track, UserPlaylist } from '@shared/types/models';

export interface PlaylistsStore {
  subscribe: Writable<UserPlaylist[]>['subscribe'];
  load: () => Promise<void>;
  create: (title: string) => Promise<UserPlaylist | undefined>;
  rename: (id: string, title: string) => Promise<void>;
  delete: (id: string) => Promise<void>;
  search: (query: string) => Promise<UserPlaylist[]>;
  addTrack: (playlistId: string, track: Track) => Promise<void>;
  removeTrack: (playlistId: string, position: number) => Promise<void>;
}

function createPlaylistsStore(): PlaylistsStore {
  const { subscribe, set, update } = writable<UserPlaylist[]>([]);

  return {
    subscribe,

    /** Load playlists from the Rust backend. */
    async load() {
      try {
        const entries = await commands.getAllPlaylists();
        set(entries ?? []);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
      }
    },

    /** Create a new playlist. Returns the created playlist. */
    async create(title: string): Promise<UserPlaylist | undefined> {
      try {
        const pl = await commands.createPlaylist(title);
        // Guard against a falsy/invalid return so the store never carries
        // undefined entries — those would crash `{#each ... (pl.id)}`
        // consumers (e.g. ListPicker displayLists) and corrupt Svelte's
        // render, leaving modal backdrops mounted and the UI locked.
        if (pl && typeof pl === 'object' && 'id' in pl) {
          update((entries) => [pl, ...entries]);
          notifications.push({ type: 'success', title: 'Lists', message: 'List created', dismissible: true });
          return pl;
        }
        return undefined;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
        return undefined;
      }
    },

    /** Rename a playlist. */
    async rename(id: string, title: string) {
      try {
        await commands.renamePlaylist(id, title);
        update((entries) =>
          entries.map((p) => (p.id === id ? { ...p, title } : p))
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
      }
    },

    /** Delete a playlist. */
    async delete(id: string) {
      try {
        await commands.deletePlaylist(id);
        update((entries) => entries.filter((p) => p.id !== id));
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
      }
    },

    /** Search playlists by title. */
    async search(query: string) {
      try {
        return await commands.searchUserPlaylists(query);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
        return [];
      }
    },

    /** Add a track to a playlist. */
    async addTrack(playlistId: string, track: Track) {
      try {
        await commands.addTrackToPlaylist(playlistId, track);
        notifications.push({ type: 'success', title: 'Lists', message: 'Track added to list', dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
      }
    },

    /** Remove a track from a playlist by position. */
    async removeTrack(playlistId: string, position: number) {
      try {
        await commands.removeTrackFromPlaylist(playlistId, position);
        notifications.push({ type: 'success', title: 'Lists', message: 'Track removed', dismissible: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Lists Error', message: msg, dismissible: true });
      }
    },
  };
}

export const playlists = createPlaylistsStore();

/** Derived store for the 5 most recent playlists. */
export const recentPlaylists = derived(playlists, ($playlists) => $playlists.slice(0, 5));
