// COPY PROVENANCE: snapshot of the frozen Svelte language registry
// (frontend-v2/src/lib/languages/registry.ts, framework-neutral pure data).
// Phase E converges both on a shared module — until then THIS copy is the
// React lane's authority; do not edit the frozen original.
// Language registry (W2, LANGUAGE_REGISTRY_MANDATE): every language the GUI can
// name — builtin definitions with per-language capabilities plus user-defined
// languages. Adapters by canonical model semantics: a language is project
// metadata (the target side of Project.translations), NOT frontend-only state —
// the same locale ids the backend writes into Translation.locale.
//
// Registry invariants (mandate):
// - Unknown locale NEVER rejects: resolveLanguage() returns a Generic fallback
//   definition so a dataset from a newer/other build still opens.
// - User languages get Generic capabilities (everything a translator needs is
//   assumed available; morphology is not).
// - Custom languages persist to localStorage; the store re-reads on load.
// - wordinfo is evidence-based (OFFICIAL_LANG_PACKS.md §2): only ru, de, uk
//   ship WordInfo grammar tables in the official language repos; en is the
//   canonical source locale and ja/zh are non-inflected (CJK).

export type Script = 'latin' | 'cyrillic' | 'cjk' | 'unknown';

export type Direction = 'ltr' | 'rtl';

/** What RimLoc (and RimWorld) can do for this language as a TARGET. */
export interface LanguageCapabilities {
  /** Usable as a translation target at all. */
  translation: boolean;
  /** Translation memory applies. */
  tm: boolean;
  /** Glossary support. */
  glossary: boolean;
  /** AI drafting applies. */
  ai: boolean;
  /** Editable in the workspace editor. */
  editor: boolean;
  /** Structural validation (placeholders, TKey, markup). */
  structural: boolean;
  /** WordInfo grammar tables ({lookup:} resolution) exist for this language. */
  wordinfo: boolean;
  /** Language-specific morphology worker (cases, plurals) — ru/de/uk class. */
  morphology: boolean;
}

/** One registry entry: how the GUI names and treats a language. */
export interface LanguageDefinition {
  /** Locale id used in Project.translations and the target switcher (BCP-47-ish). */
  localeId: string;
  /** English display name ("Russian"). */
  displayName: string;
  /** Native name ("Русский") — shown in the switcher and manager. */
  nativeName: string;
  /** BCP-47 tag for UI attributes / export metadata. */
  bcp47: string;
  /** RimWorld Languages/<folder> name (mod folders + official pack tars). */
  rimworldFolder: string;
  script: Script;
  direction: Direction;
  capabilities: LanguageCapabilities;
  /** Where the definition came from. */
  origin: 'builtin' | 'user';
}

const GENERIC_CAPABILITIES: LanguageCapabilities = {
  translation: true,
  tm: true,
  glossary: true,
  ai: true,
  editor: true,
  structural: true,
  wordinfo: false,
  morphology: false
};

function caps(wordinfo: boolean, morphology: boolean): LanguageCapabilities {
  return { ...GENERIC_CAPABILITIES, wordinfo, morphology };
}

/**
 * Builtin languages (mandate): eight first-class targets. wordinfo/morphology
 * follow the official-pack evidence (ru/de/uk have WordInfo; ja/zh are CJK and
 * do not inflect; pl/es ship no WordInfo tables).
 */
