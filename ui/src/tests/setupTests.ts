import { vi } from 'vitest';

// jsdom does not implement window.scrollTo, but svelte-routing calls it
// during route initialization. Suppress the resulting warning to keep test
// output focused on real failures.
Object.defineProperty(window, 'scrollTo', {
  value: vi.fn(),
  writable: true,
});
