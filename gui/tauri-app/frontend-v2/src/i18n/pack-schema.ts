// Language pack schema + strict whole-pack validation (self-localization
// wave B2, docs/development/SELFLOC_BRIDGE.md — "Language Pack Schema").
//
// Trust boundary: a pack is DATA ONLY. Values are rendered through Svelte
// text interpolation and can never reach an execution or HTML path ({@html}
// is not used anywhere in this codebase for i18n strings). Validation is
// therefore about structural integrity, not content filtering: a value
// containing "<script>" is harmless text and is accepted by schema rules
// (it renders literally).
//
// Rejection semantics: ANY structural problem rejects the WHOLE pack —
// a partially-applied pack would silently mix languages without the user
// knowing which entries came from where. An incomplete pack (not every
// catalog id present) is NOT a structural problem: missing ids legitimately
// fall back to the built-in dictionaries (en source texts / current locale).

/** Only schema version this runtime knows. Unknown versions are rejected. */
export const PACK_SCHEMA_VERSION = '1' as const;

/** Per-message value limit: catalog texts are short UI copy. */
export const PACK_VALUE_MAX_LENGTH = 4000;
/** Hard cap on message count: the full catalog is ~1.2k entries. */
export const PACK_MAX_MESSAGES = 5000;
/** Raw serialized pack size guard applied before JSON.parse. */
export const PACK_RAW_MAX_BYTES = 5_000_000;

/** Locale form: BCP47-lite — language tag plus optional subtags (zh-Hans,
 * pt-BR). Case-insensitive on purpose; a pack locale is data, not an enum:
 * unknown-but-well-formed locales are allowed (that is the point of packs). */
const LOCALE_RE = /^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$/;

/** Catalog revision shape: git short/long sha, optional -dirty suffix (same
 * shape catalog.meta.json writes). A pack built against an older revision is
 * structurally valid — revision-based rebase semantics are wave B3/B4. */
const REVISION_RE = /^[0-9a-f]{7,40}(-dirty)?$/;

/** Message id shape (SF-2): same contract as the contribution bundle's
 * CHANGE_ID_RE in scripts/contribution-schema.ts (kept as a local copy —
 * this module ships in the browser bundle and must not depend on the
 * scripts layer). No leading/trailing dots on top of the regex. */
const MESSAGE_ID_RE = /^[A-Za-z0-9_.\-\/]{1,200}$/;

function isValidMessageId(id: string): boolean {
  return MESSAGE_ID_RE.test(id) && !id.startsWith('.') && !id.endsWith('.');
}

/** {name} interpolation tokens, same syntax the i18n store replaces. */
const PLACEHOLDER_RE = /\{(\w+)\}/g;

/** Sorted, deduplicated placeholder names referenced by a message text. */
export function extractPlaceholders(text: string): string[] {
  return [...new Set([...text.matchAll(PLACEHOLDER_RE)].map((m) => m[1]))].sort();
}

export interface PackMessage {
  id: string;
  value: string;
}

export interface LanguagePack {
  schema_version: typeof PACK_SCHEMA_VERSION;
  locale: string;
  base_catalog_revision: string;
  messages: PackMessage[];
}

/** Machine-readable reject reasons (diagnostic ids, shown in the dev UI as
 * backend-style data — the established convention for non-copy strings). */
export type PackRejectReason =
  | 'malformed'
  | 'unknown_schema_version'
  | 'bad_locale'
  | 'bad_revision'
  | 'too_large'
  | 'bad_messages'
  | 'bad_message_entry'
  | 'bad_id'
  | 'duplicate_id'
  | 'unknown_id'
  | 'empty_value'
  | 'value_too_long'
  | 'placeholder_mismatch';

/** Typed load result: ok, or rejected WHOLE with a reason. There is no
 * "partially loaded" outcome by design. */
export type PackLoadResult =
  | { ok: true; pack: LanguagePack }
  | { ok: false; reason: PackRejectReason; details: string };

