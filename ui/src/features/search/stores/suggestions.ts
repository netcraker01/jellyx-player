/**
 * Suggestion categories store — fetches genre/mood categories from the backend.
 *
 * Provides a simple reactive store that loads suggestion categories once
 * and caches them for the session.
 */
import { writable, type Writable } from 'svelte/store';
import { getSuggestionCategories } from '@services/commands';
import type { SuggestionCategory } from '@shared/types/models';

/** Whether categories are currently loading. */
export const isLoadingCategories = writable(false);

/** Error message from the last failed load (null if no error). */
export const categoriesError = writable<string | null>(null);

/** The loaded suggestion categories. */
export const suggestionCategories: Writable<SuggestionCategory[]> = writable([]);

let _loaded = false;

/** Load suggestion categories from the backend. Skips if already loaded. */
export async function loadSuggestionCategories(): Promise<void> {
  if (_loaded) return;
  _loaded = true;
  isLoadingCategories.set(true);
  categoriesError.set(null);
  try {
    const cats = await getSuggestionCategories();
    suggestionCategories.set(cats);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    categoriesError.set(msg);
    // Allow retry on next call
    _loaded = false;
  } finally {
    isLoadingCategories.set(false);
  }
}

/** Force reload categories (useful if previous load failed). */
export async function reloadSuggestionCategories(): Promise<void> {
  _loaded = false;
  await loadSuggestionCategories();
}