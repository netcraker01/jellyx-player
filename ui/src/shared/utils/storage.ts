/**
 * localStorage migration helpers for PR 3.
 *
 * Implements the localStorage key migration contract:
 *  - All new writes go to `jellyx-*` keys.
 *  - Reads fall back to legacy `helix-*` keys when the new key is absent.
 *  - Legacy keys are never deleted by these helpers.
 */

const STORAGE_PREFIX = 'jellyx-';
const LEGACY_PREFIX = 'helix-';

/**
 * Build the canonical (new) key name.
 */
export function canonicalKey(suffix: string): string {
  return `${STORAGE_PREFIX}${suffix}`;
}

/**
 * Build the legacy key name.
 */
export function legacyKey(suffix: string): string {
  return `${LEGACY_PREFIX}${suffix}`;
}

/**
 * Read a string value from localStorage.
 * Prefers the canonical `jellyx-*` key; falls back to `helix-*` if missing.
 * Returns null if neither exists or localStorage is unavailable.
 */
export function getMigratedItem(suffix: string): string | null {
  try {
    const canonical = localStorage.getItem(canonicalKey(suffix));
    if (canonical !== null) return canonical;
    return localStorage.getItem(legacyKey(suffix));
  } catch {
    return null;
  }
}

/**
 * Write a string value to the canonical `jellyx-*` key.
 * Silently no-ops if localStorage is unavailable.
 */
export function setMigratedItem(suffix: string, value: string): void {
  try {
    localStorage.setItem(canonicalKey(suffix), value);
  } catch {
    // localStorage may be unavailable (SSR / private mode)
  }
}

/**
 * Remove the canonical `jellyx-*` key.
 * Does NOT touch the legacy `helix-*` key.
 */
export function removeMigratedItem(suffix: string): void {
  try {
    localStorage.removeItem(canonicalKey(suffix));
  } catch {
    // localStorage may be unavailable
  }
}
