/**
 * Grouped search store — IPC-backed Svelte store for grouped search results.
 *
 * Follows the same pattern as the search store: loading flag, error state,
 * and a search action that calls commands.searchGrouped.
 */
import { writable } from 'svelte/store';
import { searchGrouped as searchGroupedCommand } from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { GroupedSearchResult, SearchFilter } from '@shared/types/models';

export interface GroupedSearchStore {
  subscribe: typeof writable<GroupedSearchResult | null>['subscribe'];
  search: (query: string, filter?: SearchFilter) => Promise<void>;
  clear: () => void;
}

function createGroupedSearchStore(): GroupedSearchStore {
  const { subscribe, set } = writable<GroupedSearchResult | null>(null);

  return {
    subscribe,

    /** Execute a grouped search query against the Rust backend. */
    async search(query: string, filter?: SearchFilter) {
      if (!query.trim()) {
        set(null);
        return;
      }
      isSearchingGrouped.set(true);
      groupedSearchError.set(null);
      try {
        const results = await searchGroupedCommand(query, filter);
        set(results);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        groupedSearchError.set(msg);
        notifications.push({ type: 'error', title: 'Search Failed', message: msg, dismissible: true });
        set(null);
        throw e;
      } finally {
        isSearchingGrouped.set(false);
      }
    },

    /** Clear grouped search results and error state. */
    clear() {
      set(null);
      groupedSearchError.set(null);
    },
  };
}

/** Grouped search results (songs, artists, albums). */
export const groupedSearchResults = createGroupedSearchStore();

/** Whether a grouped search request is in flight. */
export const isSearchingGrouped = writable(false);

/** Error message from the last failed grouped search (null if no error). */
export const groupedSearchError = writable<string | null>(null);

/** Convenience action: search grouped results. */
export const searchGrouped = groupedSearchResults.search.bind(groupedSearchResults);

/** Convenience action: clear grouped search results. */
export const clearSearchGrouped = groupedSearchResults.clear.bind(groupedSearchResults);
