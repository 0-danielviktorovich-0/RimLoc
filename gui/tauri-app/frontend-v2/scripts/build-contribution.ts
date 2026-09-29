/**
 * Offline contribution bundle builder (owner mandate §5/§6/§11:
 * translation-only payload, validation gate BEFORE assembly, preview before
 * sending). docs/development/SELFLOC_BRIDGE.md, "Contribution Bundle".
 *
 * Input — a translator's change file for ONE non-source locale:
 *
 *   { "locale": "ru", "changes": [{ "id": "...", "value": "..." }] }
 *
 * Anything else in the payload (foreign root fields, foreign entry fields)
 * is dropped and reported; entries failing the §6 gate (id exists in the en
 * catalog, placeholder set matches the source contract, sane string value,
 * no secret patterns) are dropped with an exact reason.
 *
 * Readiness statuses:
 *   READY             every change valid — bundle emitted;
 *   PARTIAL-BUT-VALID some changes valid — bundle emitted from the valid
 *                     subset, rejected entries enumerated in the preview;
 *   NEEDS-FIXES       structural breakage or zero valid entries — NO bundle
 *                     file, exact blocker list on stdout (exit code 1).
 *
 * The emitted bundle contains ONLY schema fields
 * (scripts/contribution-schema.ts). The preview on stdout is the future
 * "preview before send" surface (§5); it is plain text, not UI.
 */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { en } from '../src/i18n/en';
import { extractPlaceholders, resolveCatalogRevision } from './export-catalog';
import {
  BUNDLE_KIND,
  BUNDLE_SCHEMA_VERSION,
  CHANGES_MAX_COUNT,
  ContributionBundle,
  BundleChange,
  CHANGE_ID_RE,
  CONTROL_CHARS_RE,
  CONTRIBUTOR_NAME_MAX_LEN,
  CONTRIBUTOR_NOTE_MAX_LEN,
  VALUE_MAX_LEN,
  isContributableLocale,
  isValidChangeId,
  scanSecrets,
  sortChangesById,
} from './contribution-schema';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url)); // scripts/
const ROOT = dirname(SCRIPT_DIR); // frontend-v2 package root
const DEFAULT_OUT_DIR = join(ROOT, 'dist', 'contribution');

export type BundleStatus = 'READY' | 'PARTIAL-BUT-VALID' | 'NEEDS-FIXES';

/** A precise, translator-actionable rejection reason. Never echoes secrets. */
export interface ValidationIssue {
  ref: string; // change id, or changes[i] / <root> when the id itself is unusable
  reason: string;
}

export interface SanitizeReport {
  changes: Array<{ id: string; value: string }>; // shape-correct entries, input order
  dropped: ValidationIssue[];
  foreignRootFields: string[];
  foreignEntryFields: string[];
  /** SF-4: non-fatal notes for the preview (equal duplicates collapsed). */
  warnings: string[];
  /** SF-4: the same id carried two different values — structural breakage. */
  conflictingDuplicate: boolean;
}

export interface BuildResult {
  status: BundleStatus;
  bundle?: ContributionBundle;
  issues: ValidationIssue[];
  preview: string;
}

/**
 * First sanitization pass: keep only well-shaped {id, value} string pairs,
 * record and drop everything else. Foreign fields never survive this pass.
 */
export function sanitizeInput(raw: unknown): SanitizeReport {
  const report: SanitizeReport = {
    changes: [],
    dropped: [],
    foreignRootFields: [],
    foreignEntryFields: [],
    warnings: [],
    conflictingDuplicate: false,
  };
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) {
    report.dropped.push({
      ref: '<root>',
      reason: 'input root must be a JSON object with locale + changes',
    });
    return report;
  }
  const root = raw as Record<string, unknown>;
  report.foreignRootFields = Object.keys(root).filter(
    (k) => k !== 'locale' && k !== 'changes' && k !== 'contributor',
  );
  if (!Array.isArray(root.changes)) {
    report.dropped.push({ ref: '<root>', reason: 'changes must be an array' });
    return report;
  }
  const seen = new Map<string, string>();
  root.changes.forEach((entry, i) => {
    const ref = `changes[${i}]`;
    if (typeof entry !== 'object' || entry === null || Array.isArray(entry)) {
      report.dropped.push({ ref, reason: 'entry must be an object with id + value' });
      return;
    }
    const e = entry as Record<string, unknown>;
    const extras = Object.keys(e).filter((k) => k !== 'id' && k !== 'value');
    if (extras.length > 0) report.foreignEntryFields.push(...extras);
    if (typeof e.id !== 'string' || e.id.length === 0) {
      report.dropped.push({ ref, reason: 'id must be a non-empty string' });
      return;
    }
    // SF-2: refuse ids that can never be safely addressed as dictionary keys
    // before any catalog lookup happens.
    if (!isValidChangeId(e.id)) {
      report.dropped.push({
        ref,
        reason: `id ${JSON.stringify(
          e.id,
        )} violates the id contract (bad_id): must match ${CHANGE_ID_RE.source} with no leading/trailing dots`,
      });
      return;
    }
    if (typeof e.value !== 'string') {
      report.dropped.push({ ref: e.id, reason: 'value must be a string' });
      return;
    }
    // SF-4: duplicates are decided explicitly — identical values collapse
    // into the first occurrence with a preview warning; conflicting values
    // are structural breakage (duplicate_id), never last-write-wins.
    if (seen.has(e.id)) {
      if (seen.get(e.id) === e.value) {
        report.warnings.push(
          `duplicate id ${JSON.stringify(e.id)} collapsed: identical values, first occurrence kept`,
        );
      } else {
        report.conflictingDuplicate = true;
        report.dropped.push({
          ref: e.id,
          reason:
            'duplicate_id: the same id carries conflicting values — rebuild the change file with one entry per id',
        });
      }
      return;
    }
    seen.set(e.id, e.value);
    report.changes.push({ id: e.id, value: e.value });
  });
  return report;
}