export const BUILTIN_LANGUAGES: LanguageDefinition[] = [
  {
    localeId: 'en',
    displayName: 'English',
    nativeName: 'English',
    bcp47: 'en',
    rimworldFolder: 'English',
    script: 'latin',
    direction: 'ltr',
    capabilities: caps(false, false),
    origin: 'builtin'
  },
  {
    localeId: 'ru',
    displayName: 'Russian',
    nativeName: 'Русский',
    bcp47: 'ru',
    rimworldFolder: 'Russian (Русский)',
    script: 'cyrillic',
    direction: 'ltr',
    capabilities: caps(true, true),
    origin: 'builtin'
  },
  {
    localeId: 'uk',
    displayName: 'Ukrainian',
    nativeName: 'Українська',
    bcp47: 'uk',
    rimworldFolder: 'Ukrainian (Українська)',
    script: 'cyrillic',
    direction: 'ltr',
    capabilities: caps(true, true),
    origin: 'builtin'
  },
  {
    localeId: 'ja',
    displayName: 'Japanese',
    nativeName: '日本語',
    bcp47: 'ja',
    rimworldFolder: 'Japanese (日本語)',
    script: 'cjk',
    direction: 'ltr',
    capabilities: caps(false, false),
    origin: 'builtin'
  },
  {
    localeId: 'de',
    displayName: 'German',
    nativeName: 'Deutsch',
    bcp47: 'de',
    rimworldFolder: 'German (Deutsch)',
    script: 'latin',
    direction: 'ltr',
    capabilities: caps(true, true),
    origin: 'builtin'
  },
  {
    localeId: 'pl',
    displayName: 'Polish',
    nativeName: 'Polski',
    bcp47: 'pl',
    rimworldFolder: 'Polish (Polski)',
    script: 'latin',
    direction: 'ltr',
    capabilities: caps(false, false),
    origin: 'builtin'
  },
  {
    localeId: 'es',
    displayName: 'Spanish',
    nativeName: 'Español',
    bcp47: 'es',
    rimworldFolder: 'Spanish (Español)',
    script: 'latin',
    direction: 'ltr',
    capabilities: caps(false, false),
    origin: 'builtin'
  },
  {
    localeId: 'zh-Hans',
    displayName: 'Chinese (Simplified)',
    nativeName: '简体中文',
    bcp47: 'zh-Hans',
    rimworldFolder: 'ChineseSimplified (简体中文)',
    script: 'cjk',
    direction: 'ltr',
    capabilities: caps(false, false),
    origin: 'builtin'
  }
];

const BUILTIN_BY_ID = new Map(BUILTIN_LANGUAGES.map((l) => [l.localeId, l]));

export const SOURCE_LOCALE = 'en';
export const SOURCE_LANGUAGE: LanguageDefinition = BUILTIN_BY_ID.get('en')!;

/** localStorage key for user-defined languages. */
const CUSTOM_KEY = 'rimloc.languages.custom.v1';

/**
 * Locale-id shape check (port of the Svelte registry's isValidLocaleId,
 * registry.ts:246 — acceptance MUST-FIX: «1плохо код!» принимался, потому
 * что React-лейн валидатор не вызывал вовсе). BCP-47-ish: 2-3 буквенных
 * базовых сегмента, дальше любые алфавитно-цифровые subtag'и через дефис.
 */
export function isValidLocaleId(id: string): boolean {
  return /^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(id);
}

/** Fallback label for unknown locales shown in UI ("Language xq-42"). */
export function genericDisplayName(localeId: string): string {
  return `Language ${localeId}`;
}

/** Build a Generic-capability definition for a locale id we know nothing about. */
export function genericDefinition(localeId: string): LanguageDefinition {
  return {
    localeId,
    displayName: genericDisplayName(localeId),
    nativeName: localeId,
    bcp47: localeId,
    rimworldFolder: localeId,
    script: 'unknown',
    direction: 'ltr',
    capabilities: { ...GENERIC_CAPABILITIES },
    origin: 'user'
  };
}

