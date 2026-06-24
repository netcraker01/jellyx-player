/**
 * Player store action tests.
 *
 * Verifies that playTrack delegates to the correct Tauri command
 * based on the track source.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { playTrack } from './player';
import { Source } from '@shared/types/models';
import type { Track } from '@shared/types/models';

const mocks = vi.hoisted(() => ({
  playStream: vi.fn(),
  playLocal: vi.fn(),
  push: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  playStream: mocks.playStream,
  playLocal: mocks.playLocal,
  pause: vi.fn(),
  resume: vi.fn(),
  next: vi.fn(),
  previous: vi.fn(),
  seek: vi.fn(),
  setVolume: vi.fn(),
  setShuffle: vi.fn(),
  cycleRepeat: vi.fn(),
  removeFromQueue: vi.fn(),
  clearQueue: vi.fn(),
  playNext: vi.fn(),
}));

vi.mock('@services/events', () => ({
  onTrackChanged: vi.fn(),
  onStateChanged: vi.fn(),
  onQueueUpdated: vi.fn(),
  onProgressTick: vi.fn(),
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: mocks.push,
  },
}));

vi.mock('@features/favorites/stores/favorites', () => ({
  favorites: {
    subscribe: vi.fn(),
  },
}));

vi.mock('@i18n', () => ({
  t: {
    subscribe: vi.fn(() => () => {}),
  },
}));

const remoteTrack: Track = {
  id: 'track:yt:1',
  source: Source.YouTube,
  sourceId: 'yt-1',
  title: 'Remote Track',
  artist: 'Artist',
  streamUrl: 'https://stream.test/track.mp3',
  metadata: {},
};

const localTrack: Track = {
  id: 'track:local:1',
  source: Source.Local,
  sourceId: '/music/track.mp3',
  title: 'Local Track',
  artist: 'Artist',
  localPath: '/music/track.mp3',
  metadata: {},
};

describe('player store > playTrack', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('invokes playStream for remote tracks with a streamUrl', async () => {
    mocks.playStream.mockResolvedValueOnce(undefined);

    await playTrack(remoteTrack);

    expect(mocks.playStream).toHaveBeenCalledWith(remoteTrack);
    expect(mocks.playLocal).not.toHaveBeenCalled();
  });

  it('invokes playLocal for local tracks with a localPath', async () => {
    mocks.playLocal.mockResolvedValueOnce(undefined);

    await playTrack(localTrack);

    expect(mocks.playLocal).toHaveBeenCalledWith('/music/track.mp3');
    expect(mocks.playStream).not.toHaveBeenCalled();
  });
});
