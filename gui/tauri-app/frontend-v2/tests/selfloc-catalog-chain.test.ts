// Self-localization E2E slice, contribution chain (owner mandate §12; docs/
// development/SELFLOC_BRIDGE.md). This is the TS half of the vertical slice
// whose Rust half lives in crates/rimloc-services/src/ui_catalog.rs
// (selfloc_session_e2e): the SAME translation set (tests/fixtures/
// selfloc-e2e-translations.json is the single source for both) is exported
// as a contribution bundle, applied onto an ISOLATED tmp copy of the
// dictionaries, and regenerated through the real exporter.
//
// Proven here (do NOT duplicate elsewhere):
//   - pack preview + incomplete fallback: tests/i18n-pack.test.ts;
//   - bundle schema/apply guards: tests/contribution-bundle.test.ts and
//     tests/contribution-apply.test.ts (the round-trip pattern reused here).
//
// The specific §12 evidence of THIS test:
//   1. the session-form translations reconcile with the bundle form through
//      a small test-only bridge (no product code);
//   2. the bundle builds READY with correct placeholder sets from the
//      shared fixture (the same values the Rust session applied and
//      validated);
//   3. apply + `npm run export:catalog` regenerate generated/ so every
//      changed message's `translated` equals the fixture value;
//   4. the en catalog is byte-identical to the repository's, and the other
//      1172 messages are untouched (a surgical, all-or-nothing chain).
import { execFileSync, execSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { buildContribution } from '../scripts/build-contribution';
import { applyToSource, planApply } from '../scripts/apply-contribution';

const ROOT = join(__dirname, '..');
const TSX = join(ROOT, 'node_modules', '.bin', 'tsx');
const EN_TS = join(ROOT, 'src', 'i18n', 'en.ts');
const RU_TS = join(ROOT, 'src', 'i18n', 'ru.ts');
const REPO_EN_JSON = join(ROOT, 'src', 'i18n', 'generated', 'catalog.en.json');
const REPO_RU_JSON = join(ROOT, 'src', 'i18n', 'generated', 'catalog.ru.json');

interface FixtureChange {
  id: string;
  value: string;
}
interface SelflocFixture {
  locale: string;
  changes: FixtureChange[];
  broken_first: FixtureChange;
  fixed: FixtureChange;
}
const fx = JSON.parse(
  readFileSync(join(__dirname, 'fixtures', 'selfloc-e2e-translations.json'), 'utf8'),
) as SelflocFixture;

/**
 * Test-only bridge between the two forms of the E2E slice. The Rust session
 * stores a translation as (SourceEntryId { kind: Keyed, key }, locale, text);
 * the contribution bundle carries {id, value}. The catalog id IS the Keyed
 * key by construction (ui_catalog adapter), so the mapping is a field rename
 * — kept explicit here so a future drift between the two forms fails HERE,
 * visibly, instead of silently disconnecting the two halves.
 */
function sessionTranslationsToBundleChanges(
  translations: Array<{ key: string; locale: string; text: string }>,
): Array<{ id: string; value: string }> {
  return translations.map((t) => ({ id: t.key, value: t.text }));
}

describe('selfloc E2E: contribution chain on the shared fixture', () => {
  it(
    'session-form translations -> bundle -> apply -> export:catalog (en untouched)',
    () => {
      // The same set the Rust session test applies: `changes` + `fixed`.
      // `broken_first` never enters a bundle: the builder's placeholder gate
      // rejects it by contract (the session validator is what caught it on
      // the Rust side).
      const sessionForm = [
        ...fx.changes,
        fx.fixed,
      ].map((c) => ({ key: c.id, locale: fx.locale, text: c.value }));
      const bundleChanges = sessionTranslationsToBundleChanges(sessionForm);
      expect(bundleChanges).toHaveLength(3);

      // isolated tmp checkout: the two dictionaries + the real exporter
      const tmp = mkdtempSync(join(tmpdir(), 'rimloc-selfloc-chain-'));
      try {
        mkdirSync(join(tmp, 'src', 'i18n'), { recursive: true });
        mkdirSync(join(tmp, 'scripts'), { recursive: true });
        copyFileSync(EN_TS, join(tmp, 'src', 'i18n', 'en.ts'));
        copyFileSync(RU_TS, join(tmp, 'src', 'i18n', 'ru.ts'));
        copyFileSync(join(ROOT, 'scripts', 'export-catalog.ts'), join(tmp, 'scripts', 'export-catalog.ts'));
        copyFileSync(join(ROOT, 'package.json'), join(tmp, 'package.json'));
        execSync('git init -q', { cwd: tmp });
        execSync('git add package.json src scripts', { cwd: tmp });
        execSync(
          'git -c user.email=fixture@example.com -c user.name=fixture commit -qm "fixture: pristine dictionaries"',
          { cwd: tmp },
        );
        const revision = execSync('git rev-parse HEAD', { cwd: tmp, encoding: 'utf8' }).trim();

        // 1. build the bundle against the fixture revision
        const built = buildContribution(
          { locale: fx.locale, changes: bundleChanges },
          en,
          revision,
          ru,
        );
        expect(built.status).toBe('READY');
        expect(built.issues).toEqual([]);
        expect(built.bundle!.changes).toHaveLength(3);

        // 2. apply onto the tmp ru.ts (surgical line replacement)
        const plan = planApply(built.bundle!, en, ru);
        expect(plan.errors).toEqual([]);
        expect(plan.entries.map((e) => e.action)).toEqual(['replace', 'replace', 'replace']);
        const ruPath = join(tmp, 'src', 'i18n', 'ru.ts');
        writeFileSync(ruPath, applyToSource(readFileSync(ruPath, 'utf8'), plan.entries), 'utf8');

        // 3. regenerate the catalog with the REAL exporter inside the fixture
        execFileSync(TSX, ['scripts/export-catalog.ts'], { cwd: tmp, stdio: 'pipe' });

        // 4. generated values equal the session-applied (fixture) values
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
          expect(message!.translated, change.id).toBe(change.value);
          changedCount += repoById.get(change.id) !== change.value ? 1 : 0;
        }
        // exactly the three fixture ids moved; everything else is untouched
        expect(changedCount).toBe(3);
        expect(generatedRu.messages).toHaveLength(repoRu.messages.length);
        for (const m of generatedRu.messages) {
          if (!built.bundle!.changes.some((c) => c.id === m.id)) {
            expect(m.translated, m.id).toBe(repoById.get(m.id));
          }
        }

        // 5. the en catalog is untouched by the whole chain (byte-for-byte)
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

  it('the broken placeholder translation is refused by the bundle gate (negative evidence)', () => {
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
