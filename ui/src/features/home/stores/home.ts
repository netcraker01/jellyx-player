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
  const { subscribe, set } = homeData;

  return {
    subscribe,
    loading: homeLoading,
    error: homeError,

    /** Load the Home snapshot from the Rust backend. */
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
