// i18n store: locale state + t() lookup. Svelte 5 runes in a .svelte.ts module.
import { ru } from './ru';
import { en } from './en';

export type Locale = 'ru' | 'en';

const DICTS: Record<Locale, Record<string, string>> = { ru, en };
const STORAGE_KEY = 'rimloc.locale';

function readInitial(): Locale {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'ru' || v === 'en') return v;
  } catch {
    /* storage unavailable */
  }
  return 'ru'; // spec: ru is the default locale, native-speaker quality
}

export type TParams = Record<string, string | number>;

class I18nStore {
  locale = $state<Locale>(readInitial());

  constructor() {
    this.applyLang();
  }

  /** Translate a key with optional {param} interpolation. Falls back: locale -> en -> key. */
  t(key: string, params?: TParams): string {
    let text = DICTS[this.locale][key] ?? DICTS.en[key] ?? key;
    if (params) {
      for (const [name, value] of Object.entries(params)) {
        text = text.replaceAll(`{${name}}`, String(value));
      }
    }
    return text;
  }

  setLocale(locale: Locale) {
    this.locale = locale;
    try {
      localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      /* storage unavailable */
    }
    this.applyLang();
  }

  private applyLang() {
    document.documentElement.lang = this.locale;
  }
}

export const i18n = new I18nStore();

/** Convenience accessor: reactive because it reads the i18n.locale $state during render. */
export function t(key: string, params?: TParams): string {
  return i18n.t(key, params);
}
