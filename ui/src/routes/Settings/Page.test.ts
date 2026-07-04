/**
 * Settings page tests.
 *
 * Verifies the Settings page mounts safely and renders its core sections.
 *
 * Spec: FR-012 — Settings page is reachable and renders.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  getVersion: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  getVersion: mocks.getVersion,
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
import SettingsPage from './Page.svelte';

describe('Settings page', () => {
  beforeEach(() => {
    mocks.getVersion.mockReset();
    translations.set({
      'app.title': 'Helix',
      'app.tagline': 'Your music, your privacy',
      'settings.title': 'Settings',
      'settings.language': 'Language',
      'settings.about': 'About Helix',
      'settings.version': 'Version',
      'settings.about_description': 'Privacy-first desktop music player. No accounts, no ads, no tracking.',
      'settings.about_repo': 'Source code',
      'settings.about_releases': 'Releases',
      'settings.about_issues': 'Report a bug',
      'settings.about_credits': 'Made with care by netcraker · © 2026 · Licensed under AGPL-3.0',
      'common.loading': 'Loading...',
      'common.error': 'Error',
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
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
  });

  it('renders expanded About Helix content with links and credits', () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    const { container } = render(SettingsPage);
    // Tagline is rendered inside the About section
    expect(container.textContent).toContain('Your music, your privacy');
    // Expanded description
    expect(container.textContent).toContain('Privacy-first desktop music player');
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
