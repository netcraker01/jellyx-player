/**
 * Tests for `extractErrorMessage`.
 *
 * Covers the three required cases:
 *   - Tauri `AppError` object extraction (maps code → errors.* i18n key, falls back to details)
 *   - `Error` instance → .message
 *   - unknown type → generic message
 */
import { describe, it, expect, vi } from 'vitest';
import { extractErrorMessage } from './errors';

/** Minimal translate stub matching the real i18n signature. */
function makeTranslate(
  dict: Record<string, string>,
): (key: string, params?: Record<string, string | number>) => string {
  return (key, params) => {
    const template = dict[key];
    if (template == null) {
      // The real i18n returns the key itself when no translation exists.
      return key;
    }
    if (!params) return template;
    return Object.entries(params).reduce(
      (str, [k, v]) => str.replace(`{${k}}`, String(v)),
      template,
    );
  };
}

const translate = makeTranslate({
  'errors.PLAYBACK_ERROR': 'Playback error: {reason}',
  'errors.NETWORK_TIMEOUT': 'Connection timed out. Check your network.',
  'errors.STREAM_NOT_FOUND': 'Stream not found. It may be unavailable.',
  'errors.DEVICE_NOT_FOUND': 'Audio device not found.',
  'errors.UNKNOWN_ERROR': 'Something went wrong. Try again.',
});

describe('extractErrorMessage > AppError object', () => {
  it('maps a known code to the errors.<code> i18n key and interpolates details as {reason}', () => {
    const appError = { code: 'PLAYBACK_ERROR', details: 'decode: corrupted frame' };
    expect(extractErrorMessage(appError, translate)).toBe(
      'Playback error: decode: corrupted frame',
    );
  });

  it('maps a code without {reason} interpolation to its bare translation', () => {
    const appError = { code: 'NETWORK_TIMEOUT', details: 'upstream unreachable' };
    expect(extractErrorMessage(appError, translate)).toBe(
      'Connection timed out. Check your network.',
    );
  });

  it('uses details directly when the code has no i18n mapping', () => {
    const appError = { code: 'UNMAPPED_CODE', details: 'raw backend details' };
    expect(extractErrorMessage(appError, translate)).toBe('raw backend details');
  });

  it('falls back to the generic UNKNOWN_ERROR message when code is unmapped and details are absent', () => {
    const appError = { code: 'UNMAPPED_CODE' };
    expect(extractErrorMessage(appError, translate)).toBe(
      'Something went wrong. Try again.',
    );
  });

  it('handles a null details field', () => {
    const appError = { code: 'PLAYBACK_ERROR', details: null };
    expect(extractErrorMessage(appError, translate)).toBe('Playback error: ');
  });

  it('handles details: undefined on a known code', () => {
    const appError = { code: 'STREAM_NOT_FOUND' };
    expect(extractErrorMessage(appError, translate)).toBe(
      'Stream not found. It may be unavailable.',
    );
  });
});

describe('extractErrorMessage > Error instance', () => {
  it('returns the Error .message', () => {
    expect(extractErrorMessage(new Error('boom'), translate)).toBe('boom');
  });

  it('returns an empty string when the Error has no message', () => {
    expect(extractErrorMessage(new Error(''), translate)).toBe('');
  });
});

describe('extractErrorMessage > unknown type', () => {
  it('returns the generic message for a number', () => {
    expect(extractErrorMessage(42, translate)).toBe('Something went wrong. Try again.');
  });

  it('returns the generic message for null', () => {
    expect(extractErrorMessage(null, translate)).toBe('Something went wrong. Try again.');
  });

  it('returns the generic message for undefined', () => {
    expect(extractErrorMessage(undefined, translate)).toBe('Something went wrong. Try again.');
  });

  it('returns the generic message for a plain object without a code', () => {
    expect(extractErrorMessage({ foo: 'bar' }, translate)).toBe(
      'Something went wrong. Try again.',
    );
  });

  it('uses a string value directly', () => {
    expect(extractErrorMessage('a raw failure', translate)).toBe('a raw failure');
  });

  it('uses the translate default when no UNKNOWN_ERROR key is registered', () => {
    const emptyTranslate = makeTranslate({});
    const spy = vi.fn(emptyTranslate);
    expect(extractErrorMessage(123, spy)).toBe('Something went wrong. Try again.');
  });
});