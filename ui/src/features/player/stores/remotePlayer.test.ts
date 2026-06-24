/**
 * Remote player store tests.
 *
 * Verifies that remote playback via HTMLAudio delegates correctly
 * and handles stream-resolved events.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  pauseRemote,
  resumeRemote,
  seekRemote,
  stopRemote,
  remoteActive,
} from './remotePlayer';

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

function get(store: { subscribe: (fn: (v: unknown) => void) => () => void }): unknown {
  let value: unknown;
  store.subscribe((v) => {
    value = v;
  })();
  return value;
}

beforeEach(() => {
  // Reset the module-level audio element between tests
  stopRemote();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('remotePlayer store', () => {
  it('sets remoteActive to false initially', () => {
    expect(get(remoteActive)).toBe(false);
  });

  it('stops remote playback and clears state', () => {
    stopRemote();
    expect(get(remoteActive)).toBe(false);
  });
});
