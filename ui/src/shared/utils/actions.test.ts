/**
 * Shared action tests.
 *
 * Verifies that playTrack dispatches to the correct Tauri command
 * depending on whether the track is local or remote.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { playTrack } from './actions';
import { Source } from '@shared/types/models';
import type { Track } from '@shared/types/models';

const mocks = vi.hoisted(() => ({
  play: vi.fn(),
  playLocal: vi.fn(),
  push: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  play: mocks.play,
  playLocal: mocks.playLocal,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: mocks.push,
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

describe('playTrack', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('invokes play for remote tracks with a streamUrl', async () => {
    mocks.play.mockResolvedValueOnce(undefined);

    await playTrack(remoteTrack);

    expect(mocks.play).toHaveBeenCalledWith('https://stream.test/track.mp3');
    expect(mocks.playLocal).not.toHaveBeenCalled();
  });

  it('invokes playLocal for local tracks with a localPath', async () => {
    mocks.playLocal.mockResolvedValueOnce(undefined);

    await playTrack(localTrack);

    expect(mocks.playLocal).toHaveBeenCalledWith('/music/track.mp3');
    expect(mocks.play).not.toHaveBeenCalled();
  });

  it('prefers playLocal when both streamUrl and localPath are present', async () => {
    mocks.playLocal.mockResolvedValueOnce(undefined);

    const mixed: Track = { ...remoteTrack, localPath: '/music/track.mp3' };
    await playTrack(mixed);

    expect(mocks.playLocal).toHaveBeenCalledWith('/music/track.mp3');
    expect(mocks.play).not.toHaveBeenCalled();
  });

  it('does nothing when the track has neither streamUrl nor localPath', async () => {
    const empty: Track = {
      id: 'track:empty',
      source: Source.YouTube,
      sourceId: 'yt-empty',
      title: 'Empty',
      artist: 'Artist',
      metadata: {},
    };

    await playTrack(empty);

    expect(mocks.play).not.toHaveBeenCalled();
    expect(mocks.playLocal).not.toHaveBeenCalled();
  });

  it('shows a notification on playback error', async () => {
    mocks.play.mockRejectedValueOnce(new Error('backend failed'));

    await playTrack(remoteTrack);

    expect(mocks.push).toHaveBeenCalledWith({
      type: 'error',
      title: 'Playback Error',
      message: 'backend failed',
      dismissible: true,
    });
  });
});