/**
 * §6 validation gate for one change, against the en catalog (id existence +
 * placeholder contract) and basic value sanity. Returns null when valid.
 */
export function validateChange(
  change: { id: string; value: string },
  enDict: Record<string, string>,
): ValidationIssue | null {
  const { id, value } = change;
  // SF-2: hasOwn only — `id in dict` / truthy dict[id] resolve inherited
  // names ('constructor', 'toString') and crash the placeholder check on a
  // function instead of refusing the id.
  if (!Object.hasOwn(enDict, id)) {
    return {
      ref: id,
      reason: 'id does not exist in the en catalog (new keys are not contributable)',
    };
  }
  if (value.length === 0) {
    return { ref: id, reason: 'value is empty (omit the change instead)' };
  }
  // SF-6: whitespace-only values are as good as empty — they render as
  // nothing visible, and the pack layer already rejects them (`empty_value`,
  // src/i18n/pack-schema.ts). The builder refuses them under the SAME reason
  // code so a READY bundle can never contradict the pack gate downstream.
  // Semantics: a value that is empty after trimming surrounding whitespace
  // (spaces, tabs, newlines) is "whitespace-only"; a value with any visible
  // character inside is fine.
  if (value.trim().length === 0) {
    return {
      ref: id,
      reason: 'value is whitespace-only (empty_value): it renders as nothing and the pack layer rejects it — omit the change instead',
    };
  }
  if (value.length > VALUE_MAX_LEN) {
    return { ref: id, reason: `value exceeds the ${VALUE_MAX_LEN}-character limit` };
  }
  if (CONTROL_CHARS_RE.test(value)) {
    return { ref: id, reason: 'value contains control characters' };
  }
  const expected = extractPlaceholders(enDict[id]);
  const actual = extractPlaceholders(value);
  if (JSON.stringify(expected) !== JSON.stringify(actual)) {
    return {
      ref: id,
      reason: `placeholder set mismatch: source requires ${JSON.stringify(expected)}, value has ${JSON.stringify(actual)}`,
    };
  }
  const secrets = scanSecrets(value);
  if (secrets.length > 0) {
    return {
      ref: id,
      reason: `value matches secret pattern(s) ${JSON.stringify(secrets)} — credentials never enter a bundle`,
    };
  }
  return null;
}

/**
 * Pure bundle assembly from already-validated changes: schema fields only,
 * changes sorted by id. `baseCatalogRevision` is injected so tests stay
 * deterministic; the CLI passes resolveCatalogRevision().
 */
export function buildBundle(
  locale: string,
  validChanges: BundleChange[],
  baseCatalogRevision: string,
  contributor?: { display_name?: string; note?: string },
): ContributionBundle {
  return {
    schema_version: BUNDLE_SCHEMA_VERSION,
    kind: BUNDLE_KIND,
    locale,
    base_catalog_revision: baseCatalogRevision,
    changes: sortChangesById(validChanges),
    ...(contributor !== undefined &&
    (contributor.display_name !== undefined || contributor.note !== undefined)
      ? { contributor }
      : {}),
  };
}

/** Collect contributor strings from a raw payload, shape-checked, no extras. */
function extractContributor(raw: unknown): { display_name?: string; note?: string } | undefined {
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return undefined;
  const c = raw as Record<string, unknown>;
  const out: { display_name?: string; note?: string } = {};
  if (typeof c.display_name === 'string') out.display_name = c.display_name;
  if (typeof c.note === 'string') out.note = c.note;
  return Object.keys(out).length > 0 ? out : undefined;
}

