// i18n store: locale state + t() lookup. Svelte 5 runes in a .svelte.ts module.
import { ru } from './ru';
import { en } from './en';
import { parseAndValidatePack, validatePackObject, type PackLoadResult } from './pack-schema';

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

/** State marker of an active pack preview (wave B2). Explicitly surfaced so
 * the UI (and tests) can always tell built-in rendering from pack rendering.
 * Previews are intentionally NOT persisted: they live in memory only and die
 * with the page — an unacknowledged preview can never survive a reload. */
export interface PackPreviewState {
  /** Pack locale tag from the pack file (any well-formed BCP47-lite tag). */
  locale: string;
  /** Catalog revision the pack was built against. */
  revision: string;
  /** How many messages the pack actually overrides. */
  count: number;
}

class I18nStore {
  locale = $state<Locale>(readInitial());

  /** Active pack preview, null = built-in dictionaries only. */
  preview = $state<PackPreviewState | null>(null);
  /** Overlay messages of the active preview (id -> value). */
  private packMessages = $state<Record<string, string> | null>(null);

  constructor() {
    this.applyLang();
  }

  /** Translate a key with optional {param} interpolation.
   * Resolution: pack preview overlay -> current locale -> en -> key.
   * The final `key` fallback is an honest diagnostic render (a missing id
   * shows its id), never an empty string; pack values can never introduce an
   * empty render because pack validation rejects empty values outright. */
  t(key: string, params?: TParams): string {
    let text = this.packMessages?.[key] ?? DICTS[this.locale][key] ?? DICTS.en[key] ?? key;
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

  /**
   * Load a language pack as an explicit UI preview (wave B2). Validates the
   * whole pack first; on ANY structural problem the pack is rejected whole
   * and the current state is untouched. On success the overlay is installed
   * in memory only: the UI locale setting (`locale`), localStorage and the
   * project target locales are NOT modified — a preview never outlives the
   * page and is always reverted by clearPreview().
   */
  previewPack(raw: unknown): PackLoadResult {
    // Accept both entry shapes: raw JSON text (file input) or a parsed object.
    const result =
      typeof raw === 'string' ? parseAndValidatePack(raw, en) : validatePackObject(raw, en);
    if (!result.ok) return result;
    const messages: Record<string, string> = {};
    for (const m of result.pack.messages) messages[m.id] = m.value;
    this.packMessages = messages;
    this.preview = {
      locale: result.pack.locale,
      revision: result.pack.base_catalog_revision,
      count: result.pack.messages.length
    };
    return result;
  }

  /** Explicitly drop the active preview; rendering returns to built-in. */
  clearPreview() {
    this.packMessages = null;
    this.preview = null;
  }

  get previewActive(): boolean {
    return this.preview !== null;
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
