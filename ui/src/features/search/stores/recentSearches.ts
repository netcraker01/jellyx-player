/**
 * Recent searches store — persists the last 5 search queries to localStorage.
 *
 * Uses the migrated storage helpers (`jellyx-recent-searches` key with
 * `helix-recent-searches` fallback). Queries are deduplicated case-insensitively
 * and kept newest-first.
 */
import { writable, type Readable } from 'svelte/store';
import { getMigratedItem, setMigratedItem } from '@shared/utils/storage';

const STORAGE_SUFFIX = 'recent-searches';
const MAX_ENTRIES = 5;

function loadInitial(): string[] {
  try {
    const raw = getMigratedItem(STORAGE_SUFFIX);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((q) => typeof q === 'string').slice(0, MAX_ENTRIES) : [];
  } catch {
    return [];
  }
}

const _store = writable<string[]>(loadInitial());

export const recentSearches: Readable<string[]> = _store;

/** Add a query to the recent searches list. Trims, deduplicates
 *  case-insensitively, unshifts to front, and clamps to MAX_ENTRIES. */
export function addRecentSearch(query: string): void {
  const trimmed = query.trim();
  if (!trimmed) return;
  _store.update((current) => {
    const filtered = current.filter((q) => q.toLowerCase() !== trimmed.toLowerCase());
    const updated = [trimmed, ...filtered].slice(0, MAX_ENTRIES);
    setMigratedItem(STORAGE_SUFFIX, JSON.stringify(updated));
    return updated;
  });
}

/** Clear all recent searches. */
export function clearRecentSearches(): void {
  _store.set([]);
  setMigratedItem(STORAGE_SUFFIX, JSON.stringify([]));
}