import { describe, it, expect, beforeEach, vi } from 'vitest';
import { fireEvent, render } from '@testing-library/svelte';
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

describe('App sidebar runtime navigation', () => {
  beforeEach(async () => {
    window.history.replaceState({}, '', '/');
    window.location.hash = '';
    await initI18n();
    mocks.loadWatchedFolders.mockReset().mockResolvedValue(undefined);
    mocks.loadLocalTracks.mockReset().mockResolvedValue(undefined);
    mocks.getVersion.mockReset().mockResolvedValue('0.1.0');
  });

  it('navigates to Library from the real App sidebar', async () => {
    const { getByText, container } = render(App);
    await fireEvent.click(getByText('Library'));
    expect(window.location.hash).toBe('#/library');
    expect(container.textContent).toContain('Local Library');
  });

  it('navigates to Favorites from the real App sidebar', async () => {
    const { getByText, container } = render(App);
    await fireEvent.click(getByText('Favorites'));
    expect(window.location.hash).toBe('#/favorites');
    expect(container.textContent).toContain('Favorites');
  });
});
