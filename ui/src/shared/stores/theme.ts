/**
 * Dark-mode theme store.
 *
 * Sets CSS custom properties on :root for the Jellyx dark theme.
 * Future: add theme switching (light mode toggle).
 */

import { writable } from 'svelte/store';

type Theme = 'dark';

// Jellyx is dark-mode only per UI_DESIGN.md
export const currentTheme = writable<Theme>('dark');

/**
 * Apply theme tokens to the document root.
 * Currently hardcoded to dark theme; expand when light mode is added.
 */
export function applyTheme(theme: Theme = 'dark'): void {
  const root = document.documentElement;
  // Tokens are defined in tokens.css; this function
  // is a hook for future theme switching.
  root.setAttribute('data-theme', theme);
}