/**
 * SF-3 gate over contributor metadata: secrets there block the whole record
 * (`sensitive_contributor_meta` — stricter of the two contract options: the
 * note is never silently stripped, the translator rebuilds without it);
 * control characters and an oversized name are `bad_contributor_meta`.
 * Reasons carry pattern NAMES only, never the matched text.
 */
export function validateContributorMeta(
  contributor: { display_name?: string; note?: string } | undefined,
): ValidationIssue[] {
  if (contributor === undefined) return [];
  const issues: ValidationIssue[] = [];
  for (const key of ['display_name', 'note'] as const) {
    const value = contributor[key];
    if (value === undefined) continue;
    const max = key === 'display_name' ? CONTRIBUTOR_NAME_MAX_LEN : CONTRIBUTOR_NOTE_MAX_LEN;
    if (key === 'display_name' && CONTROL_CHARS_RE.test(value)) {
      issues.push({
        ref: `contributor.${key}`,
        reason: 'display_name contains control characters (bad_contributor_meta)',
      });
    }
    if (value.length > max) {
      issues.push({
        ref: `contributor.${key}`,
        reason: `exceeds the ${max}-character limit (bad_contributor_meta)`,
      });
      continue;
    }
    const hits = scanSecrets(value);
    if (hits.length > 0) {
      issues.push({
        ref: `contributor.${key}`,
        reason: `matches secret pattern(s) ${JSON.stringify(
          hits,
        )} (sensitive_contributor_meta) — credentials never enter a bundle, not even as metadata`,
      });
    }
  }
  return issues;
}

/**
 * Human-readable preview (§5 "preview before send", plain text): counts of
 * improvements vs identical values against the current locale dictionary,
 * the id list, sanitization notes, validation verdict, base revision.
 */
export function buildPreview(
  input: {
    locale: string;
    baseCatalogRevision: string;
    changes: BundleChange[];
    issues: ValidationIssue[];
    foreignRootFields: string[];
    foreignEntryFields: string[];
    warnings?: string[];
    currentDict?: Record<string, string>;
  },
  status: BundleStatus,
): string {
  const { locale, baseCatalogRevision, changes, issues, currentDict } = input;
  const lines: string[] = [];
  lines.push('contribution bundle preview');
  lines.push(`locale: ${locale}`);
  lines.push(`base catalog revision: ${baseCatalogRevision}`);
  lines.push(`changes: ${changes.length} valid`);
  if (issues.length > 0) {
    lines.push(`rejected: ${issues.length} (see below)`);
  }
  if (currentDict !== undefined) {
    const improved = changes.filter((c) => currentDict[c.id] !== c.value).length;
    const identical = changes.length - improved;
    lines.push(`vs current catalog: ${improved} improved, ${identical} identical (no-op)`);
  } else {
    lines.push('vs current catalog: all new (no existing dictionary for this locale)');
  }
  lines.push(`ids: ${changes.map((c) => c.id).join(', ') || '(none)'}`);
  if (input.foreignRootFields.length > 0) {
    lines.push(
      `sanitization: dropped foreign root field(s) ${JSON.stringify([...new Set(input.foreignRootFields)])}`,
    );
  }
  if (input.foreignEntryFields.length > 0) {
    lines.push(
      `sanitization: dropped foreign entry field(s) ${JSON.stringify([...new Set(input.foreignEntryFields)])}`,
    );
  }
  if (input.warnings !== undefined && input.warnings.length > 0) {
    lines.push(`warnings: ${input.warnings.length}`);
    for (const warning of input.warnings) lines.push(`  WARNING ${warning}`);
  }
  lines.push(
    `validation: ${
      status === 'READY'
        ? 'passed'
        : status === 'PARTIAL-BUT-VALID'
          ? 'passed for the valid subset only'
          : 'failed'
    }`,
  );
  lines.push(`status: ${status}`);
  for (const issue of issues) {
    lines.push(`  NEEDS FIX ${issue.ref}: ${issue.reason}`);
  }
  return lines.join('\n');
}

/**
 * Full pipeline: sanitize -> validate -> assemble -> preview. Never throws
 * on bad input; NEEDS-FIXES carries the exact issue list and no bundle.
 */
