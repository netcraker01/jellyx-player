/**
 * Route rendering tests for /library and /settings.
 *
 * Verifies the app router actually renders the Library and Settings
 * page content when their respective routes are active.
 *
 * Spec: FR-011 — App shell navigation exposes all routes.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { initI18n } from '@i18n';
import App from '../app/App.svelte';

const mocks = vi.hoisted(() => ({
  loadWatchedFolders: vi.fn(),
  loadLocalTracks: vi.fn(),
  getVersion: vi.fn(),
}));

vi.mock('@features/library/stores/library', () => ({
  watchedFolders: { subscribe: (fn: any) => { fn([]); return () => {}; }, set: vi.fn() },
  localTracks: { subscribe: (fn: any) => { fn([]); return () => {}; }, set: vi.fn() },
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
  });

  it('renders the Library page at /library', () => {
    setHash('#/library');
    const { container } = render(App);
    expect(container.textContent).toContain('Local Library');
  });

  it('renders the Settings page at /settings', () => {
    setHash('#/settings');
    const { container } = render(App);
    expect(container.textContent).toContain('Settings');
  });
});
