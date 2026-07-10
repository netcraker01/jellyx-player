/**
 * BrandLockup tests.
 *
 * Verifies the sidebar header renders the Jellyx logo and wordmark
 * from the new brand assets.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import BrandLockup from './BrandLockup.svelte';
import { initI18n } from '@i18n';

describe('BrandLockup', () => {
  beforeEach(async () => {
    await initI18n();
  });

  it('renders the Jellyx wordmark', () => {
    const { getByText } = render(BrandLockup);
    expect(getByText('Jellyx')).toBeTruthy();
  });

  it('renders the Jellyx logo with the correct aria label', () => {
    const { getByLabelText } = render(BrandLockup);
    expect(getByLabelText('Jellyx icon')).toBeTruthy();
  });

  it('uses a default label prop of "Jellyx" for the brand lockup', () => {
    const { getByRole } = render(BrandLockup);
    expect(getByRole('banner').getAttribute('aria-label')).toBe('Jellyx');
  });

  it('accepts a custom label prop and reflects it on the banner', () => {
    const { getByRole } = render(BrandLockup, { props: { label: 'Jellyx Player' } });
    expect(getByRole('banner').getAttribute('aria-label')).toBe('Jellyx Player');
  });
});
