/**
 * Asset URL utilities for serving local files via Tauri's asset protocol.
 *
 * Wraps `convertFileSrc()` from `@tauri-apps/api/core` with graceful
 * browser fallback when Tauri is unavailable.
 */

import { convertFileSrc } from '@tauri-apps/api/core';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Convert a local file path to a loadable URL via Tauri's asset protocol.
 *
 * Used to serve cached album art images from `~/.local/share/helix/art/`
 * into `<img src>` attributes.
 *
 * `convertFileSrc` is synchronous in Tauri v2 — it just transforms the path
 * into a `http://asset.localhost/` URL. No IPC involved.
 *
 * @param thumbnail - Absolute filesystem path to the cached art file, or undefined
 * @returns Asset protocol URL string, or undefined if no thumbnail
 */
export function albumArtUrl(thumbnail: string | undefined): string | undefined {
  if (!thumbnail) return undefined;

  if (isTauri) {
    return convertFileSrc(thumbnail);
  }

  // Browser fallback: no asset protocol available
  return undefined;
}