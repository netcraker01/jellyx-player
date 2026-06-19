/**
 * Home store — IPC-backed Svelte store for the Home snapshot.
 *
 * Loads recently played and recommendations from the Rust backend
 * in a single call. Centralizes loading/error state for the Home route.
 */
import { writable, type Writable } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { HomeSnapshot } from '@shared/types/models';

export interface HomeStore {
  subscribe: Writable<HomeSnapshot | null>['subscribe'];
  loading: Writable<boolean>;
  error: Writable<string | null>;
  load(): Promise<void>;
  clear(): void;
}

// ── Internal stores ───────────────────────────────────────────────

const homeData = writable<HomeSnapshot | null>(null);
export const homeLoading = writable(false);
export const homeError = writable<string | null>(null);

// ── Store factory ───────────────────────────────────────────────────

function createHomeStore(): HomeStore {
  const { subscribe, set, update } = homeData;

  return {
    subscribe,
    loading: homeLoading,
    error: homeError,

    /** Load the Home snapshot from the Rust backend.
     *
     * Recently played is fetched first so the page renders immediately;
     * recommendations are loaded asynchronously to avoid blocking startup.
     */
    async load() {
      homeLoading.set(true);
      homeError.set(null);

      try {
        const snapshot = await commands.getHomeSnapshot();
        set(snapshot);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        homeError.set(msg);
        notifications.push({ type: 'error', title: 'Home Error', message: msg, dismissible: true });
        set(null);
      } finally {
        homeLoading.set(false);
      }

      // Fetch heavy recommendations in the background so the UI stays responsive.
      // Not awaited — load() resolves after the cheap snapshot.
      commands.getHomeRecommendations()
        .then((recommendations) => {
          update((current) => {
            if (!current) return null;
            return { ...current, recommendations };
          });
        })
        .catch((recErr) => {
          // Recommendations are secondary; do not fail the whole page.
          // eslint-disable-next-line no-console
          console.warn('Failed to load recommendations:', recErr);
        });
    },

    /** Reset all Home state to initial values. */
    clear() {
      set(null);
      homeLoading.set(false);
      homeError.set(null);
    },
  };
}

export const homeStore = createHomeStore();
