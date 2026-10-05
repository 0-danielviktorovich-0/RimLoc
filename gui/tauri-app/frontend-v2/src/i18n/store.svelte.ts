// i18n store: locale state + t() lookup. Svelte 5 runes in a .svelte.ts module.
import { ru } from './ru';
import { en } from './en';
import { PLACEHOLDER_RE, parseAndValidatePack, validatePackObject, type PackLoadResult } from './pack-schema';

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

/** Primary language subtags whose script reads right-to-left. The preview
 * locale tag is a BCP47-lite tag ("he", "ar-EG", "fa"), so the PRIMARY
 * subtag decides; everything else renders left-to-right. */
const RTL_PRIMARY_LANGS = new Set(['ar', 'he', 'fa', 'ur', 'ps', 'sd', 'ug', 'yi', 'dv', 'ckb']);

function dirForLocale(tag: string): 'rtl' | 'ltr' {
  const primary = tag.split('-')[0]?.toLowerCase() ?? '';
  return RTL_PRIMARY_LANGS.has(primary) ? 'rtl' : 'ltr';
}

/** Pre-preview document metadata, kept so clearPreview() can restore EXACTLY
 * what the page had (lang may be absent; dir may carry a host value). */
interface DocumentLangSnapshot {
  lang: string | null;
  dir: string | null;
}

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
  /** What the document carried before the FIRST preview override (SF-08). */
  private docLangSnapshot: DocumentLangSnapshot | null = null;

  constructor() {
    this.applyLang();
  }

  /** Translate a key with optional {param} interpolation.
   * Resolution: pack preview overlay -> current locale -> en -> key.
   * The final `key` fallback is an honest diagnostic render (a missing id
   * shows its id), never an empty string; pack values can never introduce an
   * empty render because pack validation rejects empty values outright. */
  t(key: string, params?: TParams): string {
    const text = this.packMessages?.[key] ?? DICTS[this.locale][key] ?? DICTS.en[key] ?? key;
    if (!params) return text;
    // SF-07: ONE pass over the ORIGINAL message with a FUNCTION replacement.
    // The substituted value is joined in as a literal: it is never re-scanned
    // for {tokens} (a value "{b}" must not pull in b's value) and `$`
    // sequences in it are never interpreted (a STRING replacement would treat
    // $&, $`, $' and $$ as replacement patterns). A parameter name without an
    // entry in params keeps its literal {name} token — the honest diagnostic
    // render this store promises.
    return text.replace(PLACEHOLDER_RE, (token, name: string) =>
      Object.hasOwn(params, name) ? String(params[name]) : token,
    );
  }

  /** Whether the key resolves in the pack overlay, the current locale or en
   *  — the honest existence check behind backend-message localization
   *  (audit §4/§7): a caller maps a stable backend id onto an i18n key and
   *  falls back to the raw server message when this returns false, so an
   *  unknown future id can never render as its own key. */
  has(key: string): boolean {
    return (
      this.packMessages?.[key] !== undefined ||
      DICTS[this.locale][key] !== undefined ||
      DICTS.en[key] !== undefined
    );
  }

  setLocale(locale: Locale) {
    this.locale = locale;
    try {
      localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      /* storage unavailable */
    }
    // SF-08: a preference change MID-PREVIEW updates what "restore" means
    // for lang — the latest user decision wins over the pre-preview value.
    // (dir keeps its pre-preview restore target: the preference does not
    // speak about direction, the rendered locale does.)
    if (this.preview && this.docLangSnapshot) this.docLangSnapshot.lang = locale;
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
    // SF-08: the document metadata must reflect the language ACTUALLY
    // RENDERED. The first override snapshots what the page had (nested
    // previewPack calls must not overwrite the original snapshot); a
    // rejected pack above never reached this line, so the document is only
    // touched after the whole pack validated.
    if (!this.preview) {
      this.docLangSnapshot = {
        lang: document.documentElement.getAttribute('lang'),
        dir: document.documentElement.getAttribute('dir')
      };
    }
    this.packMessages = messages;
    this.preview = {
      locale: result.pack.locale,
      revision: result.pack.base_catalog_revision,
      count: result.pack.messages.length
    };
    this.applyLang();
    return result;
  }

  /** Explicitly drop the active preview; rendering — and the document
   * lang/dir metadata — return to the pre-preview state. */
  clearPreview() {
    this.packMessages = null;
    this.preview = null;
    this.applyLang();
  }

  get previewActive(): boolean {
    return this.preview !== null;
  }

  /**
   * SF-08: single sync point for the document `<html lang>` / `<dir>`
   * metadata. With an active pack preview the EFFECTIVE rendered language
   * is the preview locale — the metadata must say so (a Japanese preview
   * under `html lang="ru"` is wrong metadata, and a RTL preview needs
   * `dir="rtl"` to render honestly) — regardless of which UI locale the
   * user prefers. Without a preview the UI locale applies and the exact
   * pre-preview metadata is restored.
   */
  private applyLang() {
    if (this.preview) {
      document.documentElement.lang = this.preview.locale;
      document.documentElement.dir = dirForLocale(this.preview.locale);
      return;
    }
    const snap = this.docLangSnapshot;
    this.docLangSnapshot = null;
    if (snap) {
      if (snap.lang === null) document.documentElement.removeAttribute('lang');
      else document.documentElement.lang = snap.lang;
      if (snap.dir === null) document.documentElement.removeAttribute('dir');
      else document.documentElement.dir = snap.dir;
      return;
    }
    document.documentElement.lang = this.locale;
  }
}

export const i18n = new I18nStore();

/** Convenience accessor: reactive because it reads the i18n.locale $state during render. */
export function t(key: string, params?: TParams): string {
  return i18n.t(key, params);
}
