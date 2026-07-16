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
  getTelemetrySettings: vi.fn(),
  setTelemetryEnabled: vi.fn(),
  getFailureDiagnostics: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  getVersion: mocks.getVersion,
  getSourceSettings: mocks.getSourceSettings,
  setSourceEnabled: mocks.setSourceEnabled,
  getAudioSettings: mocks.getAudioSettings,
  getTelemetrySettings: mocks.getTelemetrySettings,
  setTelemetryEnabled: mocks.setTelemetryEnabled,
  getFailureDiagnostics: mocks.getFailureDiagnostics,
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
    mocks.getTelemetrySettings.mockReset();
    mocks.setTelemetryEnabled.mockReset();
    mocks.getFailureDiagnostics.mockReset();
    mocks.getSourceSettings.mockResolvedValue([]);
    mocks.getAudioSettings.mockResolvedValue({ normalizeAudio: true });
    mocks.getTelemetrySettings.mockResolvedValue({ enabled: false });
    mocks.getFailureDiagnostics.mockResolvedValue({ eventsLastHour: 2, errorRatePercent: 1.5, counters: {}, recentEvents: [], operationRates: {}, latency: {}, alerts: [] });
    activateMiniPlayerSkin('classic-jellyx');
    setMiniPlayerScale(1);
    translations.set({
      'app.title': 'Jellyx',
      'app.tagline': 'Background music for long work sessions',
      'settings.title': 'Settings',
      'settings.privacy': 'Privacy',
      'settings.telemetry': 'Share anonymous failure signals',
      'settings.telemetry_desc': 'Off by default.',
      'settings.telemetry_details': 'Turn this off at any time.',
      'settings.language': 'Language',
      'settings.about': 'About Jellyx Player',
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
      'settings.diagnostics_title': 'Local diagnostics',
      'settings.diagnostics_desc': 'A local, redacted summary for troubleshooting.',
      'settings.diagnostics_events_last_hour': 'Events in the last hour',
      'settings.diagnostics_error_rate': 'Observed error rate',
      'settings.diagnostics_unavailable': 'Diagnostics are temporarily unavailable.',
      'settings.diagnostics_refresh': 'Refresh diagnostics',
      'settings.diagnostics_refreshing': 'Refreshing…',
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
    expect(container.textContent).toContain('About Jellyx Player');
    expect(container.textContent).toContain('Version');
    expect(container.textContent).toContain('Mini player skins');
  });

  it('persists telemetry only after an explicit user toggle', async () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);
    const toggle = screen.getByRole<HTMLInputElement>('checkbox', { name: 'Share anonymous failure signals' });
    expect(toggle.checked).toBe(false);
    await fireEvent.click(toggle);
    expect(mocks.setTelemetryEnabled).toHaveBeenCalledWith(true);
  });

  it('shows local diagnostics and refreshes them without telemetry opt-in', async () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);
    await vi.waitFor(() => expect(screen.getByText('Events in the last hour')).toBeTruthy());
    expect(screen.getByText('2')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Refresh diagnostics' }));
    expect(mocks.getFailureDiagnostics).toHaveBeenCalledTimes(2);
  });

  it.each([
    { initial: false, stale: false, expected: true, label: 'opt-in' },
    { initial: true, stale: true, expected: false, label: 'opt-out' },
  ])('keeps the successful $label toggle when the initial consent read resolves late', async ({ initial, stale, expected }) => {
    let resolveInitial!: (value: { enabled: boolean }) => void;
    mocks.getTelemetrySettings.mockImplementationOnce(() => new Promise((resolve) => { resolveInitial = resolve; }));
    mocks.setTelemetryEnabled.mockResolvedValue(undefined);
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);

    const toggle = screen.getByRole<HTMLInputElement>('checkbox', { name: 'Share anonymous failure signals' });
    if (initial) {
      // Establish the initial UI state before the user action in this sequence.
      resolveInitial({ enabled: initial });
      await Promise.resolve();
    }
    await fireEvent.click(toggle);
    expect(mocks.setTelemetryEnabled).toHaveBeenCalledWith(expected);

    if (!initial) resolveInitial({ enabled: stale });
    await Promise.resolve();
    expect(toggle.checked).toBe(expected);
  });

  it('serializes rapid telemetry writes so the latest action persists last', async () => {
    let resolveFirst!: () => void;
    let resolveSecond!: () => void;
    mocks.setTelemetryEnabled
      .mockImplementationOnce(() => new Promise<void>((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise<void>((resolve) => { resolveSecond = resolve; }));
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);

    const toggle = screen.getByRole<HTMLInputElement>('checkbox', { name: 'Share anonymous failure signals' });
    await fireEvent.click(toggle);
    await fireEvent.click(toggle);
    expect(mocks.setTelemetryEnabled).toHaveBeenCalledTimes(1);
    expect(mocks.setTelemetryEnabled).toHaveBeenLastCalledWith(true);

    resolveFirst();
    await vi.waitFor(() => expect(mocks.setTelemetryEnabled).toHaveBeenCalledTimes(2));
    expect(mocks.setTelemetryEnabled).toHaveBeenLastCalledWith(false);
    resolveSecond();
    await Promise.resolve();
    expect(toggle.checked).toBe(false);
  });

  it('recovers a queued consent write after the prior write fails', async () => {
    let rejectFirst!: (error: Error) => void;
    mocks.setTelemetryEnabled
      .mockImplementationOnce(() => new Promise<void>((_, reject) => { rejectFirst = reject; }))
      .mockResolvedValueOnce(undefined);
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);

    const toggle = screen.getByRole<HTMLInputElement>('checkbox', { name: 'Share anonymous failure signals' });
    await fireEvent.click(toggle);
    await fireEvent.click(toggle);
    rejectFirst(new Error('temporary persistence failure'));

    await vi.waitFor(() => expect(mocks.setTelemetryEnabled).toHaveBeenCalledTimes(2));
    expect(mocks.setTelemetryEnabled).toHaveBeenLastCalledWith(false);
    await vi.waitFor(() => expect(toggle.checked).toBe(false));
  });

  it('activates the Graphite Jellyx mini-player skin and updates active state', async () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    render(SettingsPage);

    expect(get(selectedMiniPlayerSkinId)).toBe('classic-jellyx');
    expect(screen.getByText('Graphite Jellyx')).toBeTruthy();
    expect(screen.getByText('A dark graphite skin with the same compact design.')).toBeTruthy();
    const activateButton = screen.getByRole<HTMLButtonElement>('button', { name: 'Activate Graphite Jellyx' });

    expect(activateButton.textContent).toBe('Activate');
    await fireEvent.click(activateButton);

    expect(get(selectedMiniPlayerSkinId)).toBe('graphite-jellyx');
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Active Graphite Jellyx' }).disabled).toBe(true);
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
    expect(localStorage.getItem('jellyx-mini-player-scale')).toBe('0.35');
  });

  it('renders expanded About Jellyx Player content with links and credits', () => {
    mocks.getVersion.mockResolvedValueOnce('0.1.0');
    const { container } = render(SettingsPage);
    // Tagline is rendered inside the About section
    expect(container.textContent).toContain('Background music for long work sessions');
    // Expanded description
    expect(container.textContent).toContain('Desktop background music player');
    // Credits mention netcraker and 2026
    expect(container.textContent).toContain('netcraker');
    expect(container.textContent).toContain('2026');
    // Repository links target the canonical GitHub repo (repo rename handled in PR 6)
    const links = container.querySelectorAll<HTMLAnchorElement>('.about-link');
    expect(links.length).toBe(3);
    for (const a of links) {
      expect(a.href).toContain('https://github.com/netcraker01/jellyx-player');
      expect(a.getAttribute('target')).toBe('_blank');
      expect(a.getAttribute('rel')).toBe('noopener noreferrer');
    }
  });
});
