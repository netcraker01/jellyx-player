/**
 * UpdateAvailableModal component tests.
 *
 * Verifies the modal renders when the store has an update and that the
 * three actions (Update now, Remind me later, Skip this version) trigger
 * the correct store handlers. The store itself is tested separately; here
 * we mock it so the component test stays isolated.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent, cleanup } from '@testing-library/svelte';
import type { Writable } from 'svelte/store';
import type { UpdateInfo } from '@shared/types/models';

// All values referenced inside the vi.mock factory MUST be defined inside
// the hoisted block — vi.mock is hoisted above every top-level import.
const hoisted = vi.hoisted(() => {
  type State = {
    info: UpdateInfo | null;
    checking: boolean;
    error: string | null;
    modalOpen: boolean;
    prefs: import('@shared/types/models').UpdatePrefs | null;
  };
  const initial: State = {
    info: null,
    checking: false,
    error: null,
    modalOpen: false,
    prefs: null,
  };
  type Subscriber = (value: State) => void;
  let value: State = { ...initial };
  const subscribers = new Set<Subscriber>();
  const store: Writable<State> = {
    subscribe(run: Subscriber) {
      run(value);
      subscribers.add(run);
      return () => subscribers.delete(run);
    },
    set(next: State) {
      value = next;
      subscribers.forEach((run) => run(value));
    },
    update(fn: (current: State) => State) {
      value = fn(value);
      subscribers.forEach((run) => run(value));
    },
  };
  const actions = {
    updateNow: vi.fn().mockResolvedValue(undefined),
    remindLater: vi.fn().mockResolvedValue(undefined),
    skipVersion: vi.fn().mockResolvedValue(undefined),
    dismissModal: vi.fn(),
  };
  return { store, actions, initial };
});

vi.mock('./updater.store', () => ({
  updaterStore: hoisted.store,
  updateNow: hoisted.actions.updateNow,
  remindLater: hoisted.actions.remindLater,
  skipVersion: hoisted.actions.skipVersion,
  dismissModal: hoisted.actions.dismissModal,
}));

import UpdateAvailableModal from './UpdateAvailableModal.svelte';

function sampleInfo(overrides: Partial<UpdateInfo> = {}): UpdateInfo {
  return {
    currentVersion: '0.2.3',
    latestVersion: '0.2.4',
    body: 'Bug fixes and improvements',
    releaseUrl: 'https://github.com/netcraker01/jellyx-player/releases/tag/v0.2.4',
    publishedAt: '2026-07-07T10:00:00Z',
    channel: 'linux-deb',
    policy: 'open_release_page',
    isNewer: true,
    ...overrides,
  };
}

describe('UpdateAvailableModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hoisted.store.set({ ...hoisted.initial });
  });

  afterEach(() => {
    cleanup();
  });

  it('does not render when modalOpen is false', () => {
    const { container } = render(UpdateAvailableModal);
    expect(container.querySelector('.dialog-overlay')).toBeNull();
  });

  it('renders version row and channel badge when an update is available', () => {
    hoisted.store.set({ ...hoisted.initial, info: sampleInfo(), modalOpen: true });
    const { container } = render(UpdateAvailableModal);

    expect(container.textContent).toContain('0.2.3');
    expect(container.textContent).toContain('0.2.4');
    expect(container.textContent).toContain('linux-deb');
    expect(container.textContent).toContain('Bug fixes and improvements');
  });

  it('hides channel badge when channel is unknown', () => {
    hoisted.store.set({
      ...hoisted.initial,
      info: sampleInfo({ channel: 'unknown' }),
      modalOpen: true,
    });
    const { container } = render(UpdateAvailableModal);
    expect(container.querySelector('.channel-badge')).toBeNull();
  });

  it('Update now button calls updateNow()', async () => {
    hoisted.store.set({ ...hoisted.initial, info: sampleInfo(), modalOpen: true });
    const { container } = render(UpdateAvailableModal);

    const btn = container.querySelector('.btn-primary') as HTMLButtonElement;
    expect(btn).toBeTruthy();
    await fireEvent.click(btn);
    expect(hoisted.actions.updateNow).toHaveBeenCalledTimes(1);
  });

  it('Remind me later button calls remindLater(24)', async () => {
    hoisted.store.set({ ...hoisted.initial, info: sampleInfo(), modalOpen: true });
    const { container } = render(UpdateAvailableModal);

    const buttons = container.querySelectorAll('button');
    const remindBtn = Array.from(buttons).find((b) => b.textContent?.includes('Remind me later'));
    expect(remindBtn).toBeTruthy();
    await fireEvent.click(remindBtn!);
    expect(hoisted.actions.remindLater).toHaveBeenCalledWith(24);
  });

  it('Skip this version button calls skipVersion()', async () => {
    hoisted.store.set({ ...hoisted.initial, info: sampleInfo(), modalOpen: true });
    const { container } = render(UpdateAvailableModal);

    const buttons = container.querySelectorAll('button');
    const skipBtn = Array.from(buttons).find((b) => b.textContent?.includes('Skip this version'));
    expect(skipBtn).toBeTruthy();
    await fireEvent.click(skipBtn!);
    expect(hoisted.actions.skipVersion).toHaveBeenCalledTimes(1);
  });

  it('Escape key calls dismissModal()', async () => {
    hoisted.store.set({ ...hoisted.initial, info: sampleInfo(), modalOpen: true });
    const { container } = render(UpdateAvailableModal);

    const overlay = container.querySelector('.dialog-overlay') as HTMLDivElement;
    expect(overlay).toBeTruthy();
    await fireEvent.keyDown(overlay, { key: 'Escape' });
    expect(hoisted.actions.dismissModal).toHaveBeenCalledTimes(1);
  });

  it('renders error message when store has an error', () => {
    hoisted.store.set({
      ...hoisted.initial,
      info: sampleInfo(),
      modalOpen: true,
      error: 'Network failed',
    });
    const { container } = render(UpdateAvailableModal);
    expect(container.textContent).toContain('Network failed');
    expect(container.querySelector('.error-line')).toBeTruthy();
  });
});
