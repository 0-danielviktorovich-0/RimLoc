// Self-localization E2E slice, contribution chain (SF-5). The chain input is
// NO LONGER a hand-written fixture mirrored on both sides — it is the REAL
// canonical session export produced by the Rust half of the slice
// (crates/rimloc-services/tests/ui_catalog_session.rs,
// `chain_export_writes_real_artifacts_for_the_ts_side`):
//
//   session create -> apply -> validate clean -> project_export
//     -> Languages/<locale>/Keyed/Translation.xml on disk
//     -> REPARSE of the written files -> chain.json {files, values}
//
// The known directory (RIMLOC_SELFLOC_CHAIN_DIR, default
// /tmp/rimloc-selfloc-chain) is the only junction between the two halves.
// THIS test consumes the artifact as-is:
//   1. the physical XML file is re-read and every chain.json value is matched
//      against its bytes — the manifest cannot silently diverge from disk;
//   2. previewPack() with a pack built from PART of the exported values: the
//      incomplete pack loads through the real schema, pack ids render from
//      the overlay, the remaining ids fall back to the built-in dictionaries;
//   3. buildContribution() from ALL exported values -> apply onto an
//      ISOLATED tmp copy of the dictionaries -> export:catalog regenerate ->
//      every changed message's `translated` equals the exported value;
//   4. the placeholder contract stays alive ACROSS the junction: at least one
//      exported value carries a {placeholder}, and both the pack loader and
//      the bundle gate accept it on those same bytes.
//
// Without the artifact the chain tests SKIP with an honest marker — no
// fixture silently substitutes for the missing real data. The negative gate
// evidence (a broken placeholder refused) stays fixture-driven: it tests the
// TS gate itself, not the chain.
import { execFileSync, execSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { i18n } from '../src/i18n/store.svelte';
import { buildContribution } from '../scripts/build-contribution';
import { applyToSource, planApply } from '../scripts/apply-contribution';
import { resolveCatalogRevision } from '../scripts/export-catalog';

const ROOT = join(__dirname, '..');
const TSX = join(ROOT, 'node_modules', '.bin', 'tsx');
const EN_TS = join(ROOT, 'src', 'i18n', 'en.ts');
const RU_TS = join(ROOT, 'src', 'i18n', 'ru.ts');
const REPO_EN_JSON = join(ROOT, 'src', 'i18n', 'generated', 'catalog.en.json');
const REPO_RU_JSON = join(ROOT, 'src', 'i18n', 'generated', 'catalog.ru.json');

// The junction: the Rust session test writes here, this test reads from here.
const CHAIN_DIR = process.env.RIMLOC_SELFLOC_CHAIN_DIR ?? '/tmp/rimloc-selfloc-chain';
const CHAIN_JSON = join(CHAIN_DIR, 'chain.json');
const CHAIN_SKIP_NOTE = 'chain artifact missing (run cargo test ui_catalog first)';

interface ChainFileEntry {
  path: string;
  locale: string;
  key_count: number;
}
interface ChainManifest {
  exported_at_run: number;
  locale: string;
  files: ChainFileEntry[];
  values: Record<string, string>;
}
const hasArtifact = existsSync(CHAIN_JSON);
const chain: ChainManifest | null = hasArtifact
  ? (JSON.parse(readFileSync(CHAIN_JSON, 'utf8')) as ChainManifest)
  : null;

/** The negative-gate input is deliberately bad and never enters a chain. */
interface FixtureChange {
  id: string;
  value: string;
}
interface SelflocFixture {
  locale: string;
  broken_first: FixtureChange;
}
const fx = JSON.parse(
  readFileSync(join(__dirname, 'fixtures', 'selfloc-e2e-translations.json'), 'utf8'),
) as SelflocFixture;

const PLACEHOLDER_RE = /\{(\w+)\}/g;

function placeholderNames(text: string): string[] {
  return [...new Set([...text.matchAll(PLACEHOLDER_RE)].map((m) => m[1]))].sort();
}

/** Mirror of the writer's escaping (crates/rimloc-services/src/project.rs). */
function escapeXmlText(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/**
 * Isolated tmp checkout: the two dictionaries + the real exporter, under git
 * so the bundle carries an exact base revision. Nothing in the repository is
 * touched.
 */
function makeIsolatedDictCopy(): string {
  const tmp = mkdtempSync(join(tmpdir(), 'rimloc-selfloc-chain-'));
  mkdirSync(join(tmp, 'src', 'i18n'), { recursive: true });
  mkdirSync(join(tmp, 'scripts'), { recursive: true });
  copyFileSync(EN_TS, join(tmp, 'src', 'i18n', 'en.ts'));
  copyFileSync(RU_TS, join(tmp, 'src', 'i18n', 'ru.ts'));
  copyFileSync(join(ROOT, 'scripts', 'export-catalog.ts'), join(tmp, 'scripts', 'export-catalog.ts'));
  copyFileSync(join(ROOT, 'package.json'), join(tmp, 'package.json'));
  execSync('git init -q', { cwd: tmp });
  execSync('git add package.json src scripts', { cwd: tmp });
  execSync(
    'git -c user.email=chain@example.com -c user.name=chain commit -qm "isolated: pristine dictionaries"',
    { cwd: tmp },
  );
  return tmp;
}

/** Guard narrowing `chain` to non-null, skipping honestly when absent. */
function requireChain(ctx: { skip: (condition: boolean, note?: string) => void }): ChainManifest {
  if (chain === null) {
    ctx.skip(true, CHAIN_SKIP_NOTE);
  }
  return chain as ChainManifest;
}

beforeEach(() => {
  i18n.setLocale('ru');
  i18n.clearPreview();
});

afterEach(() => {
  i18n.clearPreview();
});

describe('selfloc E2E: the real session export feeds the TS chain (SF-5)', () => {
  it(
    'the chain artifact is honest: manifest values match the written XML bytes, a placeholder survives',
    (ctx) => {
      const c = requireChain(ctx);
      const ids = Object.keys(c.values);
      expect(ids.length, 'the export wrote at least one key').toBeGreaterThan(0);
      expect(c.exported_at_run).toBeGreaterThan(0);
      expect(c.locale, 'the chain locale is a real target locale').not.toBe('en');
      expect(c.files.length).toBeGreaterThan(0);

      // key_count agrees with the values map (one manifest, one truth).
      for (const f of c.files) {
        expect(f.key_count, f.path).toBe(ids.length);
        const xmlPath = join(CHAIN_DIR, f.path);
        const xml = readFileSync(xmlPath, 'utf8');
        // Every exported value is physically present in the written file,
        // escaped exactly the way the canonical writer escapes it. THIS is
        // the byte-level proof that chain.json describes the real export.
        // SF-10 scaled the chain to the FULL catalog (1284 values, was 3),
        // and the real dictionary carries edge whitespace (e.g.
        // `stage.label: "Этап: "`): the writer preserves the value
        // verbatim, while chain.json values come from a REPARSE and the
        // Keyed scanner trims edge whitespace by contract. The comparison
        // therefore extracts the exact element and normalizes ONLY the
        // edges on both sides — every interior byte stays exact.
        for (const [id, value] of Object.entries(c.values)) {
          const open = `<${id}>`;
          const close = `</${id}>`;
          const start = xml.indexOf(open);
          expect(start, `${f.path} must contain <${id}>`).toBeGreaterThanOrEqual(0);
          const end = xml.indexOf(close, start);
          expect(end, `${f.path} must close <${id}>`).toBeGreaterThan(start);
          const inner = xml.slice(start + open.length, end);
          expect(inner.trim(), `${f.path} <${id}> value`).toBe(escapeXmlText(value).trim());
        }
      }

      // The placeholder contract must survive the junction: at least one
      // exported value interpolates, and its name set matches the en source
      // (the session validator guaranteed it on the Rust side — re-checked
      // here against the catalog the pack/bundle gates will use).
      const placeholderIds = ids.filter((id) => placeholderNames(c.values[id]).length > 0);
      expect(placeholderIds.length, 'chain carries a {placeholder} value').toBeGreaterThan(0);
      for (const id of placeholderIds) {
        expect(placeholderNames(c.values[id])).toEqual(placeholderNames(en[id]));
      }
    },
  );

  it('previewPack on a PARTIAL pack built from the real export values (fallback = built-ins)', (ctx) => {
    const c = requireChain(ctx);
    const allIds = Object.keys(c.values).sort();
    // Deterministic partial pack: every placeholder-bearing id first, then
    // the rest until ~half the exported set. The remainder MUST fall back.
    const half = Math.max(1, Math.ceil(allIds.length / 2));
    const phIds = allIds.filter((id) => placeholderNames(c.values[id]).length > 0);
    const packIds = [...new Set([...phIds, ...allIds])].slice(0, half);
    const fallbackIds = allIds.filter((id) => !packIds.includes(id));
    expect(packIds.length, 'the pack is genuinely partial').toBeLessThan(allIds.length);
    expect(fallbackIds.length, 'something must fall back').toBeGreaterThan(0);

    const pack = {
      schema_version: '1',
      locale: c.locale,
      base_catalog_revision: resolveCatalogRevision(),
      messages: packIds.map((id) => ({ id, value: c.values[id] })),
    };
    const load = i18n.previewPack(pack);
    expect(load.ok, JSON.stringify(load)).toBe(true);
    expect(i18n.preview).not.toBeNull();
    expect(i18n.preview!.count).toBe(packIds.length);
    expect(i18n.preview!.locale).toBe(c.locale);

    // Pack ids render from the real exported values through the real loader.
    for (const id of packIds) {
      expect(i18n.t(id), id).toBe(c.values[id]);
    }
    // The placeholder value interpolates live on exported bytes.
    for (const id of phIds) {
      const params = Object.fromEntries(placeholderNames(c.values[id]).map((n) => [n, '§']));
      expect(i18n.t(id, params), id).not.toContain('{');
    }
    // Non-pack ids fall back to the built-in dictionary — an incomplete pack
    // is a valid pack.
    for (const id of fallbackIds) {
      expect(i18n.t(id), id).toBe(ru[id]);
    }

    // Clearing the preview drops the overlay completely.
    i18n.clearPreview();
    for (const id of packIds) {
      expect(i18n.t(id), id).toBe(ru[id]);
    }
  });

  it(
    'buildContribution from ALL export values -> apply -> export:catalog equals the export',
    (ctx) => {
      const c = requireChain(ctx);
      const allIds = Object.keys(c.values).sort();
      const phIds = allIds.filter((id) => placeholderNames(c.values[id]).length > 0);
      const tmp = makeIsolatedDictCopy();
      try {
        const revision = execSync('git rev-parse HEAD', { cwd: tmp, encoding: 'utf8' }).trim();

        // The bundle is built from the exported values and nothing else.
        const built = buildContribution(
          { locale: c.locale, changes: allIds.map((id) => ({ id, value: c.values[id] })) },
          en,
          revision,
          ru,
        );
        expect(built.status).toBe('READY');
        expect(built.issues).toEqual([]);
        expect(built.bundle!.changes).toHaveLength(allIds.length);
        // The bundle gate accepted the placeholder-bearing exported values —
        // the contract is alive on real bytes, not fixture copies.
        for (const id of phIds) {
          const change = built.bundle!.changes.find((ch) => ch.id === id);
          expect(change, id).toBeDefined();
          expect(placeholderNames(change!.value)).toEqual(placeholderNames(en[id]));
        }

        // Apply onto the isolated dictionary copy (surgical line replacement).
        const plan = planApply(built.bundle!, en, ru);
        expect(plan.errors).toEqual([]);
        const ruPath = join(tmp, 'src', 'i18n', 'ru.ts');
        writeFileSync(ruPath, applyToSource(readFileSync(ruPath, 'utf8'), plan.entries), 'utf8');

        // Regenerate with the REAL exporter inside the isolated copy.
        execFileSync(TSX, ['scripts/export-catalog.ts'], { cwd: tmp, stdio: 'pipe' });

        // Generated values equal the REAL export values — the canonical
        // session output round-tripped through pack-free contribution path.
        const generatedRu = JSON.parse(
          readFileSync(join(tmp, 'src', 'i18n', 'generated', 'catalog.ru.json'), 'utf8'),
        ) as { messages: Array<{ id: string; translated: string }> };
        const repoRu = JSON.parse(readFileSync(REPO_RU_JSON, 'utf8')) as {
          messages: Array<{ id: string; translated: string }>;
        };
        const repoById = new Map(repoRu.messages.map((m) => [m.id, m.translated]));
        let changedCount = 0;
        for (const change of built.bundle!.changes) {
          const message = generatedRu.messages.find((m) => m.id === change.id);
          expect(message, change.id).toBeDefined();
          expect(message!.translated, change.id).toBe(c.values[change.id]);
          changedCount += repoById.get(change.id) !== c.values[change.id] ? 1 : 0;
        }
        // Exactly the exported ids moved; everything else is untouched.
        expect(changedCount).toBeGreaterThan(0);
        expect(generatedRu.messages).toHaveLength(repoRu.messages.length);
        for (const m of generatedRu.messages) {
          if (!built.bundle!.changes.some((ch) => ch.id === m.id)) {
            expect(m.translated, m.id).toBe(repoById.get(m.id));
          }
        }

        // The en catalog is untouched by the whole chain (byte-for-byte).
        expect(readFileSync(join(tmp, 'src', 'i18n', 'generated', 'catalog.en.json'), 'utf8')).toBe(
          readFileSync(REPO_EN_JSON, 'utf8'),
        );
        expect(readFileSync(join(tmp, 'src', 'i18n', 'en.ts'), 'utf8')).toBe(
          readFileSync(EN_TS, 'utf8'),
        );
      } finally {
        rmSync(tmp, { recursive: true, force: true });
      }
    },
    120_000,
  );
});

describe('selfloc E2E: the bundle gate refuses a broken placeholder (negative evidence)', () => {
  it('the broken fixture translation is refused before any bundle exists', () => {
    const built = buildContribution(
      { locale: fx.locale, changes: [fx.broken_first] },
      en,
      'a'.repeat(40),
      ru,
    );
    expect(built.status).toBe('NEEDS-FIXES');
    expect(built.bundle).toBeUndefined();
    expect(built.issues.map((i) => i.reason).join(' | ')).toContain('placeholder');
  });
});
