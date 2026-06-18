/**
 * Search store — IPC-backed Svelte store for search state.
 *
 * Provides search action that calls commands.search and updates local state.
 * Follows the same pattern as the favorites store.
 */
import { writable, type Writable } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { Track } from '@shared/types/models';

export interface SearchStore {
  subscribe: Writable<Track[]>['subscribe'];
  search: (query: string) => Promise<void>;
  clear: () => void;
}

function createSearchStore(): SearchStore {
  const { subscribe, set } = writable<Track[]>([]);

  return {
    subscribe,

    /** Execute a search query against the Rust backend. */
    async search(query: string) {
      if (!query.trim()) {
        set([]);
        return;
      }
      isSearching.set(true);
      searchError.set(null);
      try {
        const results = await commands.search(query);
        set(results);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        searchError.set(msg);
        notifications.push({ type: 'error', title: 'Search Failed', message: msg, dismissible: true });
        set([]);
      } finally {
        isSearching.set(false);
      }
    },

    /** Clear search results and query. */
    clear() {
      set([]);
      searchQuery.set('');
      searchError.set(null);
    },
  };
}

/** Current search query string. */
export const searchQuery = writable('');

/** Search results (array of Track). */
export const searchResults = createSearchStore();

/** Whether a search request is in flight. */
export const isSearching = writable(false);

/** Error message from the last failed search (null if no error). */
export const searchError = writable<string | null>(null);