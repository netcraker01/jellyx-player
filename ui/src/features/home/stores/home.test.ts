/**
 * Home store tests.
 *
 * Spec: REQ-HS-1, REQ-HE-1
 * Verifies loading, error handling, clear reset, and state transitions.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { homeStore, homeLoading, homeError } from './home';
import { notifications } from '@shared/stores/notifications';
import * as commands from '@services/commands';

// Mock the commands module so we can control IPC responses.
vi.mock('@services/commands', async () => {
  const actual = await vi.importActual<typeof commands>('@services/commands');
  return {
    ...actual,
    getHomeSnapshot: vi.fn(),
    getHomeRecommendations: vi.fn(() => Promise.resolve([])),
  };
});

const mockedGetHomeSnapshot = vi.mocked(commands.getHomeSnapshot);
const mockedGetHomeRecommendations = vi.mocked(commands.getHomeRecommendations);

function createMockSnapshot(): commands.HomeSnapshot {
  return {
    recentlyPlayed: [
      {
        id: 1,
        track: {
          id: 'track-1',
          source: 'Local' as commands.Source,
          sourceId: 'local-1',
          title: 'Recent Track',
          artist: 'Recent Artist',
          metadata: {},
        },
        playedAt: '2026-06-18T10:00:00Z',
      },
    ],
    recommendations: [
      {
        type: 'Track',
        track: {
          id: 'track-2',
          source: 'Local' as commands.Source,
          sourceId: 'local-2',
          title: 'Recommended Track',
          artist: 'Recommended Artist',
          metadata: {},
        },
        reason: 'From your favorites',
      },
    ],
  };
}

// Flush pending promises so background .then() callbacks run.
async function flushPromises() {
  return new Promise<void>((resolve) => setTimeout(resolve, 0));
}

describe('homeStore', () => {
  beforeEach(() => {
    homeStore.clear();
    notifications.clear();
    vi.clearAllMocks();
  });

  it('initializes as null with loading false and no error', () => {
    expect(get(homeStore)).toBeNull();
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it('loads snapshot data and clears loading/error state', async () => {
    const snapshot = createMockSnapshot();
    mockedGetHomeSnapshot.mockResolvedValueOnce({ ...snapshot, recommendations: [] });
    mockedGetHomeRecommendations.mockResolvedValueOnce(snapshot.recommendations);

    const promise = homeStore.load();
    expect(get(homeLoading)).toBe(true);
    expect(get(homeError)).toBeNull();

    await promise;

    const state = get(homeStore);
    expect(state).not.toBeNull();
    expect(state!.recentlyPlayed).toHaveLength(1);
    expect(state!.recentlyPlayed[0].track.title).toBe('Recent Track');
    // Recommendations arrive asynchronously after the main snapshot.
    await flushPromises();
    expect(state!.recommendations).toHaveLength(1);
    expect(state!.recommendations[0].type).toBe('Track');
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it('sets error state and pushes a toast notification when load fails', async () => {
    mockedGetHomeSnapshot.mockRejectedValueOnce(new Error('IPC failure'));

    await homeStore.load();

    expect(get(homeStore)).toBeNull();
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBe('IPC failure');

    const notifs = get(notifications);
    expect(notifs).toHaveLength(1);
    expect(notifs[0].type).toBe('error');
    expect(notifs[0].title).toBe('Home Error');
    expect(notifs[0].message).toBe('IPC failure');
  });

  it('resets all state when clear() is called', async () => {
    const snapshot = createMockSnapshot();
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();
    expect(get(homeStore)).not.toBeNull();

    homeStore.clear();

    expect(get(homeStore)).toBeNull();
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it('handles an empty snapshot gracefully', async () => {
    const emptySnapshot: commands.HomeSnapshot = {
      recentlyPlayed: [],
      recommendations: [],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(emptySnapshot);

    await homeStore.load();

    const state = get(homeStore);
    expect(state).not.toBeNull();
    expect(state!.recentlyPlayed).toHaveLength(0);
    expect(state!.recommendations).toHaveLength(0);
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it(' narrows RecommendationItem union by type', () => {
    const item: commands.RecommendationItem = {
      type: 'Artist',
      id: 'artist-1',
      name: 'Artist One',
      trackCount: 5,
      reason: 'Because you listened',
    };

    expect(item.type).toBe('Artist');
    expect('track' in item).toBe(false);
    expect(item.name).toBe('Artist One');
  });
});
