/**
 * Asset URL utility tests.
 *
 * Verifies that albumArtUrl correctly handles:
 * - undefined input
 * - HTTPS remote URLs (pass-through)
 * - Local filesystem paths (Tauri asset protocol / browser fallback)
 *
 * Note: `isTauri` is evaluated at module import time (captured from window
 * global). Tests that need to simulate a Tauri environment must set the
 * window mock BEFORE the module under test is loaded, which requires a
 * dynamic import or careful hoisting.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';

const mocks = vi.hoisted(() => ({
  convertFileSrc: vi.fn((path: string) => `asset://mock/${path}`),
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: mocks.convertFileSrc,
}));

import { albumArtUrl } from './assetUrl';

describe('albumArtUrl', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('returns undefined for undefined input', () => {
    expect(albumArtUrl(undefined)).toBeUndefined();
    expect(mocks.convertFileSrc).not.toHaveBeenCalled();
  });

  it('returns undefined for empty string', () => {
    expect(albumArtUrl('')).toBeUndefined();
    expect(mocks.convertFileSrc).not.toHaveBeenCalled();
  });

  it('blocks plain HTTP URLs (not allowed by CSP)', () => {
    const url = 'http://img.youtube.com/vi/abc123/0.jpg';
    expect(albumArtUrl(url)).toBeUndefined();
    expect(mocks.convertFileSrc).not.toHaveBeenCalled();
  });

  it('passes through remote HTTPS URLs unchanged', () => {
    const url = 'https://img.youtube.com/vi/abc123/0.jpg';
    expect(albumArtUrl(url)).toBe(url);
    expect(mocks.convertFileSrc).not.toHaveBeenCalled();
  });

  it('returns undefined for local paths outside Tauri (browser fallback)', () => {
    // In the test environment, window exists but __TAURI_INTERNALS__ does not,
    // so albumArtUrl should return undefined for local paths.
    const path = '/home/user/.local/share/helix/art/cover.jpg';
    expect(albumArtUrl(path)).toBeUndefined();
    expect(mocks.convertFileSrc).not.toHaveBeenCalled();
  });

  it('returns undefined for data URIs (not http/https)', () => {
    // Data URIs are intentionally not passed through — they are unusual for
    // album art and would need explicit extension if required.
    const dataUrl = 'data:image/png;base64,iVBORw0KGgo=';
    expect(albumArtUrl(dataUrl)).toBeUndefined();
    expect(mocks.convertFileSrc).not.toHaveBeenCalled();
  });
});
