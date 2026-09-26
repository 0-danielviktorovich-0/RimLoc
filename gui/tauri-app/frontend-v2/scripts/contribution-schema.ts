/**
 * Shared contract for the offline contribution bundle
 * (docs/development/SELFLOC_BRIDGE.md, "Contribution Bundle" section).
 *
 * A bundle is TRANSLATION DATA ONLY: no executable code, no file paths, no
 * tool metadata. Both directions of the offline contribution flow consume
 * this module:
 *   - scripts/build-contribution.ts  (translator changes -> bundle)
 *   - scripts/apply-contribution.ts  (approved bundle -> TS dictionaries)
 *
 * Sanitization contract (owner mandate §11/§21): every producer MUST emit
 * only the schema fields declared here; unknown fields of a caller-supplied
 * payload are dropped by the builder and REFUSED by the applier. Secret
 * patterns are never carried into a bundle — the builder treats a secret
 * hit as a validation failure for the entry.
 */

/** Bundle schema version; additive changes are minor, breaking ones bump this. */
export const BUNDLE_SCHEMA_VERSION = '1';

/** Marker that the payload is a UI translation contribution, nothing else. */
export const BUNDLE_KIND = 'rimloc-ui-translation';

/** The source locale is authoritative and never accepts contributions. */
export const SOURCE_LOCALE = 'en';

/** Hard cap on a single translated value (current catalog max is 230 chars). */
export const VALUE_MAX_LEN = 1000;

/** Hard cap on contributor display name / note length. */
export const CONTRIBUTOR_NAME_MAX_LEN = 120;
export const CONTRIBUTOR_NOTE_MAX_LEN = 500;

/** Hard cap on changes per bundle (catalog is ~1.2k messages; generous headroom). */
export const CHANGES_MAX_COUNT = 5000;

/**
 * Locale tag shape: lowercase language, optional subtags. Deliberately
 * rejects `/`, `\`, `..`, whitespace — the locale is used to derive
 * `src/i18n/<locale>.ts` paths, so traversal characters must be impossible.
 */
export const LOCALE_RE = /^[a-z]{2,3}(?:-[A-Za-z0-9]+)*$/;

/** catalog_revision / base_catalog_revision shape: git sha (7-40 hex) with optional -dirty. */
export const REVISION_RE = /^[0-9a-f]{7,40}(?:-dirty)?$/;

/** One translation change. Exactly these two fields, nothing else. */
export interface BundleChange {
  id: string;
  value: string;
}

/** Optional attribution. Exactly these fields when present. */
export interface BundleContributor {
  display_name?: string;
  note?: string;
}

/** The contribution bundle: translation data only (schema v1). */
export interface ContributionBundle {
  schema_version: typeof BUNDLE_SCHEMA_VERSION;
  kind: typeof BUNDLE_KIND;
  locale: string;
  base_catalog_revision: string;
  changes: BundleChange[];
  contributor?: BundleContributor;
}

/** Strip the honest-but-incidental `-dirty` suffix for staleness comparison. */
export function normalizeRevision(revision: string): string {
  return revision.replace(/-dirty$/, '');
}

/**
 * Secret-pattern scan over arbitrary text. Returns PATTERN NAMES only —
 * never the matched text, so a hit can be reported without leaking the
 * secret into logs, previews or test output.
 */
const SECRET_PATTERNS: Array<{ name: string; re: RegExp }> = [
  { name: 'aws-access-key', re: /\bAKIA[0-9A-Z]{16}\b/ },
  { name: 'github-token', re: /\bgh[oprsu]_[A-Za-z0-9]{30,}\b/ },
  { name: 'github-fine-grained-token', re: /\bgithub_pat_[A-Za-z0-9_]{60,}\b/ },
  { name: 'slack-token', re: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/ },
  { name: 'api-key-prefix', re: /\bsk-[A-Za-z0-9_-]{20,}\b/ },
  {
    name: 'credential-assignment',
    re: /\b(?:password|passwd|secret|token|api[_-]?key|access[_-]?key)\b\s*[:=]\s*\S{8,}/i,
  },
  { name: 'private-key-block', re: /-----BEGIN [A-Z ]*PRIVATE KEY-----/ },
];

/** Pattern names whose signature the text matches (empty = clean). */
export function scanSecrets(text: string): string[] {
  return SECRET_PATTERNS.filter((p) => p.re.test(text)).map((p) => p.name);
}

/** True when the locale may receive translations (exists and is not the source). */
export function isContributableLocale(locale: string): boolean {
  return LOCALE_RE.test(locale) && locale !== SOURCE_LOCALE;
}

/**
 * Escape a plain string into a single-quoted TS literal body. Non-ASCII is
 * written as-is (the dictionaries are UTF-8 with raw Cyrillic already);
 * only characters that would break the literal are escaped, backslash first.
 */
