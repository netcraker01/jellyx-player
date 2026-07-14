/**
 * Route rendering tests for /library and /settings.
 *
 * Verifies the app router actually renders the Library and Settings
 * page content when their respective routes are active.
 *
 * Spec: FR-011 — App shell navigation exposes all routes.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import { initI18n } from '@i18n';
import App from '../app/App.svelte';

const mocks = vi.hoisted(() => ({
  loadWatchedFolders: vi.fn(),
  loadLocalTracks: vi.fn(),
  getVersion: vi.fn(),
  getSourceSettings: vi.fn(),
  getTelemetrySettings: vi.fn(),
}));

vi.mock('@features/library/stores/library', () => ({
  watchedFolders: { subscribe: (fn: any) => { fn([]); return () => {}; }, set: vi.fn() },
  localTracks: { subscribe: (fn: any) => { fn([]); return () => {}; }, set: vi.fn() },
  tracksByFolder: { subscribe: (fn: any) => { fn(new Map()); return () => {}; } },
  isScanning: { subscribe: (fn: any) => { fn(false); return () => {}; }, set: vi.fn() },
  scanStatus: { subscribe: (fn: any) => { fn(null); return () => {}; }, set: vi.fn() },
  scanError: { subscribe: (fn: any) => { fn(null); return () => {}; }, set: vi.fn() },
  loadWatchedFolders: mocks.loadWatchedFolders,
  loadLocalTracks: mocks.loadLocalTracks,
  scanNewFolder: vi.fn(),
  removeFolder: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  getVersion: mocks.getVersion,
  getSourceSettings: mocks.getSourceSettings,
  getTelemetrySettings: mocks.getTelemetrySettings,
  setSourceEnabled: vi.fn(),
  setTelemetryEnabled: vi.fn(),
  getAudioSettings: vi.fn(),
  setPlaybackNormalizeAudio: vi.fn(),
  setNormalizeAudio: vi.fn(),
}));

function setHash(hash: string) {
  window.history.replaceState({}, '', '/' + hash);
  window.dispatchEvent(new HashChangeEvent('hashchange'));
}

describe('Route rendering', () => {
  beforeEach(async () => {
    window.history.replaceState({}, '', '/');
    window.location.hash = '';
    await initI18n();
    mocks.loadWatchedFolders.mockReset().mockResolvedValue(undefined);
    mocks.loadLocalTracks.mockReset().mockResolvedValue(undefined);
    mocks.getVersion.mockReset().mockResolvedValue('0.1.0');
    mocks.getSourceSettings.mockReset().mockResolvedValue([]);
    mocks.getTelemetrySettings.mockReset().mockResolvedValue({ enabled: false });
  });

  // App mounts route-level subscriptions. Explicitly destroy each render so a
  // worker never retains route listeners while Vitest is tearing it down.
  afterEach(() => cleanup());

  it('renders the Library page at /library', () => {
    setHash('#/library');
    const { container } = render(App);
    expect(container.textContent).toContain('Local File');
  });

  it('renders the Settings page at /settings', () => {
    setHash('#/settings');
    const { container } = render(App);
    expect(container.textContent).toContain('Settings');
  });
});
