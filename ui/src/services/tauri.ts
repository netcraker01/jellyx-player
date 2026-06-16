/**
 * Tauri IPC abstraction layer.
 *
 * Provides `invokeCommand` and `subscribeEvent` with graceful
 * fallback when running in browser (dev mode without Tauri).
 */

type UnlistenFn = () => void;

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Invoke a Tauri command. Returns fallback value when Tauri is unavailable.
 */
export async function invokeCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  // Browser fallback: return empty/default
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