export function buildContribution(
  raw: unknown,
  enDict: Record<string, string>,
  baseCatalogRevision: string,
  currentDict?: Record<string, string>,
): BuildResult {
  const sanitize = sanitizeInput(raw);
  const root =
    typeof raw === 'object' && raw !== null && !Array.isArray(raw)
      ? (raw as Record<string, unknown>)
      : undefined;
  const rootLocale = root?.locale;

  const issues: ValidationIssue[] = [...sanitize.dropped];
  const localeOk = typeof rootLocale === 'string' && isContributableLocale(rootLocale);
  if (!localeOk) {
    issues.unshift({
      ref: '<root>',
      reason:
        typeof rootLocale === 'string'
          ? `locale ${JSON.stringify(rootLocale)} is not contributable (must match a locale tag and must not be the source locale "en")`
          : 'locale must be a string',
    });
  }

  const overLimit = sanitize.changes.length > CHANGES_MAX_COUNT;
  if (overLimit) {
    issues.push({ ref: '<root>', reason: `changes exceeds ${CHANGES_MAX_COUNT} entries` });
  }

  // SF-3: contributor metadata is part of the record — a secret (or a broken
  // name) in it blocks READY entirely; nothing is emitted to strip it from.
  const contributor = extractContributor(root?.contributor);
  const contributorIssues = validateContributorMeta(contributor);
  issues.push(...contributorIssues);

  const valid: BundleChange[] = [];
  if (!overLimit) {
    // SF-1: each change records the value the translator saw — the snapshot
    // the applier later compares the live dictionary against. A key absent
    // from the current dictionary (brand-new locale) snapshots ''.
    const baseOf = (id: string): string =>
      currentDict !== undefined && Object.hasOwn(currentDict, id) ? currentDict[id] : '';
    for (const change of sanitize.changes) {
      const issue = validateChange(change, enDict);
      if (issue) issues.push(issue);
      else valid.push({ id: change.id, value: change.value, base_value: baseOf(change.id) });
    }
  }

  if (
    !localeOk ||
    overLimit ||
    contributorIssues.length > 0 ||
    sanitize.conflictingDuplicate ||
    valid.length === 0
  ) {
    const preview = buildPreview(
      {
        locale: typeof rootLocale === 'string' ? rootLocale : '(invalid)',
        baseCatalogRevision,
        changes: [],
        issues,
        foreignRootFields: sanitize.foreignRootFields,
        foreignEntryFields: sanitize.foreignEntryFields,
        warnings: sanitize.warnings,
        currentDict,
      },
      'NEEDS-FIXES',
    );
    return { status: 'NEEDS-FIXES', issues, preview };
  }

  const bundle = buildBundle(rootLocale as string, valid, baseCatalogRevision, contributor);
  const status: BundleStatus = issues.length === 0 ? 'READY' : 'PARTIAL-BUT-VALID';
  const preview = buildPreview(
    {
      locale: bundle.locale,
      baseCatalogRevision,
      changes: bundle.changes,
      issues,
      foreignRootFields: sanitize.foreignRootFields,
      foreignEntryFields: sanitize.foreignEntryFields,
      warnings: sanitize.warnings,
      currentDict,
    },
    status,
  );
  return { status, bundle, issues, preview };
}

/** Best-effort load of the current dictionary for the locale (new locale = undefined). */
async function loadCurrentDict(locale: string): Promise<Record<string, string> | undefined> {
  try {
    const mod = (await import(
      pathToFileURL(join(SCRIPT_DIR, '..', 'src', 'i18n', `${locale}.ts`)).href
    )) as Record<string, unknown>;
    const dict = mod[locale];
    return typeof dict === 'object' && dict !== null ? (dict as Record<string, string>) : undefined;
  } catch {
    return undefined;
  }
}

async function main(): Promise<number> {
  const args = process.argv.slice(2);
  const outFlag = args.indexOf('--out');
  let outPath: string | undefined;
  if (outFlag !== -1) {
    outPath = args[outFlag + 1];
    args.splice(outFlag, 2);
  }
  const inputPath = args[0];
  if (!inputPath) {
    console.error('usage: npm run build:contribution -- <changed.json> [--out <bundle.json>]');
    return 1;
  }

  let raw: unknown;
  try {
    raw = JSON.parse(readFileSync(inputPath, 'utf8'));
  } catch (err) {
    console.error(`NEEDS-FIXES: cannot read/parse ${inputPath}: ${(err as Error).message}`);
    return 1;
  }

  const revision = resolveCatalogRevision();
  const rootLocale =
    typeof raw === 'object' && raw !== null && !Array.isArray(raw)
      ? (raw as Record<string, unknown>).locale
      : undefined;
  const current =
    typeof rootLocale === 'string' && isContributableLocale(rootLocale)
      ? await loadCurrentDict(rootLocale)
      : undefined;

  const result = buildContribution(raw, en, revision, current);

  console.log(result.preview);
  if (result.status === 'NEEDS-FIXES') {
    console.error('\nNEEDS-FIXES: no bundle written — fix the listed problems and rebuild.');
    return 1;
  }

  const out = outPath ?? join(DEFAULT_OUT_DIR, `${result.bundle!.locale}.translation-bundle.json`);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, `${JSON.stringify(result.bundle, null, 2)}\n`);

  console.log(`\nbundle written: ${out}`);
  console.log('next: review the preview, then hand the bundle file to the maintainer (offline).');
  return 0;
}

/** Run only when invoked as the entry script, not when imported by tests. */
const invokedDirectly =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  main().then((code) => {
    process.exitCode = code;
  });
}
