import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import MiniPlayer from './MiniPlayer.svelte';
import { exitMiniPlayer } from './mode';
import { activateMiniPlayerSkin } from './skins';

vi.mock('./mode', () => ({
  exitMiniPlayer: vi.fn().mockResolvedValue(undefined),
}));

describe('MiniPlayer', () => {
  afterEach(() => {
    activateMiniPlayerSkin('ipod-classic');
  });

  it('offers a clear action to return to the full app', async () => {
    render(MiniPlayer);

    const button = screen.getByRole('button', { name: 'Return to full app' });
    expect(button).toBeTruthy();

    await fireEvent.click(button);

    expect(exitMiniPlayer).toHaveBeenCalledTimes(1);
  });

  it('renders the selected skin contract, sizing, and theme', () => {
    activateMiniPlayerSkin('graphite-pocket');

    const { container } = render(MiniPlayer);
    const shell = screen.getByLabelText('Mini player');
    const device = container.querySelector<HTMLElement>('.device');

    expect(device?.dataset.skin).toBe('graphite-pocket');
    expect(screen.getByLabelText('Graphite Pocket')).toBeTruthy();
    expect(shell.getAttribute('style')).toContain('--skin-card-width: 300px');
    expect(shell.getAttribute('style')).toContain('--skin-card-height: 480px');
    expect(shell.getAttribute('style')).toContain('--skin-shell: #2f343d');
    expect(shell.getAttribute('style')).toContain('--skin-accent: #93c5fd');
  });
});
