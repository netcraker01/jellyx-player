/**
 * Sidebar navigation test.
 *
 * Verifies that the sidebar exposes Library and Settings routes
 * alongside the existing primary routes.
 *
 * Spec: FR-011 — App shell navigation exposes all routes.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { fireEvent, render } from '@testing-library/svelte';
import SidebarTestWrapper from './SidebarTestWrapper.svelte';
import { initI18n } from '@i18n';

describe('Sidebar navigation', () => {
  beforeEach(async () => {
    await initI18n();
    window.history.replaceState({}, '', '/');
    window.location.hash = '';
  });

  it('renders the Library navigation button', () => {
    const { getByText } = render(SidebarTestWrapper);
    expect(getByText('Library')).toBeTruthy();
  });

  it('renders the Settings navigation button', () => {
    const { getByText } = render(SidebarTestWrapper);
    expect(getByText('Settings')).toBeTruthy();
  });

  it('renders all primary routes', () => {
    const { getByText } = render(SidebarTestWrapper);
    for (const label of ['Home', 'Search', 'Lists', 'Now Playing', 'Library', 'Settings']) {
      expect(getByText(label)).toBeTruthy();
    }
  });

  it('navigates using hash history when clicking a sidebar button', async () => {
    const { getByText } = render(SidebarTestWrapper);
    await fireEvent.click(getByText('Library'));
    expect(window.location.hash).toBe('#/library');
  });
});
