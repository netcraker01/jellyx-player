/**
 * CSS token tests for brand migration.
 *
 * Verifies that the new --jellyx-* canonical tokens are present in the
 * tokens.css source and that --helix-* aliases are retained.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

const tokensSource = readFileSync(resolve(__dirname, 'tokens.css'), 'utf-8');

describe('Jellyx CSS tokens source', () => {
  it('defines --jellyx-gradient-primary', () => {
    expect(tokensSource).toContain('--jellyx-gradient-primary');
    expect(tokensSource).toContain('linear-gradient');
  });

  it('defines --color-jellyx-cyan', () => {
    expect(tokensSource).toMatch(/--color-jellyx-cyan:\s*#00E5FF/i);
  });

  it('keeps --helix-* aliases mapping to canonical values', () => {
    expect(tokensSource).toContain('--color-helix-cyan: var(--color-jellyx-cyan);');
    expect(tokensSource).toContain('--helix-gradient-primary: var(--jellyx-gradient-primary);');
  });
});
