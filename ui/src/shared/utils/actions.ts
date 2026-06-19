/**
 * Shared track actions — thin wrappers around Tauri commands.
 * Used by TrackList and other components to avoid duplicate command imports.
 */
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import type { Track } from '@shared/types/models';

/** Play a track, dispatching to the correct backend command by source. */
export async function playTrack(track: Track): Promise<void> {
  try {
    if (track.localPath) {
      await commands.playLocal(track.localPath);
    } else if (track.streamUrl) {
      await commands.play(track.streamUrl);
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Playback Error', message: msg, dismissible: true });
  }
}

/** Add a track to the playback queue by its ID. */
export async function addToQueueAction(trackId: string): Promise<void> {
  try {
    await commands.addToQueue(trackId);
    notifications.push({ type: 'success', title: 'Queue', message: 'Track added to queue', dismissible: true });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Insert a track immediately after the current track in the queue. */
export async function playNextAction(trackId: string): Promise<void> {
  try {
    await commands.playNext(trackId);
    notifications.push({ type: 'info', title: 'Queue', message: 'Track set to play next', dismissible: true });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Remove a track from the playback queue by its ID. */
export async function removeFromQueueAction(trackId: string): Promise<void> {
  try {
    await commands.removeFromQueue(trackId);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Clear the entire playback queue. */
export async function clearQueueAction(): Promise<void> {
  try {
    await commands.clearQueue();
    notifications.push({ type: 'info', title: 'Queue', message: 'Queue cleared', dismissible: true });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}