import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const desktopDirectory = resolve(__dirname, '../../../../jellyx-desktop');
const mainCapability = JSON.parse(readFileSync(resolve(desktopDirectory, 'capabilities/main.json'), 'utf-8')) as {
  permissions: string[];
};
const tauriConfig = JSON.parse(readFileSync(resolve(desktopDirectory, 'tauri.conf.json'), 'utf-8')) as {
  app: { windows: Array<{ transparent?: boolean }> };
};

describe('mini-player desktop configuration', () => {
  it('permits the native resize lock used by mini-player mode', () => {
    expect(mainCapability.permissions).toContain('core:window:allow-set-resizable');
  });

  it('keeps the main window transparent for the mini-player surround', () => {
    expect(tauriConfig.app.windows[0]?.transparent).toBe(true);
  });
});
