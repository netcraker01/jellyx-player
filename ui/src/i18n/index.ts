/**
 * Jellyx i18n system.
 *
 * Reactive Svelte store for translations.
 * Backend sends error codes → frontend maps to translated strings.
 * No restart needed on language switch.
 */

import { writable, derived } from 'svelte/store';

// ── Types ──────────────────────────────────────────

type LocaleCode = string;
type TranslationMap = Record<string, string | Record<string, unknown>>;

// ── State ──────────────────────────────────────────

/** Current locale code (e.g. 'en', 'es') */
export const locale = writable<LocaleCode>('en');

/** Loaded translation dictionary for current locale */
export const translations = writable<TranslationMap>({});

/** Reactive translate function — updates when locale changes */
export const t = derived(translations, ($t) => {
  return (key: string, params?: Record<string, string | number>): string => {
    const keys = key.split('.');
    let value: unknown = $t;

    for (const k of keys) {
      if (value && typeof value === 'object' && k in value) {
        value = (value as Record<string, unknown>)[k];
      } else {
        return key; // fallback: show the key itself
      }
    }

    if (typeof value !== 'string') return key;

    if (params) {
      return Object.entries(params).reduce(
        (str, [k, v]) => str.replace(`{${k}}`, String(v)),
        value,
      );
    }

    return value;
  };
});

// ── Locale detection ───────────────────────────────

/**
 * Detect system language from browser/OS.
 * Returns ISO code like 'en', 'es', 'pt-BR'.
 */
export function detectSystemLocale(): LocaleCode {
  try {
    const lang = navigator.language || (navigator as any).userLanguage || 'en';
    return lang.split('-')[0]; // 'es-MX' → 'es'
  } catch {
    return 'en';
  }
}

// ── Load translations ──────────────────────────────

const translationCache = new Map<LocaleCode, TranslationMap>();

async function loadTranslations(code: LocaleCode): Promise<TranslationMap> {
  // Check cache first
  if (translationCache.has(code)) {
    return translationCache.get(code)!;
  }

  try {
    const mod = await import(`./locales/${code}.json`);
    translationCache.set(code, mod.default);
    return mod.default;
  } catch {
    // Fallback to English
    if (code !== 'en') {
      return loadTranslations('en');
    }
    return {};
  }
}

// ── Switch locale ──────────────────────────────────

/**
 * Switch the application language.
 * All reactive translations update instantly.
 */
export async function switchLocale(code: LocaleCode): Promise<void> {
  const dict = await loadTranslations(code);
  locale.set(code);
  translations.set(dict);

  // Persist preference to canonical key; legacy key is left intact as fallback.
  try {
    localStorage.setItem('jellyx-locale', code);
  } catch {
    // localStorage might not be available
  }
}

// ── Initialize ─────────────────────────────────────

/**
 * Initialize i18n system.
 * Call once at app startup.
 */
export async function initI18n(): Promise<void> {
  // Check saved preference first — canonical jellyx-locale wins over legacy helix-locale.
  let code: LocaleCode | null = null;
  try {
    code = localStorage.getItem('jellyx-locale') ?? localStorage.getItem('helix-locale');
  } catch {
    // ignore
  }

  if (!code) {
    code = detectSystemLocale();
  }

  const dict = await loadTranslations(code);
  locale.set(code);
  translations.set(dict);
}
