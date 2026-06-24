/**
 * AlbumCard component tests.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  albumArtUrl: vi.fn((path?: string) => path),
}));

vi.mock('@app/router/navigation', () => ({
  navigate: mocks.navigate,
}));

vi.mock('@shared/utils/assetUrl', () => ({
  albumArtUrl: mocks.albumArtUrl,
}));

import AlbumCard from '@shared/components/AlbumCard.svelte';

describe('AlbumCard', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders title, artist, year, and track count', () => {
    const { container } = render(AlbumCard, {
      props: {
        id: 'album:discovery:daft-punk',
        title: 'Discovery',
        artist: 'Daft Punk',
        year: 2001,
        trackCount: 14,
      },
    });

    expect(container.textContent).toContain('Discovery');
    expect(container.textContent).toContain('Daft Punk');
    expect(container.textContent).toContain('2001');
    expect(container.textContent).toContain('14 tracks');
  });

  it('navigates to album page on click', async () => {
    const { container } = render(AlbumCard, {
      props: {
        id: 'album:discovery:daft-punk',
        title: 'Discovery',
        artist: 'Daft Punk',
        trackCount: 14,
      },
    });

    const card = container.querySelector('.album-card');
    expect(card).toBeTruthy();
    await fireEvent.click(card!);

    expect(mocks.navigate).toHaveBeenCalledWith('/album/album%3Adiscovery%3Adaft-punk');
  });

  it('renders placeholder when cover is missing', () => {
    const { container } = render(AlbumCard, {
      props: { id: 'album:x:y', title: 'X', artist: 'Y', trackCount: 1 },
    });

    expect(container.querySelector('.album-art-placeholder')).toBeTruthy();
  });
});
