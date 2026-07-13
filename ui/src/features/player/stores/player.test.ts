/**
 * Player store action tests.
 *
 * Verifies that playTrack delegates to the correct Tauri command
 * based on the track source, and that setVolume scales 0-100 → 0.0-1.0
 * at the IPC boundary (fix for local playback clamping to max).
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  playTrack,
  setVolume,
  volume,
  shouldAcceptStreamResolution,
} from './player';
import { Source } from '@shared/types/models';
import type { FrequencyData, Track } from '@shared/types/models';

const mocks = vi.hoisted(() => ({
  playStream: vi.fn(),
  playLocal: vi.fn(),
  isLatestStreamRequest: vi.fn((_requestId: number) => true),
  invalidateStreamRequests: vi.fn(),
  push: vi.fn(),
  setVolumeCmd: vi.fn(),
  onFftFrame: vi.fn().mockResolvedValue(() => {}),
  onTrackChanged: vi.fn().mockResolvedValue(() => {}),
  onStateChanged: vi.fn().mockResolvedValue(() => {}),
  onQueueUpdated: vi.fn().mockResolvedValue(() => {}),
  onProgressTick: vi.fn().mockResolvedValue(() => {}),
  onBufferingProgress: vi.fn().mockResolvedValue(() => {}),
  onStreamResolved: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@services/commands', () => ({
  playStream: mocks.playStream,
  playLocal: mocks.playLocal,
  isLatestStreamRequest: mocks.isLatestStreamRequest,
  invalidateStreamRequests: mocks.invalidateStreamRequests,
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
  onFftFrame: mocks.onFftFrame,
  onTrackChanged: mocks.onTrackChanged,
  onStateChanged: mocks.onStateChanged,
  onQueueUpdated: mocks.onQueueUpdated,
  onProgressTick: mocks.onProgressTick,
  onBufferingProgress: mocks.onBufferingProgress,
  onStreamResolved: mocks.onStreamResolved,
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
    expect(mocks.invalidateStreamRequests).toHaveBeenCalled();
  });
});

describe('stream resolution correlation', () => {
  it('rejects an older same-track resolution after replay', () => {
    mocks.isLatestStreamRequest.mockImplementation((requestId: number) => requestId === 2);
    const older = { trackId: remoteTrack.id, streamRequestId: 1, streamUrl: 'http://proxy/old' };
    const newer = { ...older, streamRequestId: 2, streamUrl: 'http://proxy/new' };

    expect(shouldAcceptStreamResolution(remoteTrack, older)).toBe(false);
    expect(shouldAcceptStreamResolution(remoteTrack, newer)).toBe(true);
  });
});

describe('player event bootstrap FFT ownership', () => {
  function prepareBootstrapMocks(): void {
    vi.resetModules();
    vi.clearAllMocks();
    mocks.onFftFrame.mockResolvedValue(() => {});
    mocks.onTrackChanged.mockResolvedValue(() => {});
    mocks.onStateChanged.mockResolvedValue(() => {});
    mocks.onQueueUpdated.mockResolvedValue(() => {});
    mocks.onProgressTick.mockResolvedValue(() => {});
    mocks.onBufferingProgress.mockResolvedValue(() => {});
    mocks.onStreamResolved.mockResolvedValue(() => {});
    mocks.isLatestStreamRequest.mockReturnValue(true);
  }

  it('starts one local listener at bootstrap, publishes frames without a visualizer, and gates stale sources', async () => {
    vi.resetModules();
    vi.clearAllMocks();
    let onFrame: ((data: { bins: Float32Array; sampleRate: number; peak: number }) => void) | undefined;
    mocks.onFftFrame.mockImplementation(async (callback) => {
      onFrame = callback;
      return vi.fn();
    });

    const {
      frequencyData: bootstrapFrequencyData,
      initPlayerEvents: bootstrapPlayerEvents,
      publishFftFrame: publishBootstrapFrame,
      selectFftSource: selectBootstrapSource,
    } = await import('./player');

    await bootstrapPlayerEvents();
    await bootstrapPlayerEvents();

    expect(mocks.onFftFrame).toHaveBeenCalledTimes(1);
    const localFrame = { bins: new Float32Array([0.2]), sampleRate: 44_100, peak: 0.2 };
    onFrame?.(localFrame);

    let value: FrequencyData | null = null;
    const unsubscribe = bootstrapFrequencyData.subscribe((frame) => { value = frame; });
    expect(value).toBe(localFrame);

    selectBootstrapSource('remote');
    onFrame?.(localFrame);
    expect(value).toBeNull();

    const remoteFrame = { bins: new Float32Array([0.8]), sampleRate: 44_100, peak: 0.8 };
    publishBootstrapFrame('remote', remoteFrame);
    expect(value).toBe(remoteFrame);

    selectBootstrapSource('local');
    publishBootstrapFrame('remote', remoteFrame);
    expect(value).toBeNull();
    unsubscribe();
  });

  it('retries initialization after the first FFT listener attempt rejects', async () => {
    prepareBootstrapMocks();
    mocks.onFftFrame.mockRejectedValueOnce(new Error('FFT unavailable'));
    const { initPlayerEvents } = await import('./player');

    await expect(initPlayerEvents()).rejects.toThrow('FFT unavailable');
    await initPlayerEvents();

    expect(mocks.onFftFrame).toHaveBeenCalledTimes(2);
    expect(mocks.onTrackChanged).toHaveBeenCalledTimes(1);
  });

  it('shares one in-flight initialization across concurrent callers', async () => {
    prepareBootstrapMocks();
    let resolveListener!: (unlisten: () => void) => void;
    mocks.onFftFrame.mockImplementationOnce(() => new Promise((resolve) => { resolveListener = resolve; }));
    const { initPlayerEvents } = await import('./player');

    const first = initPlayerEvents();
    const second = initPlayerEvents();
    await Promise.resolve();
    expect(mocks.onFftFrame).toHaveBeenCalledTimes(1);
    resolveListener(() => {});
    await Promise.all([first, second]);

    expect(mocks.onFftFrame).toHaveBeenCalledTimes(1);
    expect(mocks.onTrackChanged).toHaveBeenCalledTimes(1);
  });

  it('cleans partial setup before retrying so listeners are not duplicated', async () => {
    prepareBootstrapMocks();
    const stopListener = vi.fn();
    const stopTrackChanged = vi.fn();
    mocks.onFftFrame.mockResolvedValue(stopListener);
    mocks.onTrackChanged.mockResolvedValue(stopTrackChanged);
    mocks.onStateChanged.mockRejectedValueOnce(new Error('state listener unavailable'));
    const { initPlayerEvents } = await import('./player');

    await expect(initPlayerEvents()).rejects.toThrow('state listener unavailable');
    await initPlayerEvents();

    expect(stopListener).toHaveBeenCalledTimes(1);
    expect(stopTrackChanged).toHaveBeenCalledTimes(1);
    expect(mocks.onFftFrame).toHaveBeenCalledTimes(2);
    expect(mocks.onTrackChanged).toHaveBeenCalledTimes(2);
    expect(mocks.onStateChanged).toHaveBeenCalledTimes(2);
  });

  it('clears isPlaying when the backend propagates Stopped after a stream failure', async () => {
    prepareBootstrapMocks();
    let onStateChanged: ((state: string) => void) | undefined;
    mocks.onStateChanged.mockImplementation(async (callback) => {
      onStateChanged = callback;
      return () => {};
    });
    const { initPlayerEvents, isPlaying } = await import('./player');
    await initPlayerEvents();

    onStateChanged?.('Playing');
    let playing = false;
    const unsubscribe = isPlaying.subscribe((value) => { playing = value; });
    expect(playing).toBe(true);

    onStateChanged?.('Stopped');
    expect(playing).toBe(false);
    unsubscribe();
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