function reject(reason: PackRejectReason, details: string): PackLoadResult {
  return { ok: false, reason, details };
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/**
 * Validate an already-parsed pack object against the base (en) catalog.
 * Pure function — no store access, no DOM, no IO.
 *
 * `baseCatalog` is the authoritative en dictionary: pack ids must exist in
 * it and every value must interpolate exactly the same placeholder set as
 * its base text (the same contract the catalog bridge and the mod validator
 * already use for {name} tokens).
 */
export function validatePackObject(raw: unknown, baseCatalog: Record<string, string>): PackLoadResult {
  if (!isPlainObject(raw)) return reject('malformed', 'pack root must be a JSON object');

  // SF-2: a JSON-parsed `{"__proto__": {...}}` is an own data property that
  // must never be carried around or spread — refuse the pack structurally.
  if (Object.hasOwn(raw, '__proto__')) {
    return reject('malformed', 'pack root must not contain a __proto__ key (prototype-pollution guard)');
  }

  if (raw.schema_version !== PACK_SCHEMA_VERSION) {
    return reject('unknown_schema_version', `expected schema_version "${PACK_SCHEMA_VERSION}"`);
  }

  const locale = raw.locale;
  if (typeof locale !== 'string' || !LOCALE_RE.test(locale)) {
    return reject('bad_locale', `locale ${JSON.stringify(locale)} is not a valid locale tag`);
  }

  const revision = raw.base_catalog_revision;
  if (typeof revision !== 'string' || !REVISION_RE.test(revision)) {
    return reject('bad_revision', `base_catalog_revision ${JSON.stringify(revision)} is not a catalog revision`);
  }

  const messages = raw.messages;
  if (!Array.isArray(messages)) return reject('bad_messages', 'messages must be an array');
  if (messages.length > PACK_MAX_MESSAGES) {
    return reject('too_large', `messages.length ${messages.length} exceeds ${PACK_MAX_MESSAGES}`);
  }

  const seen = new Set<string>();
  const parsed: PackMessage[] = [];
  for (let i = 0; i < messages.length; i++) {
    const entry = messages[i];
    if (
      !isPlainObject(entry) ||
      typeof entry.id !== 'string' ||
      entry.id === '' ||
      typeof entry.value !== 'string'
    ) {
      return reject('bad_message_entry', `messages[${i}] must be {id: string, value: string}`);
    }
    // SF-2: shape-check the id BEFORE any catalog addressing, so inherited
    // property names can never reach baseCatalog[...].
    if (!isValidMessageId(entry.id)) {
      return reject(
        'bad_id',
        `messages[${i}] id ${JSON.stringify(entry.id)} violates the id contract (bad_id)`
      );
    }
    if (seen.has(entry.id)) return reject('duplicate_id', `id "${entry.id}" appears more than once`);
    seen.add(entry.id);

    // SF-2: hasOwn only — baseCatalog[entry.id] on an inherited name
    // ('constructor', '__proto__') resolves the prototype chain and crashes
    // the placeholder comparison on a non-string.
    if (!Object.hasOwn(baseCatalog, entry.id)) {
      return reject('unknown_id', `id "${entry.id}" does not exist in the base catalog`);
    }
    const base = baseCatalog[entry.id];
    if (!entry.value.trim()) return reject('empty_value', `value for "${entry.id}" is empty`);
    if (entry.value.length > PACK_VALUE_MAX_LENGTH) {
      return reject('value_too_long', `value for "${entry.id}" exceeds ${PACK_VALUE_MAX_LENGTH} chars`);
    }
    const packPh = extractPlaceholders(entry.value);
    const basePh = extractPlaceholders(base);
    if (JSON.stringify(packPh) !== JSON.stringify(basePh)) {
      return reject(
        'placeholder_mismatch',
        `id "${entry.id}": placeholders [${packPh}] do not match base [${basePh}]`
      );
    }
    parsed.push({ id: entry.id, value: entry.value });
  }

  return { ok: true, pack: { schema_version: PACK_SCHEMA_VERSION, locale, base_catalog_revision: revision, messages: parsed } };
}

/**
 * Entry point for raw pack text (file input, paste): size guard, JSON.parse,
 * then object validation. Parse failures are `malformed` — same whole-pack
 * rejection semantics.
 */
export function parseAndValidatePack(text: string, baseCatalog: Record<string, string>): PackLoadResult {
  if (text.length > PACK_RAW_MAX_BYTES) {
    return reject('too_large', `serialized pack exceeds ${PACK_RAW_MAX_BYTES} bytes`);
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    return reject('malformed', `not valid JSON: ${e instanceof Error ? e.message : String(e)}`);
  }
  return validatePackObject(parsed, baseCatalog);
}
