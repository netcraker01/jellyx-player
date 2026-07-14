import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import MiniPlayer from './MiniPlayer.svelte';
import { exitMiniPlayer, minimizeMiniPlayer, quitFromMiniPlayer } from './mode';
import { activateMiniPlayerSkin, setMiniPlayerScale } from './skins';

const nativeWindow = vi.hoisted(() => ({
  updateMiniWindowSize: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./mode', () => ({
  exitMiniPlayer: vi.fn().mockResolvedValue(undefined),
  minimizeMiniPlayer: vi.fn().mockResolvedValue(undefined),
  quitFromMiniPlayer: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./nativeWindow', () => nativeWindow);

describe('MiniPlayer', () => {
  afterEach(() => {
    activateMiniPlayerSkin('ipod-classic');
    setMiniPlayerScale(1);
    vi.clearAllMocks();
  });

  it('offers a clear action to return to the full app', async () => {
    render(MiniPlayer);

    const button = screen.getByRole('button', { name: 'Return to full app' });
    expect(button).toBeTruthy();

    await fireEvent.click(button);

    expect(exitMiniPlayer).toHaveBeenCalledTimes(1);
  });

  it('renders integrated mini window controls', async () => {
    render(MiniPlayer);

    await fireEvent.click(screen.getByRole('button', { name: 'Minimize mini player' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Quit Jellyx Player' }));

    expect(screen.getByLabelText('Mini player window controls')).toBeTruthy();
    expect(minimizeMiniPlayer).toHaveBeenCalledTimes(1);
    expect(quitFromMiniPlayer).toHaveBeenCalledTimes(1);
  });

  it('marks non-interactive surfaces as native drag regions without disabling controls', () => {
    const { container } = render(MiniPlayer);

    expect(screen.getByLabelText('Mini player').hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(container.querySelector('.device')?.hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(container.querySelector('.screen')?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(container.querySelector('.track-card')?.hasAttribute('data-tauri-drag-region')).toBe(true);

    expect(screen.getByRole('button', { name: 'Return to full app' }).hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(screen.getByRole('button', { name: 'Minimize mini player' }).hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(screen.getByRole('button', { name: 'Quit Jellyx Player' }).hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(screen.getByRole('button', { name: 'Previous' }).hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(screen.getByRole('button', { name: 'Next' }).hasAttribute('data-tauri-drag-region')).toBe(false);
    expect(screen.getByRole('button', { name: 'Play' }).hasAttribute('data-tauri-drag-region')).toBe(false);
  });

  it('renders the selected skin contract, sizing, and theme', () => {
    activateMiniPlayerSkin('graphite-pocket');
    setMiniPlayerScale(0.3);

    const { container } = render(MiniPlayer);
    const shell = screen.getByLabelText('Mini player');
    const device = container.querySelector<HTMLElement>('.device');

    expect(device?.dataset.skin).toBe('graphite-pocket');
    expect(device?.dataset.kind).toBe('ipod');
    expect(device?.dataset.shape).toBe('rounded-rectangle');
    expect(screen.getByLabelText('Graphite Pocket')).toBeTruthy();
    expect(shell.getAttribute('style')).toContain('--skin-card-width: 300px');
    expect(shell.getAttribute('style')).toContain('--skin-card-height: 480px');
    expect(shell.getAttribute('style')).toContain('--skin-window-width: 90px');
    expect(shell.getAttribute('style')).toContain('--skin-window-height: 144px');
    expect(shell.getAttribute('style')).toContain('--skin-scale: 0.3');
    expect(shell.getAttribute('style')).toContain('--skin-shell: #2f343d');
    expect(shell.getAttribute('style')).toContain('--skin-screen-text: #111827');
    expect(shell.getAttribute('style')).toContain('--skin-accent: #93c5fd');
  });

  it('uses a proportional compact skin instead of independent width or height squeezing', () => {
    setMiniPlayerScale(0.3);

    const { container } = render(MiniPlayer);
    const shell = screen.getByLabelText('Mini player');
    const device = container.querySelector<HTMLElement>('.device');

    expect(shell.getAttribute('style')).toContain('--skin-window-width: 96px');
    expect(shell.getAttribute('style')).toContain('--skin-window-height: 144px');
    expect(device?.classList.contains('compact')).toBe(true);
  });

  it('uses a transparent background so the OS composes the window surround', () => {
    const { container } = render(MiniPlayer);
    const main = container.querySelector<HTMLElement>('.mini-player');

    expect(main).toBeTruthy();
    // The computed background must not be opaque — body and .mini-player are both transparent.
    const bodyBg = window.getComputedStyle(document.body).backgroundColor;
    const mainBg = main ? window.getComputedStyle(main).backgroundColor : 'rgb(0, 0, 0)';
    // Either transparent or rgba with 0 alpha
    expect(bodyBg).toMatch(/rgba?\(\s*0\s*,\s*0\s*,\s*0\s*,\s*0\s*\)|transparent/);
    expect(mainBg).toMatch(/rgba?\(\s*0\s*,\s*0\s*,\s*0\s*,\s*0\s*\)|transparent/);
  });

  it('serializes overlapping size updates and finishes at the latest requested size', async () => {
    let resolveInitialResize!: () => void;
    let resolveLatestResize!: () => void;
    nativeWindow.updateMiniWindowSize
      .mockImplementationOnce(() => new Promise<void>((resolve) => { resolveInitialResize = resolve; }))
      .mockImplementationOnce(() => new Promise<void>((resolve) => { resolveLatestResize = resolve; }));

    render(MiniPlayer);
    setMiniPlayerScale(0.5);
    setMiniPlayerScale(0.3);
    await tick();

    expect(nativeWindow.updateMiniWindowSize).toHaveBeenCalledTimes(1);
    expect(nativeWindow.updateMiniWindowSize).toHaveBeenLastCalledWith({ width: 320, height: 480 });

    resolveInitialResize();
    await tick();
    await Promise.resolve();

    expect(nativeWindow.updateMiniWindowSize).toHaveBeenCalledTimes(2);
    expect(nativeWindow.updateMiniWindowSize).toHaveBeenLastCalledWith({ width: 96, height: 144 });

    resolveLatestResize();
    await Promise.resolve();
  });
});
