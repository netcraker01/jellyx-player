/**
 * ArtistCard component tests.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

vi.mock('@app/router/navigation', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@shared/utils/assetUrl', () => ({
  albumArtUrl: vi.fn((path?: string) => path),
}));

import ArtistCard from '@shared/components/ArtistCard.svelte';

describe('ArtistCard', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders artist name and track count', () => {
    const { container } = render(ArtistCard, {
      props: { id: 'artist:daft-punk', name: 'Daft Punk', trackCount: 12 },
    });

    expect(container.textContent).toContain('Daft Punk');
    expect(container.textContent).toContain('12 tracks');
  });

  it('navigates to artist page on click', async () => {
    const { container } = render(ArtistCard, {
      props: { id: 'artist:daft-punk', name: 'Daft Punk', trackCount: 12 },
    });

    const card = container.querySelector('.artist-card');
    expect(card).toBeTruthy();
    await fireEvent.click(card!);

    expect(mocks.navigate).toHaveBeenCalledWith('/artist/artist%3Adaft-punk');
  });

  it('renders placeholder when thumbnail is missing', () => {
    const { container } = render(ArtistCard, {
      props: { id: 'artist:queen', name: 'Queen', trackCount: 10 },
    });

    expect(container.querySelector('.artist-art-placeholder')).toBeTruthy();
  });
});
