/**
 * Deterministic return path: apply an APPROVED contribution bundle onto the
 * authoritative TS dictionaries (docs/development/SELFLOC_BRIDGE.md,
 * "Contribution Bundle"). The TS layer stays the ONE AUTHORITY — this script
 * edits generated *source* from reviewed data, never the other way around.
 *
 * Contract:
 *   - the bundle is parsed STRICTLY (schema fields exactly, scripts/contribution-schema.ts);
 *   - the source locale "en" is NEVER touched (parse refuses it outright);
 *   - only the dictionary of bundle.locale is edited, and only by REPLACING
 *     the value of an existing key in place; new keys are never created —
 *     every id must exist in en (catalog contract) AND in the locale file;
 *   - the §6 validation gate is re-run at apply time against the CURRENT en
 *     catalog (defense in depth against hand-edited bundles);
 *   - stale-source safety (§20): when the current catalog revision differs
 *     from base_catalog_revision, apply refuses and asks for a rebase;
 *     --allow-stale downgrades that to a warning;
 *   - value-conflict safety (SF-1): every change carries base_value (the
 *     dictionary value the translator saw); when the live value differs the
 *     run refuses with a conflict list — a manual edit is never silently
 *     overwritten, and --allow-stale does NOT bypass this gate;
 *   - --dry-run prints the diff plan and writes nothing;
 *   - application is all-or-nothing: any entry error refuses the whole run.
 *
 * The edit itself is a surgical line replacement of `'key': 'value',` in
 * src/i18n/<locale>.ts (single-quote/backslash escaping, non-ASCII kept as
 * UTF-8). After a real apply, `npm run export:catalog` must be re-run and
 * <locale>.ts committed together with src/i18n/generated/ — the drift-guard
 * test enforces exactly that.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { en } from '../src/i18n/en';
import { extractPlaceholders, resolveCatalogRevision } from './export-catalog';
import {
  ContributionBundle,
  escapeRegExp,
  escapeTsValue,
  normalizeRevision,
  parseContributionBundle,
} from './contribution-schema';
import { validateChange } from './build-contribution';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url)); // scripts/
const ROOT = dirname(SCRIPT_DIR); // frontend-v2 package root
const I18N_DIR = join(ROOT, 'src', 'i18n');

export interface PlanEntry {
  id: string;
  action: 'replace' | 'noop';
  from: string; // current decoded dictionary value
  to: string; // bundle value
}

export interface ApplyIssue {
  ref: string;
  reason: string;
}

export interface ApplyPlan {
  entries: PlanEntry[];
  errors: ApplyIssue[];
}

/** Decode a single-quoted TS literal body back into the plain string. */
export function unescapeTsLiteral(raw: string): string {
  return raw.replace(/\\([\s\S])/g, (_m, ch: string) => {
    switch (ch) {
      case 'n':
        return '\n';
      case 'r':
        return '\r';
      case 't':
        return '\t';
      case '\\':
      case "'":
        return ch;
      default:
        return `\\${ch}`; // unknown escape: keep verbatim
    }
  });
}

/**
 * Build the apply plan. Pure: takes dictionaries, touches no files.
 * Errors are accumulated and reported as a whole — the caller refuses the
 * entire application when errors is non-empty (fail-closed, no partials).
 */
export function planApply(
  bundle: ContributionBundle,
  enDict: Record<string, string>,
  localeDict: Record<string, string>,
): ApplyPlan {
  const entries: PlanEntry[] = [];
  const errors: ApplyIssue[] = [];
  for (const change of bundle.changes) {
    const gate = validateChange(change, enDict);
    if (gate) {
      errors.push({ ref: change.id, reason: gate.reason });
      continue;
    }
    // SF-1 value-conflict gate: the live dictionary value must still be what
    // the translator saw. A manual edit (or deletion) after the snapshot is a
    // conflict — the run refuses and nothing is written. --allow-stale only
    // downgrades the §20 revision gate, never this one.
    const current = Object.hasOwn(localeDict, change.id) ? localeDict[change.id] : '';
    if (current !== change.base_value) {
      errors.push({
        ref: change.id,
        reason: `value conflict (rebase_required): the dictionary changed since the translator saw it — current ${JSON.stringify(
          current,
        )} != base_value ${JSON.stringify(change.base_value)}; rebase the bundle (npm run build:contribution); --allow-stale does not bypass value conflicts`,
      });
      continue;
    }
    if (!Object.hasOwn(localeDict, change.id)) {
      errors.push({
        ref: change.id,
        reason:
          'id exists in en but is missing from the locale dictionary — key creation is out of contract (extend the dictionary manually first)',
      });
      continue;
    }
    const identical = localeDict[change.id] === change.value;
    entries.push({
      id: change.id,
      action: identical ? 'noop' : 'replace',
      from: localeDict[change.id],
      to: change.value,
    });
  }
  return { entries, errors };
}

/**
 * Surgical source edit: for every planned entry, replace exactly the one
 * dictionary line `'id': '…'` keeping indentation and trailing-comma state.
 * Throws when an id matches zero or several lines (planApply should have
 * caught a missing key; duplicates would mean a malformed dictionary).
 */
export function applyToSource(source: string, entries: PlanEntry[]): string {
  let out = source;
  for (const entry of entries) {
    if (entry.action === 'noop') continue;
    const re = new RegExp(
      `^([ \\t]*)'${escapeRegExp(entry.id)}'[ \\t]*:[ \\t]*'((?:[^'\\\\]|\\\\.)*)'(,)?[ \\t]*$`,
      'gm',
    );
    const matches = [...out.matchAll(re)];
    if (matches.length === 0) {
      throw new Error(`applyToSource: line for ${entry.id} not found`);
    }
    if (matches.length > 1) {
      throw new Error(`applyToSource: ${matches.length} lines match ${entry.id} — malformed dictionary`);
    }
    const m = matches[0];
    const [full, indent, , comma] = m;
    const replacement = `${indent}'${entry.id}': '${escapeTsValue(entry.to)}'${comma ?? ''}`;
    out = out.slice(0, m.index) + replacement + out.slice((m.index ?? 0) + full.length);
  }
  return out;
}

