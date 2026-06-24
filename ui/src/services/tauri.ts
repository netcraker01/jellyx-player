/**
 * Tauri IPC abstraction layer.
 *
 * Provides `invokeCommand` and `subscribeEvent` with graceful
 * fallback when running in browser (dev mode without Tauri).
 */

type UnlistenFn = () => void;

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Invoke a Tauri command. Returns a sensible default when Tauri is unavailable.
 *
 * IMPORTANT: Never returns undefined for array-returning commands.
 * A TypeError on `undefined.length` or `undefined.map()` will corrupt
 * Svelte's reactivity scheduler and lock up the entire app.
 */
export async function invokeCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  // Browser fallback: return type-appropriate defaults
  // to prevent TypeErrors that crash Svelte's reactivity.
  const arrayCommands = new Set([
    'get_all_playlists',
    'get_all_artist_favorites',
    'get_playlist_tracks',
    'get_local_tracks',
    'get_watched_folders',
    'get_history',
    'get_home_recommendations',
    'search',
    'search_playlists',
    'search_grouped',
    'get_artist_detail',
    'get_album_detail',
    'get_queue',
  ]);
  if (arrayCommands.has(cmd)) return [] as unknown as T;
  return undefined as T;
}

/**
 * Subscribe to a Tauri event. Returns an unlisten function.
 * When Tauri is unavailable, returns a no-op unlisten.
 */
export async function subscribeEvent<T>(
  event: string,
  cb: (payload: T) => void,
): Promise<UnlistenFn> {
  if (isTauri) {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<T>(event, (e) => cb(e.payload));
    return unlisten;
  }
  // Browser fallback: no-op unlisten
  return () => {};
}