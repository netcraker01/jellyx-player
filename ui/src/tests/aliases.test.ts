/**
 * Path alias resolution test.
 *
 * Verifies that the configured Vite/TypeScript path aliases
 * resolve correctly at build time.
 * Spec: FR-001 — Build pipeline with path aliases.
 */
import { describe, it, expect } from 'vitest';

// Test that path aliases resolve — these imports prove
// the alias configuration in vite.config.ts and tsconfig.json works.
import { Source, Track, Artist, Album } from '@shared/types/models';

describe('Path aliases', () => {
  it('resolves @shared/types/models', () => {
    expect(Source).toBeDefined();
    expect(Source.YouTube).toBe('YouTube');
    expect(Source.SoundCloud).toBe('SoundCloud');
    expect(Source.Local).toBe('Local');
  });

  it('exports Track interface as a type', () => {
    // Type-only check — if this compiles, the alias works.
    // We verify the import didn't fail by checking the enum
    // which is a value export alongside the types.
    const track: Track = {
      id: 'test-1',
      source: Source.YouTube,
      sourceId: 'yt-123',
      title: 'Test Track',
      artist: 'Test Artist',
      metadata: {},
    };
    expect(track.id).toBe('test-1');
  });

  it('resolves @i18n path alias', async () => {
    // Dynamic import proves the @i18n alias works.
    const i18n = await import('@i18n');
    expect(i18n.initI18n).toBeDefined();
    expect(i18n.switchLocale).toBeDefined();
    expect(i18n.locale).toBeDefined();
  });
});