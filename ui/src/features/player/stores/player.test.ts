/**
 * Player store action tests.
 *
 * Verifies that playTrack delegates to the correct Tauri command
 * based on the track source, and that setVolume scales 0-100 → 0.0-1.0
 * at the IPC boundary (fix for local playback clamping to max).
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { playTrack, setVolume, volume } from './player';
import { Source } from '@shared/types/models';
import type { Track } from '@shared/types/models';

const mocks = vi.hoisted(() => ({
  playStream: vi.fn(),
  playLocal: vi.fn(),
  push: vi.fn(),
  setVolumeCmd: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  playStream: mocks.playStream,
  playLocal: mocks.playLocal,
  pause: vi.fn(),
  resume: vi.fn(),
  next: vi.fn(),
  previous: vi.fn(),
  seek: vi.fn(),
  setVolume: mocks.setVolumeCmd,
  setShuffle: vi.fn(),
  cycleRepeat: vi.fn(),
  removeFromQueue: vi.fn(),
  clearQueue: vi.fn(),
  playNext: vi.fn(),
  getAudioSettings: vi.fn(),
  setPlaybackNormalizeAudio: vi.fn(),
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

describe('player store > setVolume (0-100 → 0.0-1.0 scaling)', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('scales a 0-100 value to 0.0-1.0 before calling the backend', async () => {
    mocks.setVolumeCmd.mockResolvedValueOnce(undefined);
    await setVolume(50);
    // Backend receives 0.5, not 50 (which would clamp to 1.0 = max volume).
    expect(mocks.setVolumeCmd).toHaveBeenCalledWith(0.5);
  });

  it('scales 100 → 1.0 and 0 → 0.0', async () => {
    mocks.setVolumeCmd.mockResolvedValue(undefined);
    await setVolume(100);
    expect(mocks.setVolumeCmd).toHaveBeenLastCalledWith(1);
    await setVolume(0);
    expect(mocks.setVolumeCmd).toHaveBeenLastCalledWith(0);
  });

  it('clamps out-of-range values before scaling', async () => {
    mocks.setVolumeCmd.mockResolvedValue(undefined);
    await setVolume(150);
    expect(mocks.setVolumeCmd).toHaveBeenLastCalledWith(1);
    await setVolume(-20);
    expect(mocks.setVolumeCmd).toHaveBeenLastCalledWith(0);
  });

  it('updates the volume store to the clamped 0-100 value', async () => {
    mocks.setVolumeCmd.mockResolvedValueOnce(undefined);
    await setVolume(42);
    let value = -1;
    const unsub = volume.subscribe((v) => { value = v; });
    expect(value).toBe(42);
    unsub();
  });
});
