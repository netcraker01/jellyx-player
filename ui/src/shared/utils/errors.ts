/**
 * Error message extraction for Tauri IPC errors.
 *
 * Tauri commands return structured `AppError { code, details }` objects — not
 * `Error` instances — when they fail (see `jellyx-desktop/src/errors/types.rs`).
 * Naive `String(e)` on these plain objects produces `[object Object]`, which is
 * what users saw in error toasts. This helper extracts a user-facing message
 * from any caught value, mapping the structured `code` to an existing
 * `errors.*` i18n key, then falling back to `details`, then to a generic
 * message.
 *
 * Extraction order:
 *   1. `Error` instance          → `e.message`
 *   2. AppError object `{code, details?}` → `translate('errors.<code>', {reason: details})`;
 *      if no translation exists for that code → `details` → generic message.
 *   3. `string`                  → the string itself
 *   4. anything else             → `translate('errors.UNKNOWN_ERROR')`
 */

/** Shape of the structured error Tauri serializes from Rust `AppError`. */
interface AppErrorLike {
  code: string;
  details?: string | null;
}

/** Generic fallback message used when no details are available. */
const GENERIC_FALLBACK = 'Something went wrong. Try again.';

/**
 * Extract a user-facing message from a thrown value.
 *
 * @param e         The caught value (`Error`, `AppError` object, `string`, or unknown).
 * @param translate The i18n translate function (obtained via `get(t)`).
 * @returns A localized, human-readable error message.
 */
export function extractErrorMessage(
  e: unknown,
  translate: (key: string, params?: Record<string, string | number>) => string,
): string {
  // 1. Standard Error instance → use its .message.
  if (e instanceof Error) {
    return e.message;
  }

  // 2. Tauri AppError: a plain object with { code, details? }.
  if (e !== null && typeof e === 'object' && 'code' in e) {
    const { code, details } = e as AppErrorLike;
    if (typeof code === 'string' && code.length > 0) {
      const i18nKey = `errors.${code}`;
      // Pass details as the {reason} interpolation param (used by errors.PLAYBACK_ERROR).
      // The `default` param is a test-time fallback convention used across the codebase.
      const translated = translate(i18nKey, {
        reason: details ?? '',
        default: details ?? '',
      });
      // If translate returned the key unchanged, no translation exists for this
      // code — fall through to details / generic below.
      if (translated !== i18nKey) {
        return translated;
      }
    }
    // No i18n mapping for this code: use details directly if available.
    if (details) {
      return details;
    }
    return translate('errors.UNKNOWN_ERROR', { default: GENERIC_FALLBACK });
  }

  // 3. Raw string → use it directly.
  if (typeof e === 'string') {
    return e;
  }

  // 4. Unknown type → generic message.
  const unknownKey = 'errors.UNKNOWN_ERROR';
  const unknownTranslated = translate(unknownKey, { default: GENERIC_FALLBACK });
  // If no UNKNOWN_ERROR translation is registered, the real i18n returns the
  // raw key — fall back to the hardcoded generic message so users never see a
  // raw `errors.UNKNOWN_ERROR` string in a toast.
  return unknownTranslated === unknownKey ? GENERIC_FALLBACK : unknownTranslated;
}