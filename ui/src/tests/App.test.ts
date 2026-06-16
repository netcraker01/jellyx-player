/**
 * App.svelte smoke test.
 *
 * Verifies the app shell structure: that App.svelte is a valid Svelte
 * component, and that its sub-components (Sidebar, BottomBar) can be
 * imported and are renderable in isolation.
 *
 * Spec: FR-011 — App shell with Sidebar + Router + BottomBar.
 *
 * Note: Full App rendering requires a browser context with History API
 * (svelte-routing uses pushState). We test sub-components in isolation
 * where the Router context is not needed, and verify the module graph
 * is intact by checking imports resolve.
 */
import { describe, it, expect } from 'vitest';

import App from '../app/App.svelte';
import Sidebar from '../app/layout/Sidebar.svelte';
import BottomBar from '../app/layout/BottomBar.svelte';

describe('App shell module graph', () => {
  it('App.svelte is a valid Svelte component', () => {
    // A compiled Svelte component is a function/class with a render method
    expect(typeof App).toBe('function');
  });

  it('Sidebar.svelte is a valid Svelte component', () => {
    expect(typeof Sidebar).toBe('function');
  });

  it('BottomBar.svelte is a valid Svelte component', () => {
    expect(typeof BottomBar).toBe('function');
  });
});

describe('BottomBar renders independently', () => {
  // BottomBar has no svelte-routing dependency — safe to render
  it('renders the bottom bar with player controls', async () => {
    const { render } = await import('@testing-library/svelte');
    const { container } = render(BottomBar);
    const bottomBar = container.querySelector('.bottom-bar');
    expect(bottomBar).toBeTruthy();
  });

  it('contains play and skip controls', async () => {
    const { render } = await import('@testing-library/svelte');
    const { container } = render(BottomBar);
    const buttons = container.querySelectorAll('button');
    expect(buttons.length).toBeGreaterThanOrEqual(3); // prev, play, next
  });
});