/**
 * Shared action tests.
 *
 * Verifies that playTrack dispatches to the correct Tauri command
 * depending on whether the track is local or remote.
 * Also verifies that addToQueueAction and playNextAction pass the
 * full Track object to the fast _with_track commands.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { playTrack, addToQueueAction, playNextAction } from './actions';
import { Source } from '@shared/types/models';
import type { Track } from '@shared/types/models';

const mocks = vi.hoisted(() => ({
  playStream: vi.fn(),
  playLocal: vi.fn(),
  invalidateStreamRequests: vi.fn(),
  addToQueueWithTrack: vi.fn(),
  playNextWithTrack: vi.fn(),
  push: vi.fn(),
}));

vi.mock('@services/commands', () => ({
  playStream: mocks.playStream,
  playLocal: mocks.playLocal,
  invalidateStreamRequests: mocks.invalidateStreamRequests,
  addToQueueWithTrack: mocks.addToQueueWithTrack,
  playNextWithTrack: mocks.playNextWithTrack,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: mocks.push,
  },
}));

const { readable } = await vi.hoisted(() => import('svelte/store'));

vi.mock('@i18n', () => {
  const translateFn = (key: string, params?: Record<string, string | number>) => {
    // Return the default value from params if provided, otherwise the key
    if (params?.default) return params.default as string;
    return key;
  };
  const store = readable(translateFn, () => {});
  return { t: store };
});

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

  it('invokes playStream for remote tracks', async () => {
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
    expect(mocks.invalidateStreamRequests).toHaveBeenCalled();
  });

  it('prefers playLocal when both streamUrl and localPath are present', async () => {
    mocks.playLocal.mockResolvedValueOnce(undefined);

    const mixed: Track = { ...remoteTrack, localPath: '/music/track.mp3' };
    await playTrack(mixed);

    expect(mocks.playLocal).toHaveBeenCalledWith('/music/track.mp3');
    expect(mocks.playStream).not.toHaveBeenCalled();
  });

  it('invokes playStream for remote tracks without streamUrl', async () => {
    mocks.playStream.mockResolvedValueOnce(undefined);

    const noUrl: Track = {
      id: 'track:yt:2',
      source: Source.YouTube,
      sourceId: 'yt-2',
      title: 'No URL Track',
      artist: 'Artist',
      metadata: {},
    };
    await playTrack(noUrl);

    expect(mocks.playStream).toHaveBeenCalledWith(noUrl);
    expect(mocks.playLocal).not.toHaveBeenCalled();
  });

  it('shows a notification on playback error', async () => {
    mocks.playStream.mockRejectedValueOnce(new Error('backend failed'));

    await playTrack(remoteTrack);

    expect(mocks.push).toHaveBeenCalledWith({
      type: 'error',
      title: 'Playback Error',
      message: 'backend failed',
      dismissible: true,
    });
  });
});

describe('addToQueueAction', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('passes the full Track object to addToQueueWithTrack', async () => {
    mocks.addToQueueWithTrack.mockResolvedValueOnce(undefined);

    await addToQueueAction(localTrack);

    expect(mocks.addToQueueWithTrack).toHaveBeenCalledWith(localTrack);
  });

  it('passes the full remote Track object to addToQueueWithTrack', async () => {
    mocks.addToQueueWithTrack.mockResolvedValueOnce(undefined);

    await addToQueueAction(remoteTrack);

    expect(mocks.addToQueueWithTrack).toHaveBeenCalledWith(remoteTrack);
  });

  it('shows a notification on error', async () => {
    mocks.addToQueueWithTrack.mockRejectedValueOnce(new Error('queue failed'));

    await addToQueueAction(remoteTrack);

    expect(mocks.push).toHaveBeenCalledWith({
      type: 'error',
      title: 'Queue Error',
      message: 'queue failed',
      dismissible: true,
    });
  });
});

describe('playNextAction', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('passes the full Track object to playNextWithTrack', async () => {
    mocks.playNextWithTrack.mockResolvedValueOnce(undefined);

    await playNextAction(localTrack);

    expect(mocks.playNextWithTrack).toHaveBeenCalledWith(localTrack);
  });

  it('passes the full remote Track object to playNextWithTrack', async () => {
    mocks.playNextWithTrack.mockResolvedValueOnce(undefined);

    await playNextAction(remoteTrack);

    expect(mocks.playNextWithTrack).toHaveBeenCalledWith(remoteTrack);
  });

  it('shows a notification on error', async () => {
    mocks.playNextWithTrack.mockRejectedValueOnce(new Error('next failed'));

    await playNextAction(remoteTrack);

    expect(mocks.push).toHaveBeenCalledWith({
      type: 'error',
      title: 'Queue Error',
      message: 'next failed',
      dismissible: true,
    });
  });
});
