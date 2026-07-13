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
  proxyLocalUrl,
  loadRemoteStream,
  getAudioElement,
} from './remotePlayer';
import { reportRemoteAudioPlaybackFailure, reportRemoteAudioPlaybackRuntimeFailure, reportRemoteAudioPlaybackSuccess } from '@services/commands';
import { Source, type Track } from '@shared/types/models';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: {
    push: vi.fn(),
  },
}));

vi.mock('@services/commands', () => ({
    cacheRemoteStream: vi.fn(),
    reportRemoteAudioPlaybackFailure: vi.fn().mockResolvedValue(undefined),
    reportRemoteAudioPlaybackRuntimeFailure: vi.fn().mockResolvedValue(undefined),
    reportRemoteAudioPlaybackSuccess: vi.fn().mockResolvedValue(undefined),
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
  vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => {});
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

  it('routes cached files through the same capability-gated proxy as remote streams', () => {
    const url = proxyLocalUrl(8765, 'per-process-capability', '/tmp/cached track.m4a');

    expect(url).toContain('/proxy?cap=per-process-capability&url=');
    expect(url).toContain(encodeURIComponent('file:///tmp/cached track.m4a'));
  });

  it('does not attempt an unguarded proxy swap when the capability is unavailable', () => {
    expect(proxyLocalUrl(8765, undefined, '/tmp/cached.m4a')).not.toContain('/proxy?');
  });

  it('reports a failed HTMLAudio playback once without changing the fallback behavior', async () => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockRejectedValue(new Error('private browser error'));
    const track = {
      id: 'remote-audio-test', title: 'Private title', artist: 'Private artist', album: '', duration: 0,
      source: Source.SoundCloud, sourceId: 'private-source', streamUrl: 'http://127.0.0.1:8765/proxy?private',
    } as Track;

    await loadRemoteStream(track, track.streamUrl!);

    expect(reportRemoteAudioPlaybackFailure).toHaveBeenCalledTimes(1);
    expect(reportRemoteAudioPlaybackFailure).toHaveBeenCalledWith(expect.any(Number));
    expect(get(remoteActive)).toBe(false);
    play.mockRestore();
  });

  it('records one successful remote playback outcome after HTMLAudio starts', async () => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
    const track = {
      id: 'remote-audio-success', title: 'Private title', artist: 'Private artist', album: '', duration: 0,
      source: Source.SoundCloud, sourceId: 'private-source', streamUrl: 'http://127.0.0.1:8765/proxy?private',
    } as Track;

    await loadRemoteStream(track, track.streamUrl!);

    expect(reportRemoteAudioPlaybackSuccess).toHaveBeenCalledTimes(1);
    expect(reportRemoteAudioPlaybackSuccess).toHaveBeenCalledWith(expect.any(Number));
    expect(reportRemoteAudioPlaybackFailure).not.toHaveBeenCalled();
    play.mockRestore();
  });

  it('records one failed outcome when play rejection and media error race', async () => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockRejectedValue(new Error('private browser error'));
    const track = {
      id: 'remote-audio-race', title: 'Private title', artist: 'Private artist', album: '', duration: 0,
      source: Source.SoundCloud, sourceId: 'private-source', streamUrl: 'http://127.0.0.1:8765/proxy?private',
    } as Track;

    const loading = loadRemoteStream(track, track.streamUrl!);
    getAudioElement()?.dispatchEvent(new Event('error'));
    await loading;

    expect(reportRemoteAudioPlaybackFailure).toHaveBeenCalledTimes(1);
    expect(reportRemoteAudioPlaybackSuccess).not.toHaveBeenCalled();
    play.mockRestore();
  });

  it('records a post-start HTMLAudio error as one separate runtime failure', async () => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
    const track = { id: 'runtime-error', title: '', artist: '', album: '', duration: 0, source: Source.SoundCloud, sourceId: 'id', streamUrl: 'http://127.0.0.1:8765/proxy?runtime' } as Track;

    await loadRemoteStream(track, track.streamUrl!);
    getAudioElement()?.dispatchEvent(new Event('error'));
    getAudioElement()?.dispatchEvent(new Event('error'));

    expect(reportRemoteAudioPlaybackSuccess).toHaveBeenCalledTimes(1);
    expect(reportRemoteAudioPlaybackRuntimeFailure).toHaveBeenCalledTimes(1);
    play.mockRestore();
  });

  it('ignores stale rejected callbacks from an overlapping playback attempt', async () => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
    let rejectFirst!: (error: Error) => void;
    const firstPlay = new Promise<void>((_, reject) => { rejectFirst = reject; });
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play')
      .mockReturnValueOnce(firstPlay)
      .mockResolvedValueOnce(undefined);
    const first = { id: 'first', title: '', artist: '', album: '', duration: 0, source: Source.SoundCloud, sourceId: 'first', streamUrl: 'http://127.0.0.1:8765/proxy?first' } as Track;
    const second = { ...first, id: 'second', sourceId: 'second', streamUrl: 'http://127.0.0.1:8765/proxy?second' };

    const staleLoad = loadRemoteStream(first, first.streamUrl!);
    await loadRemoteStream(second, second.streamUrl!);
    rejectFirst(new Error('stale private error'));
    await staleLoad;

    expect(reportRemoteAudioPlaybackSuccess).toHaveBeenCalledTimes(1);
    expect(reportRemoteAudioPlaybackFailure).not.toHaveBeenCalled();
    expect(reportRemoteAudioPlaybackRuntimeFailure).not.toHaveBeenCalled();
    play.mockRestore();
  });

  it('does not restore remote state when a stale play fulfillment follows a newer failure', async () => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
    let resolveFirst!: () => void;
    const firstPlay = new Promise<void>((resolve) => { resolveFirst = resolve; });
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play')
      .mockReturnValueOnce(firstPlay)
      .mockRejectedValueOnce(new Error('newer failure'));
    const first = { id: 'same-track', title: '', artist: '', album: '', duration: 0, source: Source.SoundCloud, sourceId: 'same-track', streamUrl: 'http://127.0.0.1:8765/proxy?first' } as Track;
    const second = { ...first, streamUrl: 'http://127.0.0.1:8765/proxy?second' };

    const staleLoad = loadRemoteStream(first, first.streamUrl!);
    await loadRemoteStream(second, second.streamUrl!);
    resolveFirst();
    await staleLoad;

    expect(get(remoteActive)).toBe(false);
    expect(reportRemoteAudioPlaybackSuccess).not.toHaveBeenCalled();
    play.mockRestore();
  });
});
