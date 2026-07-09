import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import MiniPlayer from './MiniPlayer.svelte';
import { exitMiniPlayer, minimizeMiniPlayer, quitFromMiniPlayer } from './mode';
import { activateMiniPlayerSkin, setMiniPlayerScale } from './skins';

vi.mock('./mode', () => ({
  exitMiniPlayer: vi.fn().mockResolvedValue(undefined),
  minimizeMiniPlayer: vi.fn().mockResolvedValue(undefined),
  quitFromMiniPlayer: vi.fn().mockResolvedValue(undefined),
}));

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
    await fireEvent.click(screen.getByRole('button', { name: 'Quit Helix' }));

    expect(screen.getByLabelText('Mini player window controls')).toBeTruthy();
    expect(minimizeMiniPlayer).toHaveBeenCalledTimes(1);
    expect(quitFromMiniPlayer).toHaveBeenCalledTimes(1);
  });

  it('renders the selected skin contract, sizing, and theme', () => {
    activateMiniPlayerSkin('winamp-classic');
    setMiniPlayerScale(0.3);

    const { container } = render(MiniPlayer);
    const shell = screen.getByLabelText('Mini player');
    const device = container.querySelector<HTMLElement>('.device');

    expect(device?.dataset.skin).toBe('winamp-classic');
    expect(device?.dataset.kind).toBe('classic');
    expect(device?.dataset.shape).toBe('rounded-rectangle');
    expect(screen.getByLabelText('Classic')).toBeTruthy();
    expect(shell.getAttribute('style')).toContain('--skin-card-width: 400px');
    expect(shell.getAttribute('style')).toContain('--skin-card-height: 100px');
    expect(shell.getAttribute('style')).toContain('--skin-window-width: 120px');
    expect(shell.getAttribute('style')).toContain('--skin-window-height: 30px');
    expect(shell.getAttribute('style')).toContain('--skin-scale: 0.3');
    expect(shell.getAttribute('style')).toContain('--skin-shell: #171b22');
    expect(shell.getAttribute('style')).toContain('--skin-screen-text: #ffd166');
    expect(shell.getAttribute('style')).toContain('--skin-accent: #ff9f1c');
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
});
