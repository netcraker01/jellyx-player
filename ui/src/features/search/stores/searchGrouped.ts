/**
 * Grouped search store — IPC-backed Svelte store for grouped search results.
 *
 * Follows the same pattern as the search store: loading flag, error state,
 * and a search action that calls commands.searchGrouped.
 * Supports pagination via loadMore — appends remote song results.
 */
import { writable, type Writable } from 'svelte/store';
import { searchGrouped as searchGroupedCommand } from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { GroupedSearchResult, SearchFilter } from '@shared/types/models';

const PAGE_SIZE = 50;

export interface GroupedSearchStore {
  subscribe: Writable<GroupedSearchResult | null>['subscribe'];
  search: (query: string, filter?: SearchFilter) => Promise<void>;
  loadMore: () => Promise<void>;
  clear: () => void;
}

function createGroupedSearchStore(): GroupedSearchStore {
  const { subscribe, set, update } = writable<GroupedSearchResult | null>(null);
  let currentQuery = '';
  let currentFilter: SearchFilter | undefined;
  let currentOffset = 0;
  let isLoadingMore = false;

  return {
    subscribe,

    /** Execute a grouped search query against the Rust backend. */
    async search(query: string, filter?: SearchFilter) {
      if (!query.trim()) {
        set(null);
        return;
      }
      currentQuery = query;
      currentFilter = filter;
      currentOffset = 0;
      isSearchingGrouped.set(true);
      groupedSearchError.set(null);
      try {
        const results = await searchGroupedCommand(
          query,
          filter as string | undefined,
          0,
          PAGE_SIZE,
        );
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

    /** Load the next page of remote song results and append. */
    async loadMore() {
      if (isLoadingMore || !currentQuery) return;
      const current = get_current_value(subscribe);
      if (current && !current.hasMoreSongs) return;

      isLoadingMore = true;
      isLoadingMoreResults.set(true);
      currentOffset += PAGE_SIZE;
      try {
        const more = await searchGroupedCommand(
          currentQuery,
          currentFilter as string | undefined,
          currentOffset,
          PAGE_SIZE,
        );
        update((prev) => {
          if (!prev) return more;
          // Append new songs, avoid duplicates by track ID.
          const existingIds = new Set(prev.songs.map((t) => t.id));
          const newSongs = more.songs.filter((t) => !existingIds.has(t.id));
          return {
            ...prev,
            songs: [...prev.songs, ...newSongs],
            hasMoreSongs: more.hasMoreSongs,
          };
        });
      } catch (e) {
        // Revert offset on failure so the user can retry.
        currentOffset -= PAGE_SIZE;
        const msg = e instanceof Error ? e.message : String(e);
        notifications.push({ type: 'error', title: 'Load More Failed', message: msg, dismissible: true });
      } finally {
        isLoadingMore = false;
        isLoadingMoreResults.set(false);
      }
    },

    /** Clear grouped search results and error state. */
    clear() {
      currentQuery = '';
      currentOffset = 0;
      set(null);
      groupedSearchError.set(null);
    },
  };
}

/** Helper to synchronously read the current store value. */
function get_current_value<T>(subscribe: Writable<T>['subscribe']): T | undefined {
  let value: T | undefined;
  const unsub = subscribe((v) => { value = v; });
  unsub();
  return value;
}

/** Grouped search results (songs, artists, albums). */
export const groupedSearchResults = createGroupedSearchStore();

/** Whether a grouped search request is in flight. */
export const isSearchingGrouped = writable(false);

/** Whether a loadMore pagination request is in flight. */
export const isLoadingMoreResults = writable(false);

/** Error message from the last failed grouped search (null if no error). */
export const groupedSearchError = writable<string | null>(null);

/** Convenience action: search grouped results. */
export const searchGrouped = groupedSearchResults.search.bind(groupedSearchResults);

/** Convenience action: load more results (pagination). */
export const loadMoreResults = groupedSearchResults.loadMore.bind(groupedSearchResults);

/** Convenience action: clear grouped search results. */
export const clearSearchGrouped = groupedSearchResults.clear.bind(groupedSearchResults);
