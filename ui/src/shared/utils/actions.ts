/**
 * Shared track actions — thin wrappers around Tauri commands.
 * Used by TrackList and other components to avoid duplicate command imports.
 */
import * as commands from '@services/commands';

/** Play a track by its stream URL or local path. */
export async function playTrack(url: string): Promise<void> {
  try {
    await commands.play(url);
  } catch (e) {
    console.error('Failed to play track:', e);
  }
}

/** Add a track to the playback queue by its ID. */
export async function addToQueueAction(trackId: string): Promise<void> {
  try {
    await commands.addToQueue(trackId);
  } catch (e) {
    console.error('Failed to add to queue:', e);
  }
}