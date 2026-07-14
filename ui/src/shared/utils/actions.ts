/**
 * Shared track actions — thin wrappers around Tauri commands.
 * Used by TrackList and other components to avoid duplicate command imports.
 */
import { get } from 'svelte/store';
import * as commands from '@services/commands';
import { notifications } from '@shared/stores/notifications';
import { t } from '@i18n';
import { stopRemote } from '@features/player/stores/remotePlayer';
import { prepareLocalFft, selectFftSource } from '@features/player/stores/player';
import { extractErrorMessage } from '@shared/utils/errors';
import type { Track } from '@shared/types/models';

/** Play a track, dispatching to the correct backend command by source. */
export async function playTrack(track: Track): Promise<void> {
  try {
    if (track.localPath) {
      commands.invalidateStreamRequests();
      // Stop any active remote (browser) playback before starting local.
      // The Rust stop() inside play_local_track emits 'Stopped' which
      // triggers stopRemote() in the frontend, but the state changes to
      // 'Buffering' so fast that the 'Stopped' event can be missed,
      // leaving the browser audio element playing alongside cpal.
      stopRemote();
      await prepareLocalFft();
      await commands.playLocal(track.localPath);
    } else {
      // Remote track (YouTube, SoundCloud) — use playStream for HTTP streaming.
      // playStream calls stop() on the Rust side which drops any active
      // cpal stream, and stopRemote() handles the browser audio element.
      selectFftSource('remote');
      await commands.playStream(track);
    }
  } catch (e) {
    const translate = get(t);
    const msg = extractErrorMessage(e, translate);
    const drmMessage = msg.includes('DRM')
      ? translate('playback.drm_protected', { default: 'Cannot play: DRM-protected track' })
      : msg;
    notifications.push({ type: 'error', title: translate('playback.error_title', { default: 'Playback Error' }), message: drmMessage, dismissible: true });
  }
}

/** Add a track to the playback queue using the full Track object — instant, no resolve needed. */
export async function addToQueueAction(track: Track): Promise<void> {
  try {
    await commands.addToQueueWithTrack(track);
    notifications.push({ type: 'success', title: 'Queue', message: 'Track added to queue', dismissible: true });
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Insert a track immediately after the current track in the queue — instant, no resolve needed. */
export async function playNextAction(track: Track): Promise<void> {
  try {
    await commands.playNextWithTrack(track);
    notifications.push({ type: 'info', title: 'Queue', message: 'Track set to play next', dismissible: true });
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Remove a track from the playback queue by its ID. */
export async function removeFromQueueAction(trackId: string): Promise<void> {
  try {
    await commands.removeFromQueue(trackId);
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}

/** Clear the entire playback queue. */
export async function clearQueueAction(): Promise<void> {
  try {
    await commands.clearQueue();
    notifications.push({ type: 'info', title: 'Queue', message: 'Queue cleared', dismissible: true });
  } catch (e) {
    const msg = extractErrorMessage(e, get(t));
    notifications.push({ type: 'error', title: 'Queue Error', message: msg, dismissible: true });
  }
}
