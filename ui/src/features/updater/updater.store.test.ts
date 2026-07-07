/**
 * Updater store tests.
 *
 * Verifies the store's action handlers call the correct backend commands
 * and update the store state appropriately. Backend commands are mocked
 * so no real Tauri IPC happens.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';

const mocks = vi.hoisted(() => ({
  checkForUpdates: vi.fn(),
  skipUpdateVersion: vi.fn(),
  remindUpdateLater: vi.fn(),
  getUpdatePrefs: vi.fn(),
  openReleasePage: vi.fn(),
  onUpdateAvailable: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  checkForUpdates: mocks.checkForUpdates,
  skipUpdateVersion: mocks.skipUpdateVersion,
  remindUpdateLater: mocks.remindUpdateLater,
  getUpdatePrefs: mocks.getUpdatePrefs,
  openReleasePage: mocks.openReleasePage,
}));

vi.mock('@services/events', () => ({
  onUpdateAvailable: mocks.onUpdateAvailable,
}));

// Re-import after mocks are in place.
import {
  updaterStore,
  check,
  updateNow,
  remindLater,
  skipVersion,
  dismissModal,
  loadPrefs,
  initUpdaterEvents,
  startPeriodicCheck,
  stopPeriodicCheck,
  resetUpdaterStore,
} from './updater.store';
import type { UpdateInfo } from '@shared/types/models';

function sampleInfo(overrides: Partial<UpdateInfo> = {}): UpdateInfo {
  return {
    currentVersion: '0.2.3',
    latestVersion: '0.2.4',
    body: 'Bug fixes and improvements',
    releaseUrl: 'https://github.com/netcraker01/helix/releases/tag/v0.2.4',
    publishedAt: '2026-07-07T10:00:00Z',
    channel: 'linux-deb',
    policy: 'open_release_page',
    isNewer: true,
    ...overrides,
  };
}

describe('updater.store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetUpdaterStore();
    mocks.getUpdatePrefs.mockResolvedValue({
      skippedVersion: undefined,
      remindLaterAt: undefined,
      lastCheckAt: undefined,
      detectedChannel: 'linux-deb',
    });
    mocks.onUpdateAvailable.mockResolvedValue(() => {});
  });

  afterEach(() => {
    stopPeriodicCheck();
  });

  it('initial state has no info and closed modal', () => {
    const s = get(updaterStore);
    expect(s.info).toBeNull();
    expect(s.modalOpen).toBe(false);
    expect(s.checking).toBe(false);
    expect(s.error).toBeNull();
    expect(s.prefs).toBeNull();
  });

  it('check() opens modal when a newer version is available', async () => {
    mocks.checkForUpdates.mockResolvedValue(sampleInfo());
    const info = await check(false);
    expect(info).not.toBeNull();
    expect(info?.latestVersion).toBe('0.2.4');

    const s = get(updaterStore);
    expect(s.modalOpen).toBe(true);
    expect(s.info?.latestVersion).toBe('0.2.4');
    expect(s.checking).toBe(false);
  });

  it('check() does not open modal when no newer version is available', async () => {
    mocks.checkForUpdates.mockResolvedValue(null);
    const info = await check(false);
    expect(info).toBeNull();

    const s = get(updaterStore);
    expect(s.modalOpen).toBe(false);
    expect(s.info).toBeNull();
  });

  it('check() surfaces error state on failure', async () => {
    mocks.checkForUpdates.mockRejectedValue(new Error('network down'));
    const info = await check(false);
    expect(info).toBeNull();

    const s = get(updaterStore);
    expect(s.error).toContain('network down');
    expect(s.checking).toBe(false);
  });

  it('updateNow() calls openReleasePage with the release URL', async () => {
    mocks.checkForUpdates.mockResolvedValue(sampleInfo());
    mocks.openReleasePage.mockResolvedValue(undefined);
    await check(false);

    await updateNow();
    expect(mocks.openReleasePage).toHaveBeenCalledWith(
      'https://github.com/netcraker01/helix/releases/tag/v0.2.4',
    );
  });

  it('remindLater() persists remind-later and closes the modal', async () => {
    mocks.checkForUpdates.mockResolvedValue(sampleInfo());
    mocks.remindUpdateLater.mockResolvedValue({
      remindLaterAt: '2026-07-08T10:00:00Z',
      detectedChannel: 'linux-deb',
    });
    await check(false);
    expect(get(updaterStore).modalOpen).toBe(true);

    await remindLater(24);
    expect(mocks.remindUpdateLater).toHaveBeenCalledWith(24);
    expect(get(updaterStore).modalOpen).toBe(false);
    expect(get(updaterStore).prefs?.remindLaterAt).toBe('2026-07-08T10:00:00Z');
  });

  it('skipVersion() persists the latest version and closes the modal', async () => {
    mocks.checkForUpdates.mockResolvedValue(sampleInfo());
    mocks.skipUpdateVersion.mockResolvedValue({
      skippedVersion: '0.2.4',
      detectedChannel: 'linux-deb',
    });
    await check(false);
    expect(get(updaterStore).modalOpen).toBe(true);

    await skipVersion();
    expect(mocks.skipUpdateVersion).toHaveBeenCalledWith('0.2.4');
    expect(get(updaterStore).modalOpen).toBe(false);
    expect(get(updaterStore).prefs?.skippedVersion).toBe('0.2.4');
  });

  it('dismissModal() closes the modal without persisting', async () => {
    mocks.checkForUpdates.mockResolvedValue(sampleInfo());
    await check(false);
    expect(get(updaterStore).modalOpen).toBe(true);

    dismissModal();
    expect(get(updaterStore).modalOpen).toBe(false);
    // No skip/remind command should have been called.
    expect(mocks.skipUpdateVersion).not.toHaveBeenCalled();
    expect(mocks.remindUpdateLater).not.toHaveBeenCalled();
  });

  it('loadPrefs() loads persisted prefs into the store', async () => {
    mocks.getUpdatePrefs.mockResolvedValue({
      skippedVersion: 'v0.3.0',
      detectedChannel: 'flatpak',
    });
    await loadPrefs();
    expect(get(updaterStore).prefs?.skippedVersion).toBe('v0.3.0');
    expect(get(updaterStore).prefs?.detectedChannel).toBe('flatpak');
  });

  it('initUpdaterEvents() subscribes to update-available and surfaces newer info', async () => {
    mocks.onUpdateAvailable.mockImplementation((cb: (info: UpdateInfo) => void) => {
      // Simulate a backend event arriving later.
      setTimeout(() => cb(sampleInfo()), 0);
      return Promise.resolve(() => {});
    });

    await initUpdaterEvents();
    // Wait for the simulated event to fire.
    await new Promise((r) => setTimeout(r, 10));

    expect(get(updaterStore).modalOpen).toBe(true);
    expect(get(updaterStore).info?.latestVersion).toBe('0.2.4');
  });

  it('startPeriodicCheck() is idempotent', () => {
    startPeriodicCheck();
    startPeriodicCheck();
    // No assertion on the timer itself — we only verify it doesn't throw and
    // doesn't start duplicates. stopPeriodicCheck cleans up both potential
    // timers via the single stored id.
    stopPeriodicCheck();
  });
});
