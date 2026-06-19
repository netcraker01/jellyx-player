/**
 * TrackRow component tests.
 *
 * Verifies a single track row renders track info and action buttons,
 * and calls the shared action helpers on interaction.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  playTrack: vi.fn(),
  addToQueueAction: vi.fn(),
  playNextAction: vi.fn(),
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: mocks.playTrack,
  addToQueueAction: mocks.addToQueueAction,
  playNextAction: mocks.playNextAction,
}));

vi.mock('@features/favorites/stores/favorites', () => ({
  favorites: {
    add: vi.fn(),
  },
}));

import TrackRow from '@shared/components/TrackRow.svelte';
import { Source } from '@shared/types/models';

const sampleTrack = {
  id: 'track:1',
  source: Source.YouTube,
  sourceId: 'yt-1',
  title: 'One More Time',
  artist: 'Daft Punk',
  album: 'Discovery',
  duration: 320,
  thumbnail: 'https://img.test/thumb.jpg',
  streamUrl: 'https://stream.test/track.mp3',
  metadata: {},
};

describe('TrackRow', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders track title and artist', () => {
    const { container } = render(TrackRow, { props: { track: sampleTrack } });

    expect(container.textContent).toContain('One More Time');
    expect(container.textContent).toContain('Daft Punk');
  });

  it('calls playTrack with the full track when play button is clicked', async () => {
    mocks.playTrack.mockResolvedValueOnce(undefined);
    const { container } = render(TrackRow, { props: { track: sampleTrack } });

    const playBtn = container.querySelector('.play-btn');
    expect(playBtn).toBeTruthy();
    await fireEvent.click(playBtn!);

    expect(mocks.playTrack).toHaveBeenCalledWith(sampleTrack);
  });

  it('calls addToQueueAction when queue button is clicked', async () => {
    mocks.addToQueueAction.mockResolvedValueOnce(undefined);
    const { container } = render(TrackRow, { props: { track: sampleTrack } });

    const queueBtn = container.querySelector('[title="Add to Queue"]');
    expect(queueBtn).toBeTruthy();
    await fireEvent.click(queueBtn!);

    expect(mocks.addToQueueAction).toHaveBeenCalledWith('track:1');
  });

  it('calls playNextAction when play-next button is clicked', async () => {
    mocks.playNextAction.mockResolvedValueOnce(undefined);
    const { container } = render(TrackRow, { props: { track: sampleTrack } });

    const nextBtn = container.querySelector('[title="Play Next"]');
    expect(nextBtn).toBeTruthy();
    await fireEvent.click(nextBtn!);

    expect(mocks.playNextAction).toHaveBeenCalledWith('track:1');
  });
});