export function escapeTsValue(value: string): string {
  return value
    .replace(/\\/g, '\\\\')
    .replace(/'/g, "\\'")
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

/** Escape a plain string for embedding inside a RegExp source. */
export function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** Canonical bundle change order: by id. Keeps output deterministic. */
export function sortChangesById(changes: BundleChange[]): BundleChange[] {
  return [...changes].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

export type ParseResult<T> = { ok: true; value: T } | { ok: false; errors: string[] };

const BUNDLE_ROOT_FIELDS = [
  'schema_version',
  'kind',
  'locale',
  'base_catalog_revision',
  'changes',
  'contributor',
] as const;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseContributor(raw: unknown, errors: string[]): BundleContributor | undefined {
  if (raw === undefined) return undefined;
  if (!isPlainObject(raw)) {
    errors.push('contributor must be an object with optional display_name / note strings');
    return undefined;
  }
  const allowed = ['display_name', 'note'] as const;
  const extra = Object.keys(raw).filter((k) => !allowed.includes(k as (typeof allowed)[number]));
  if (extra.length > 0) {
    errors.push(`contributor: unexpected field(s) ${JSON.stringify(extra)}`);
  }
  const out: BundleContributor = {};
  for (const key of allowed) {
    const v = raw[key];
    if (v === undefined) continue;
    if (typeof v !== 'string') {
      errors.push(`contributor.${key} must be a string`);
      continue;
    }
    const max = key === 'display_name' ? CONTRIBUTOR_NAME_MAX_LEN : CONTRIBUTOR_NOTE_MAX_LEN;
    if (v.length > max) {
      errors.push(`contributor.${key} exceeds ${max} characters`);
      continue;
    }
    out[key] = v;
  }
  return Object.keys(out).length > 0 ? out : undefined;
}

/**
 * Strict parse of an UNTRUSTED bundle payload: schema fields exactly, no
 * extras anywhere, source locale refused. The applier refuses to run on any
 * error — an approved bundle is exactly what the schema says, byte for byte.
 */
export function parseContributionBundle(raw: unknown): ParseResult<ContributionBundle> {
  const errors: string[] = [];
  if (!isPlainObject(raw)) {
    return { ok: false, errors: ['bundle root must be a JSON object'] };
  }
  const extra = Object.keys(raw).filter((k) => !BUNDLE_ROOT_FIELDS.includes(k as never));
  if (extra.length > 0) {
    errors.push(
      `root: unexpected field(s) ${JSON.stringify(extra)} (schema allows ${JSON.stringify(
        BUNDLE_ROOT_FIELDS,
      )})`,
    );
  }
  if (raw.schema_version !== BUNDLE_SCHEMA_VERSION) {
    errors.push(`schema_version must be ${JSON.stringify(BUNDLE_SCHEMA_VERSION)}`);
  }
  if (raw.kind !== BUNDLE_KIND) {
    errors.push(`kind must be ${JSON.stringify(BUNDLE_KIND)}`);
  }
  const locale = raw.locale;
  if (typeof locale !== 'string' || !isContributableLocale(locale)) {
    errors.push(
      `locale must match ${LOCALE_RE.source} and must not be the source locale ${JSON.stringify(
        SOURCE_LOCALE,
      )}`,
    );
  }
  const base = raw.base_catalog_revision;
  if (typeof base !== 'string' || !REVISION_RE.test(base)) {
    errors.push('base_catalog_revision must be a git sha (7-40 hex) with optional -dirty suffix');
  }
  if (!Array.isArray(raw.changes)) {
    errors.push('changes must be an array of {id, value}');
  } else {
    if (raw.changes.length === 0) errors.push('changes must not be empty');
    if (raw.changes.length > CHANGES_MAX_COUNT) {
      errors.push(`changes exceeds ${CHANGES_MAX_COUNT} entries`);
    }
    raw.changes.forEach((entry, i) => {
      if (!isPlainObject(entry)) {
        errors.push(`changes[${i}] must be an object`);
        return;
      }
      const entryExtra = Object.keys(entry).filter((k) => k !== 'id' && k !== 'value');
      if (entryExtra.length > 0) {
        errors.push(`changes[${i}]: unexpected field(s) ${JSON.stringify(entryExtra)}`);
      }
      if (typeof entry.id !== 'string' || entry.id.length === 0) {
        errors.push(`changes[${i}].id must be a non-empty string`);
      }
      if (typeof entry.value !== 'string') {
        errors.push(`changes[${i}].value must be a string`);
      }
    });
  }
  const contributor = parseContributor(raw.contributor, errors);
  if (errors.length > 0) return { ok: false, errors };
  return {
    ok: true,
    value: {
      schema_version: BUNDLE_SCHEMA_VERSION,
      kind: BUNDLE_KIND,
      locale: locale as string,
      base_catalog_revision: base as string,
      changes: raw.changes as BundleChange[],
      ...(contributor !== undefined ? { contributor } : {}),
    },
  };
}