function readCustom(): LanguageDefinition[] {
  try {
    const raw = localStorage.getItem(CUSTOM_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const out: LanguageDefinition[] = [];
    for (const item of parsed) {
      const def = coerceDefinition(item);
      if (def) out.push(def);
    }
    return out;
  } catch {
    return [];
  }
}

function coerceDefinition(item: unknown): LanguageDefinition | null {
  if (typeof item !== 'object' || item === null) return null;
  const o = item as Record<string, unknown>;
  if (typeof o.localeId !== 'string' || !o.localeId) return null;
  return {
    localeId: o.localeId,
    displayName: typeof o.displayName === 'string' && o.displayName ? o.displayName : genericDisplayName(o.localeId),
    nativeName: typeof o.nativeName === 'string' && o.nativeName ? o.nativeName : o.localeId,
    bcp47: typeof o.bcp47 === 'string' && o.bcp47 ? o.bcp47 : o.localeId,
    rimworldFolder: typeof o.rimworldFolder === 'string' && o.rimworldFolder ? o.rimworldFolder : o.localeId,
    script: (o.script as Script) ?? 'unknown',
    direction: o.direction === 'rtl' ? 'rtl' : 'ltr',
    capabilities: { ...GENERIC_CAPABILITIES },
    origin: 'user'
  };
}

function writeCustom(list: LanguageDefinition[]) {
  try {
    localStorage.setItem(CUSTOM_KEY, JSON.stringify(list));
  } catch {
    /* storage unavailable — user languages stay session-only */
  }
}

/**
 * Registry facade: builtins + persisted user languages + never-reject resolve.
 * Deliberately a plain (non-runes) class: pure logic, runnable headless. UI
 * reactivity mirrors `custom` through the multi-target store ($state).
 */
class LanguageRegistry {
  custom: LanguageDefinition[] = readCustom();

  /** All known languages, builtins first. */
  all(): LanguageDefinition[] {
    return [...BUILTIN_LANGUAGES, ...this.custom];
  }

  /** Languages that may be added as targets (source excluded). */
  addable(): LanguageDefinition[] {
    return this.all().filter((l) => l.localeId !== SOURCE_LOCALE);
  }

  /** NEVER rejects: unknown locale → Generic fallback definition. */
  resolve(localeId: string): LanguageDefinition {
    const builtin = BUILTIN_BY_ID.get(localeId);
    if (builtin) return builtin;
    const custom = this.custom.find((l) => l.localeId === localeId);
    if (custom) return custom;
    return genericDefinition(localeId);
  }

  isBuiltin(localeId: string): boolean {
    return BUILTIN_BY_ID.has(localeId);
  }

  isCustom(localeId: string): boolean {
    return this.custom.some((l) => l.localeId === localeId);
  }

  isKnown(localeId: string): boolean {
    return this.isBuiltin(localeId) || this.isCustom(localeId);
  }

  /** Locale id syntax: 2-3 letter base + optional script/region subtags. */
  isValidLocaleId(id: string): boolean {
    return /^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(id);
  }

  /**
   * Create a user language with Generic capabilities and persist it.
   * Validation errors are returned as strings (UI dialog shows them);
   * a colliding builtin id is reported, not silently merged.
   */
  createCustom(input: {
    displayName: string;
    nativeName: string;
    localeId: string;
    rimworldFolder?: string;
    script?: Script;
    direction?: Direction;
  }): { ok: true; language: LanguageDefinition } | { ok: false; error: 'invalid-id' | 'duplicate' | 'bad-name' } {
    const localeId = input.localeId.trim();
    if (!this.isValidLocaleId(localeId)) return { ok: false, error: 'invalid-id' };
    if (this.isKnown(localeId)) return { ok: false, error: 'duplicate' };
    const displayName = input.displayName.trim();
    if (!displayName) return { ok: false, error: 'bad-name' };
    const def: LanguageDefinition = {
      localeId,
      displayName,
      nativeName: input.nativeName.trim() || localeId,
      bcp47: localeId,
      rimworldFolder: input.rimworldFolder?.trim() || localeId,
      script: input.script ?? 'unknown',
      direction: input.direction ?? 'ltr',
      capabilities: { ...GENERIC_CAPABILITIES },
      origin: 'user'
    };
    this.custom = [...this.custom, def];
    writeCustom(this.custom);
    return { ok: true, language: def };
  }

  /** Remove a user language. Builtins cannot be removed (returns false). */
  removeCustom(localeId: string): boolean {
    if (!this.isCustom(localeId)) return false;
    this.custom = this.custom.filter((l) => l.localeId !== localeId);
    writeCustom(this.custom);
    return true;
  }
}

export const registry = new LanguageRegistry();
