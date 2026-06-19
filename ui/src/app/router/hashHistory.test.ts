import { describe, expect, it, beforeEach } from 'vitest';
import { hashHistory } from './hashHistory';

describe('hashHistory', () => {
  beforeEach(() => {
    window.history.replaceState({}, '', '/');
    window.location.hash = '';
  });

  it('navigates using hash URLs', () => {
    hashHistory.navigate('/library');
    expect(window.location.hash).toBe('#/library');
    expect(hashHistory.location.pathname).toBe('/library');
  });

  it('can navigate back to root', () => {
    hashHistory.navigate('/');
    expect(hashHistory.location.pathname).toBe('/');
  });
});
