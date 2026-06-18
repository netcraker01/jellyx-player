/**
 * Home page store and integration tests.
 *
 * Spec: REQ-HS-1, REQ-HR-2, REQ-HE-1
 *
 * Tests the home store lifecycle and command integration.
 * Component rendering tests focus on store-driven state rather than
 * full DOM rendering, since HomePage uses svelte-routing's Link which
 * requires a Router context not available in jsdom.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { homeStore, homeLoading, homeError } from '@features/home/stores/home';
import * as commands from '@services/commands';
import type { HomeSnapshot, RecommendationItem, Track, HistoryEntry, Source } from '@shared/types/models';

vi.mock('@services/commands', async () => {
  const actual = await vi.importActual<typeof commands>('@services/commands');
  return {
    ...actual,
    getHomeSnapshot: vi.fn(),
  };
});

const mockedGetHomeSnapshot = vi.mocked(commands.getHomeSnapshot);

function makeTrack(id: string, title: string, artist: string): Track {
  return {
    id,
    source: 'Local' as Source,
    sourceId: id,
    title,
    artist,
    metadata: {},
  };
}

function makeHistoryEntry(id: number, track: Track): HistoryEntry {
  return { id, track, playedAt: '2026-06-18T10:00:00Z' };
}

function makeRecommendation(type: RecommendationItem['type'], reason: string): RecommendationItem {
  if (type === 'Track') {
    return { type: 'Track', track: makeTrack('rec-track-1', 'Recommended Track', 'Rec Artist'), reason };
  }
  if (type === 'Artist') {
    return { type: 'Artist', id: 'artist-1', name: 'Artist One', trackCount: 5, reason };
  }
  return { type: 'Album', id: 'album-1', title: 'Album One', artist: 'Album Artist', trackCount: 8, reason };
}

describe('Home store integration', () => {
  beforeEach(() => {
    homeStore.clear();
    mockedGetHomeSnapshot.mockReset();
  });

  it('starts with null snapshot and false loading', () => {
    expect(get(homeStore)).toBeNull();
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it('loads a complete snapshot with recently played and recommendations (REQ-HS-1)', async () => {
    const snapshot: HomeSnapshot = {
      recentlyPlayed: [makeHistoryEntry(1, makeTrack('track-1', 'Recent Track', 'Recent Artist'))],
      recommendations: [
        makeRecommendation('Track', 'From your favorites'),
        makeRecommendation('Artist', 'Because you listened to Rec Artist'),
        makeRecommendation('Album', 'Based on your listening'),
      ],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();

    const result = get(homeStore);
    expect(result).not.toBeNull();
    expect(result!.recentlyPlayed).toHaveLength(1);
    expect(result!.recommendations).toHaveLength(3);
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it('loads empty snapshot and both sections are empty (REQ-HE-1)', async () => {
    mockedGetHomeSnapshot.mockResolvedValueOnce({ recentlyPlayed: [], recommendations: [] });

    await homeStore.load();

    const result = get(homeStore);
    expect(result).not.toBeNull();
    expect(result!.recentlyPlayed).toHaveLength(0);
    expect(result!.recommendations).toHaveLength(0);
    expect(get(homeError)).toBeNull();
  });

  it('sets error state when load fails', async () => {
    mockedGetHomeSnapshot.mockRejectedValueOnce(new Error('IPC failure'));

    await homeStore.load();

    expect(get(homeStore)).toBeNull();
    expect(get(homeError)).toBe('IPC failure');
    expect(get(homeLoading)).toBe(false);
  });

  it('clears state on clear()', async () => {
    const snapshot: HomeSnapshot = {
      recentlyPlayed: [makeHistoryEntry(1, makeTrack('track-1', 'Track', 'Artist'))],
      recommendations: [],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();
    expect(get(homeStore)).not.toBeNull();

    homeStore.clear();

    expect(get(homeStore)).toBeNull();
    expect(get(homeLoading)).toBe(false);
    expect(get(homeError)).toBeNull();
  });

  it('loads partial data with only recently played (REQ-HE-1)', async () => {
    const snapshot: HomeSnapshot = {
      recentlyPlayed: [makeHistoryEntry(1, makeTrack('track-1', 'Only Recent', 'Only Artist'))],
      recommendations: [],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();

    const result = get(homeStore);
    expect(result!.recentlyPlayed).toHaveLength(1);
    expect(result!.recommendations).toHaveLength(0);
  });

  it('loads partial data with only recommendations (REQ-HE-1)', async () => {
    const snapshot: HomeSnapshot = {
      recentlyPlayed: [],
      recommendations: [makeRecommendation('Track', 'Discover from your library')],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();

    const result = get(homeStore);
    expect(result!.recentlyPlayed).toHaveLength(0);
    expect(result!.recommendations).toHaveLength(1);
    expect(result!.recommendations[0].type).toBe('Track');
    expect(result!.recommendations[0].reason).toBe('Discover from your library');
  });

  it('recommendation items have reason labels (REQ-HR-2)', async () => {
    const snapshot: HomeSnapshot = {
      recentlyPlayed: [],
      recommendations: [
        { type: 'Track' as const, track: makeTrack('t1', 'Track One', 'Artist A'), reason: 'From your favorites' },
        { type: 'Artist' as const, id: 'a1', name: 'Artist A', trackCount: 3, reason: 'Because you listened to Artist A' },
        { type: 'Album' as const, id: 'al1', title: 'Album A', artist: 'Artist A', trackCount: 4, reason: 'Based on your listening' },
      ],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();

    const result = get(homeStore);
    expect(result!.recommendations).toHaveLength(3);
    for (const item of result!.recommendations) {
      expect(item.reason).toBeTruthy();
    }
    expect(result!.recommendations[0].reason).toBe('From your favorites');
    expect(result!.recommendations[1].reason).toBe('Because you listened to Artist A');
    expect(result!.recommendations[2].reason).toBe('Based on your listening');
  });

  it('retry after error succeeds with data', async () => {
    mockedGetHomeSnapshot.mockRejectedValueOnce(new Error('IPC failure'));
    const snapshot: HomeSnapshot = {
      recentlyPlayed: [makeHistoryEntry(1, makeTrack('track-1', 'Retry Track', 'Retry Artist'))],
      recommendations: [],
    };
    mockedGetHomeSnapshot.mockResolvedValueOnce(snapshot);

    await homeStore.load();
    expect(get(homeError)).toBe('IPC failure');

    await homeStore.load();
    expect(get(homeStore)!.recentlyPlayed).toHaveLength(1);
    expect(get(homeError)).toBeNull();
  });
});