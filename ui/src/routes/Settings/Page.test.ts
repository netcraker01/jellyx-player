/**
 * Settings page tests.
 *
 * Verifies the Settings page mounts safely and renders its core sections.
 *
 * Spec: FR-012 — Settings page is reachable and renders.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  getVersion: vi.fn(),
  getSourceSettings: vi.fn(),
  setSourceEnabled: vi.fn(),
  getAudioSettings: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  getVersion: mocks.getVersion,
  getSourceSettings: mocks.getSourceSettings,
  setSourceEnabled: mocks.setSourceEnabled,
  getAudioSettings: mocks.getAudioSettings,
  setPlaybackNormalizeAudio: vi.fn(),
  setNormalizeAudio: vi.fn(),
}));

vi.mock('@i18n', () => {
  const translations = new Map<string, string>();
  const locale = { set: vi.fn(), subscribe: vi.fn(() => () => {}) };
  const t = {
    subscribe(fn: (value: unknown) => void) {
      fn((key: string) => translations.get(key) ?? key);
      return () => {};
    },
  };
  return {
    locale,
    t,
    translations: {
      set(map: Record<string, string>) {
        translations.clear();
        for (const [k, v] of Object.entries(map)) {
          translations.set(k, v);
        }
      },
    },
    switchLocale: vi.fn(),
  };
});

import { translations } from '@i18n';
import { activateMiniPlayerSkin, miniPlayerScale, selectedMiniPlayerSkinId, setMiniPlayerScale } from '@features/mini-player/skins';
import SettingsPage from './Page.svelte';
import { get } from 'svelte/store';

describe('Settings page', () => {
  beforeEach(() => {
    const values = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    });
    mocks.getVersion.mockReset();
    mocks.getSourceSettings.mockReset();
    mocks.setSourceEnabled.mockReset();
    mocks.getAudioSettings.mockReset();
    mocks.getSourceSettings.mockResolvedValue([]);
    mocks.getAudioSettings.mockResolvedValue({ normalizeAudio: true });
    activateMiniPlayerSkin('ipod-classic');
    setMiniPlayerScale(1);
    translations.set({
      'app.title': 'Helix',
      'app.tagline': 'Background music for long work sessions',
      'settings.title': 'Settings',
      'settings.language': 'Language',
      'settings.about': 'About Helix',
      'settings.version': 'Version',
      'settings.about_description': 'Desktop background music player for people who work with music on. Listen to YouTube, SoundCloud and local files without accounts, subscriptions or unnecessary video playback.',
      'settings.about_repo': 'Source code',
      'settings.about_releases': 'Releases',
      'settings.about_issues': 'Report a bug',
      'settings.about_credits': 'Made with care by netcraker · © 2026 · Licensed under AGPL-3.0',
      'settings.mini_player_skins': 'Mini player skins',
      'settings.mini_player_skins_desc': 'Choose the declarative skin used by the mini player.',
      'settings.mini_player_size': 'Mini player size',
      'settings.skin_activate': 'Activate',
      'settings.skin_active': 'Active',
      'common.loading': 'Loading...',
      'common.error': 'Error',
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('mounts safely and renders the settings heading', () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    const { container } = render(SettingsPage);
    expect(container.textContent).toContain('Settings');
  });

  it('renders language and about sections', () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    const { container } = render(SettingsPage);
    expect(container.textContent).toContain('Language');
    expect(container.textContent).toContain('About Helix');
    expect(container.textContent).toContain('Version');
    expect(container.textContent).toContain('Mini player skins');
  });

  it('activates the Classic mini-player skin and updates active state', async () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);

    expect(get(selectedMiniPlayerSkinId)).toBe('ipod-classic');
    expect(screen.getByText('Classic')).toBeTruthy();
    expect(screen.getByText('A compact horizontal hi-fi skin with dark hardware, amber display, and tactile transport controls.')).toBeTruthy();
    const activateButton = screen.getByRole<HTMLButtonElement>('button', { name: 'Activate Classic' });

    expect(activateButton.textContent).toBe('Activate');
    await fireEvent.click(activateButton);

    expect(get(selectedMiniPlayerSkinId)).toBe('winamp-classic');
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Active Classic' }).disabled).toBe(true);
  });

  it('updates the persisted mini-player size scale from the slider', async () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);

    const slider = screen.getByRole<HTMLInputElement>('slider', { name: 'Mini player size' });
    expect(slider.min).toBe('0.3');
    expect(slider.max).toBe('1');
    expect(screen.getByText('30%–100%')).toBeTruthy();

    await fireEvent.input(slider, { target: { value: '0.35' } });

    expect(get(miniPlayerScale)).toBe(0.35);
    expect(localStorage.getItem('helix-mini-player-scale')).toBe('0.35');
  });

  it('renders expanded About Helix content with links and credits', () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    const { container } = render(SettingsPage);
    // Tagline is rendered inside the About section
    expect(container.textContent).toContain('Background music for long work sessions');
    // Expanded description
    expect(container.textContent).toContain('Desktop background music player');
    // Credits mention netcraker and 2026
    expect(container.textContent).toContain('netcraker');
    expect(container.textContent).toContain('2026');
    // Repository links target the canonical GitHub repo
    const links = container.querySelectorAll<HTMLAnchorElement>('.about-link');
    expect(links.length).toBe(3);
    for (const a of links) {
      expect(a.href).toContain('https://github.com/netcraker01/helix');
      expect(a.getAttribute('target')).toBe('_blank');
      expect(a.getAttribute('rel')).toBe('noopener noreferrer');
    }
  });
});
