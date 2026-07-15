/**
 * WelcomeModal store — determines whether the welcome dialog should appear.
 *
 * Shows the dialog once per version: if the persisted "last-seen version"
 * differs from the current version (or is absent), the modal opens.
 * Dismissing it persists the current version so it won't reappear until
 * the next install/update.
 */
import { writable, get } from 'svelte/store';
import { getVersion } from '@services/commands';
import { getMigratedItem, setMigratedItem } from '@shared/utils/storage';

const STORAGE_KEY = 'welcome-last-seen-version';

export interface WelcomeState {
  modalOpen: boolean;
  version: string;
}

const store = writable<WelcomeState>({ modalOpen: false, version: '' });

export const welcomeStore = {
  subscribe: store.subscribe,
};

let initialized = false;

/** Check whether the welcome modal should show. Call once on app boot. */
export async function checkWelcome(): Promise<void> {
  if (initialized) return;
  initialized = true;
  try {
    const current = await getVersion();
    const lastSeen = getMigratedItem(STORAGE_KEY);
    if (lastSeen !== current) {
      store.set({ modalOpen: true, version: current });
    } else {
      store.set({ modalOpen: false, version: current });
    }
  } catch (err) {
    console.error('[Jellyx] Welcome check failed:', err);
    store.set({ modalOpen: false, version: '' });
  }
}

/** Dismiss the modal and persist the current version. */
export function dismissWelcome(): void {
  const version = get(store).version;
  if (version) {
    setMigratedItem(STORAGE_KEY, version);
  }
  store.update((s) => ({ ...s, modalOpen: false }));
}

/** Force-show the welcome modal (for testing). */
export async function forceShowWelcome(): Promise<void> {
  try {
    const current = get(store).version || await getVersion();
    store.set({ modalOpen: true, version: current });
  } catch (err) {
    console.error('[Jellyx] Force welcome failed:', err);
  }
}
