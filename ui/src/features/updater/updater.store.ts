/**
 * Updater store — IPC-backed Svelte store for the channel-aware updater.
 *
 * Holds the current update-modal state and exposes actions that call the
 * Rust backend (`check_for_updates`, `skip_update_version`, `remind_update_later`,
 * `open_release_page`). The backend is the source of truth for prefs and
 * suppression rules; the store is a thin reactive view over it.
 *
 * Phase 1: "Update now" opens the release page externally. No in-place
 * installation is performed.
 */
import { writable, get, type Writable } from 'svelte/store';
import * as commands from '@services/commands';
import { onUpdateAvailable } from '@services/events';
import type { UpdateInfo, UpdatePrefs } from '@shared/types/models';

export interface UpdaterState {
  /** Latest update info when a newer version is available; null otherwise. */
  info: UpdateInfo | null;
  /** True while a check is in flight. */
  checking: boolean;
  /** Error message from the last check (cleared on success). */
  error: string | null;
  /** Whether the modal should be visible. */
  modalOpen: boolean;
  /** Persisted prefs (loaded lazily). */
  prefs: UpdatePrefs | null;
}

const initialState: UpdaterState = {
  info: null,
  checking: false,
  error: null,
  modalOpen: false,
  prefs: null,
};

export const updaterStore: Writable<UpdaterState> = writable<UpdaterState>({ ...initialState });

/** Patch the store with a partial update. */
function patch(p: Partial<UpdaterState>): void {
  updaterStore.update((s) => ({ ...s, ...p }));
}

/** Run an update check. When `force` is true, ignores suppression rules
 *  (manual user trigger). On a newer version, opens the modal. */
export async function check(force = false): Promise<UpdateInfo | null> {
  patch({ checking: true, error: null });
  try {
    const info = await commands.checkForUpdates();
    if (info && info.isNewer) {
      patch({ info, modalOpen: true, checking: false });
      return info;
    }
    // No newer version: close the modal if it was open from a previous check.
    patch({ info: null, modalOpen: false, checking: false });
    return null;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    patch({ checking: false, error: msg });
    return null;
  } finally {
    // Refresh prefs so the UI reflects the latest last_check_at.
    try {
      const prefs = await commands.getUpdatePrefs();
      patch({ prefs });
    } catch {
      // ignore prefs refresh failure
    }
  }
  // unreachable — `try` block returns; satisfies TS control-flow analysis.
}

/** User clicked "Update now". In Phase 1 this opens the release page externally. */
export async function updateNow(): Promise<void> {
  const { info } = get(updaterStore);
  if (!info) return;
  try {
    await commands.openReleasePage(info.releaseUrl);
    // We don't close the modal here — the user may come back to dismiss it.
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    patch({ error: msg });
  }
}

/** User clicked "Remind me later". Persists a remind-later timestamp
 *  (default 24h) and dismisses the modal. */
export async function remindLater(hours = 24): Promise<void> {
  try {
    const prefs = await commands.remindUpdateLater(hours);
    patch({ prefs, modalOpen: false });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    patch({ error: msg });
  }
}

/** User clicked "Skip this version". Persists the skipped version and
 *  dismisses the modal. */
export async function skipVersion(): Promise<void> {
  const { info } = get(updaterStore);
  if (!info) return;
  try {
    const prefs = await commands.skipUpdateVersion(info.latestVersion);
    patch({ prefs, modalOpen: false });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    patch({ error: msg });
  }
}

/** Close the modal without persisting any action. */
export function dismissModal(): void {
  patch({ modalOpen: false });
}

/** Load persisted prefs into the store. */
export async function loadPrefs(): Promise<void> {
  try {
    const prefs = await commands.getUpdatePrefs();
    patch({ prefs });
  } catch {
    // ignore — prefs are best-effort
  }
}

/** Subscribe to backend `update-available` events (emitted by the periodic
 *  check loop). When one arrives, surface the modal. Returns an unlisten fn. */
export async function initUpdaterEvents(): Promise<() => void> {
  const unlisten = await onUpdateAvailable((info) => {
    if (info && info.isNewer) {
      patch({ info, modalOpen: true });
    }
  });
  return unlisten;
}

/** Periodic re-check interval id (24h). Stored so tests can clear it. */
let periodicTimer: ReturnType<typeof setInterval> | null = null;

/** Start the 24h periodic re-check. Idempotent — calling twice is a no-op
 *  (the second call returns without starting a duplicate timer). */
export function startPeriodicCheck(): void {
  if (periodicTimer !== null) return;
  const TWENTY_FOUR_HOURS_MS = 24 * 60 * 60 * 1000;
  periodicTimer = setInterval(() => {
    // The backend periodic loop is the source of truth; this frontend timer
    // is a belt-and-suspenders re-check that also refreshes prefs so the UI
    // shows the correct last-check timestamp.
    void check(false);
  }, TWENTY_FOUR_HOURS_MS);
}

/** Stop the periodic re-check (used by tests). */
export function stopPeriodicCheck(): void {
  if (periodicTimer !== null) {
    clearInterval(periodicTimer);
    periodicTimer = null;
  }
}

/** Reset the store to its initial state (used by tests). */
export function resetUpdaterStore(): void {
  stopPeriodicCheck();
  updaterStore.set({ ...initialState });
}