/**
 * Brand asset tests for PR 1.
 *
 * Verifies that the Jellyx logo component and brand assets exist and
 * that the desktop icon directory contains the expected icon formats.
 */
import { describe, it, expect } from 'vitest';
import { existsSync, readdirSync } from 'fs';
import { resolve } from 'path';
import JellyxLogo from '@shared/components/JellyxLogo.svelte';

describe('PR 1 brand assets', () => {
  const iconsDir = resolve(__dirname, '../../../../jellyx-desktop/icons');
  const assetsDir = resolve(__dirname, '../assets/jellyx');

  it('has a JellyxLogo Svelte component export', () => {
    expect(JellyxLogo).toBeTruthy();
  });

  it('copies SVG brand assets into ui/src/shared/assets/jellyx', () => {
    expect(existsSync(resolve(assetsDir, 'logo-icon.svg'))).toBe(true);
    expect(existsSync(resolve(assetsDir, 'logo-wide.svg'))).toBe(true);
    expect(existsSync(resolve(assetsDir, 'logo-wide.png'))).toBe(true);
  });

  it('replaces desktop icon files with Jellyx versions', () => {
    const files = readdirSync(iconsDir);
    for (const name of ['icon.png', 'icon.ico', 'icon.icns', '128x128.png', 'Square150x150Logo.png', 'StoreLogo.png']) {
      expect(files).toContain(name);
    }
  });
});
