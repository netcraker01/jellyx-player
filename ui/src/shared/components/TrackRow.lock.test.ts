/**
 * Reproduction test for the Search -> add-to-list UI lock bug.
 *
 * Verifies that after adding a track to an existing playlist (or creating
 * a new one and adding there), the ListPicker backdrop unmounts and the
 * close event propagates to the parent so navigation is not blocked.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';

const addTrackMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const createPlaylistMock = vi.hoisted(() =>
  vi.fn().mockResolvedValue({ id: 'pl-new', title: 'New List', created_at: 0, updated_at: 0 }),
);
const getPlaylistTracksMock = vi.hoisted(() => vi.fn().mockResolvedValue([]));
const searchUserPlaylistsMock = vi.hoisted(() => vi.fn().mockResolvedValue([]));
const getAllPlaylistsMock = vi.hoisted(() => vi.fn().mockResolvedValue([
  { id: 'pl-1', title: 'My List', created_at: 0, updated_at: 0 },
]));

vi.mock('@services/commands', () => ({
  addTrackToPlaylist: addTrackMock,
  createPlaylist: createPlaylistMock,
  getPlaylistTracks: getPlaylistTracksMock,
  searchUserPlaylists: searchUserPlaylistsMock,
  getAllPlaylists: getAllPlaylistsMock,
}));

vi.mock('@shared/stores/notifications', () => ({
  notifications: { push: vi.fn() },
}));

vi.mock('@shared/utils/actions', () => ({
  playTrack: vi.fn().mockResolvedValue(undefined),
  addToQueueAction: vi.fn().mockResolvedValue(undefined),
  playNextAction: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@i18n', () => ({
  t: vi.fn(() => () => 'translated'),
}));

import TrackRow from '@shared/components/TrackRow.svelte';
import type { Track } from '@shared/types/models';
import { Source } from '@shared/types/models';
import { playlists } from '@features/playlists/stores/playlists';

const sampleTrack: Track = {
  id: 'track:1',
  source: Source.YouTube,
  sourceId: 'yt-1',
  title: 'One More Time',
  artist: 'Daft Punk',
  duration: 320,
  metadata: {},
};

function renderTrackRow(props: Record<string, unknown> = {}) {
  return render(TrackRow, { props: { track: sampleTrack, ...props } });
}

describe('TrackRow add-to-list UI lock regression', () => {
  beforeEach(() => {
    addTrackMock.mockClear();
    addTrackMock.mockResolvedValue(undefined);
    createPlaylistMock.mockClear();
    createPlaylistMock.mockResolvedValue({ id: 'pl-new', title: 'New List', created_at: 0, updated_at: 0 });
    getPlaylistTracksMock.mockClear();
    getPlaylistTracksMock.mockResolvedValue([]);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('unmounts the picker backdrop after selecting an existing playlist', async () => {
    // Pre-populate the shared playlists store the way a prior Playlists-page
    // visit would, so the picker's recentPlaylists derived store has data
    // immediately when it mounts (mirrors the real app state).
    await playlists.load();

    const { container } = renderTrackRow();

    // Open the list picker via the "Add to list" button (ListMusic icon).
    const listBtn = container.querySelector('.list-btn') as HTMLElement;
    expect(listBtn).toBeTruthy();
    await fireEvent.click(listBtn);

    // Backdrop should be present now.
    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeTruthy();
    });

    // Click the first list option (wait for the store to load and render).
    let option: HTMLElement | null = null;
    await waitFor(() => {
      option = container.querySelector('.list-option');
      expect(option).toBeTruthy();
    });
    await fireEvent.click(option as unknown as HTMLElement);

    // addTrackToPlaylist should have been called.
    await waitFor(() => {
      expect(addTrackMock).toHaveBeenCalled();
    });

    // CRITICAL: the backdrop must unmount after the add resolves,
    // otherwise navigation is blocked by the fixed full-screen overlay.
    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeNull();
    });
  });

  it('unmounts the picker backdrop after creating a new playlist and adding the track', async () => {
    const { container } = renderTrackRow();

    const listBtn = container.querySelector('.list-btn') as HTMLElement;
    await fireEvent.click(listBtn);

    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeTruthy();
    });

    // Click "Create new list".
    const createNewBtn = container.querySelector('.create-new-btn') as HTMLElement;
    await fireEvent.click(createNewBtn);

    // Type a name and add.
    await waitFor(() => {
      expect(container.querySelector('.create-row input')).toBeTruthy();
    });
    const input = container.querySelector('.create-row input') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'My New List' } });

    const addBtn = container.querySelector('.create-btn') as HTMLElement;
    await fireEvent.click(addBtn);

    await waitFor(() => {
      expect(createPlaylistMock).toHaveBeenCalled();
      expect(addTrackMock).toHaveBeenCalled();
    });

    // CRITICAL: backdrop must unmount.
    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeNull();
    });
  });

  it('unmounts the picker backdrop when create fails (error path)', async () => {
    // Simulate create_playlist failing — the store returns undefined.
    createPlaylistMock.mockResolvedValue(undefined);

    const { container } = renderTrackRow();
    const listBtn = container.querySelector('.list-btn') as HTMLElement;
    await fireEvent.click(listBtn);

    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeTruthy();
    });

    const createNewBtn = container.querySelector('.create-new-btn') as HTMLElement;
    await fireEvent.click(createNewBtn);

    await waitFor(() => {
      expect(container.querySelector('.create-row input')).toBeTruthy();
    });
    const input = container.querySelector('.create-row input') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'Failing List' } });

    const addBtn = container.querySelector('.create-btn') as HTMLElement;
    await fireEvent.click(addBtn);

    await waitFor(() => {
      expect(createPlaylistMock).toHaveBeenCalled();
    });

    // CRITICAL: even when create fails, the backdrop MUST unmount.
    // The old code skipped dispatch('close') on failure, locking the UI.
    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeNull();
    });
  });

  it('unmounts the picker backdrop when addTrackToPlaylist fails (error path)', async () => {
    addTrackMock.mockRejectedValue(new Error('IPC failed'));

    const { container } = renderTrackRow();
    const listBtn = container.querySelector('.list-btn') as HTMLElement;
    await fireEvent.click(listBtn);

    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeTruthy();
    });

    const option = container.querySelector('.list-option') as HTMLElement;
    await fireEvent.click(option);

    await waitFor(() => {
      expect(addTrackMock).toHaveBeenCalled();
    });

    // CRITICAL: even when add fails, the backdrop MUST unmount.
    await waitFor(() => {
      expect(container.querySelector('.picker-backdrop')).toBeNull();
    });
  });
});