/**
 * Stale-source gate (§20): bundle base vs current catalog revision, ignoring
 * the incidental -dirty suffix. true = the catalog moved on since the bundle
 * was built.
 */
export function isStale(bundleBase: string, currentRevision: string): boolean {
  return normalizeRevision(bundleBase) !== normalizeRevision(currentRevision);
}

/** Load src/i18n/<locale>.ts as a dictionary (repository-owned path only). */
async function loadDict(locale: string): Promise<Record<string, string>> {
  const mod = (await import(pathToFileURL(join(I18N_DIR, `${locale}.ts`)).href)) as Record<
    string,
    unknown
  >;
  const dict = mod[locale];
  if (typeof dict !== 'object' || dict === null) {
    throw new Error(`src/i18n/${locale}.ts does not export a dictionary named ${locale}`);
  }
  return dict as Record<string, string>;
}

function formatPlan(bundle: ContributionBundle, plan: ApplyPlan, dryRun: boolean): string {
  const lines: string[] = [];
  lines.push(
    `apply contribution: ${dryRun ? 'DRY-RUN (no files changed)' : `writing src/i18n/${bundle.locale}.ts`}`,
  );
  lines.push(`bundle: locale ${bundle.locale} · base ${bundle.base_catalog_revision} · ${bundle.changes.length} change(s)`);
  for (const entry of plan.entries) {
    if (entry.action === 'noop') {
      lines.push(`  = ${entry.id}: value identical (no-op)`);
    } else {
      lines.push(`  ~ ${entry.id}: ${JSON.stringify(entry.from)} -> ${JSON.stringify(entry.to)}`);
    }
  }
  const replaces = plan.entries.filter((e) => e.action === 'replace').length;
  lines.push(`result: ${replaces} replacement(s), ${plan.entries.length - replaces} no-op(s)`);
  return lines.join('\n');
}

async function main(): Promise<number> {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  const allowStale = args.includes('--allow-stale');
  const bundlePath = args.find((a) => !a.startsWith('--'));
  if (!bundlePath) {
    console.error('usage: npm run apply:contribution -- <bundle.json> [--dry-run] [--allow-stale]');
    return 1;
  }

  // Gate 1: strict bundle parse. Refuses extras, wrong kind/version, the
  // source locale, malformed revisions — before anything else happens.
  let raw: unknown;
  try {
    raw = JSON.parse(readFileSync(bundlePath, 'utf8'));
  } catch (err) {
    console.error(`refused: cannot read/parse ${bundlePath}: ${(err as Error).message}`);
    return 1;
  }
  const parsed = parseContributionBundle(raw);
  if (!parsed.ok) {
    console.error(`refused: bundle violates the schema (${parsed.errors.length} problem(s)):`);
    for (const e of parsed.errors) console.error(`  - ${e}`);
    return 1;
  }
  const bundle = parsed.value;

  // Gate 2: the locale file must already exist — creating dictionaries is
  // out of contract for the return path.
  const localeFile = join(I18N_DIR, `${bundle.locale}.ts`);
  if (!existsSync(localeFile)) {
    console.error(
      `refused: src/i18n/${bundle.locale}.ts does not exist — creating new locale dictionaries is out of contract for apply`,
    );
    return 1;
  }

  // Gate 3: stale-source safety (§20).
  const currentRevision = resolveCatalogRevision();
  if (isStale(bundle.base_catalog_revision, currentRevision)) {
    if (!allowStale) {
      console.error(
        `STALE SOURCE: bundle base_catalog_revision ${bundle.base_catalog_revision} != current catalog revision ${currentRevision}.`,
      );
      console.error('The catalog moved on since this bundle was built (§20 stale-source safety).');
      console.error('Rebase the bundle against the current catalog (npm run build:contribution),');
      console.error('or re-run with --allow-stale to apply anyway (a warning is printed).');
      return 1;
    }
    console.warn(
      `warning: applying a STALE bundle (base ${bundle.base_catalog_revision} != current ${currentRevision}) — translations may no longer match updated source texts`,
    );
  }

  // Gate 4: re-run the §6 validation gate against the CURRENT en catalog and
  // require every key to already exist in the locale dictionary.
  const localeDict = await loadDict(bundle.locale);
  const plan = planApply(bundle, en, localeDict);
  if (plan.errors.length > 0) {
    console.error(`refused: ${plan.errors.length} entr(y/ies) fail the apply gate — nothing written:`);
    for (const e of plan.errors) console.error(`  - ${e.ref}: ${e.reason}`);
    return 1;
  }

  console.log(formatPlan(bundle, plan, dryRun));
  if (dryRun) {
    console.log(`\ndry-run complete — no files changed. Re-run without --dry-run to apply, then npm run export:catalog.`);
    return 0;
  }

  const source = readFileSync(localeFile, 'utf8');
  let updated: string;
  try {
    updated = applyToSource(source, plan.entries);
  } catch (err) {
    console.error(`refused: ${(err as Error).message} — nothing written`);
    return 1;
  }
  writeFileSync(localeFile, updated, 'utf8');

  const replaces = plan.entries.filter((e) => e.action === 'replace').length;
  console.log(`\napplied: ${replaces} replacement(s) -> ${localeFile}`);
  console.log(
    `next: npm run export:catalog, then commit ${bundle.locale}.ts and src/i18n/generated/ together (ONE AUTHORITY)`,
  );
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
