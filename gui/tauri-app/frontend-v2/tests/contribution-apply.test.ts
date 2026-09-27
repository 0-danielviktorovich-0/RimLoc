// Contribution return-path guards (owner mandate §5/§20; docs/development/
// SELFLOC_BRIDGE.md, "Contribution Bundle").
//
// Covers:
//   1. strict bundle parsing — extra fields anywhere, wrong kind/version,
//      source locale and malformed revisions are refused;
//   2. apply planning — replace vs no-op, refusal of unknown ids and of key
//      creation (id exists in en but not in the locale dictionary);
//   3. the surgical TS edit — exactly the target lines change, indentation
//      and trailing-comma state preserved, values escaped correctly and
//      decodable back (verified by an independent tsx evaluation);
//   4. stale-source gate (§20) — revision mismatch refuses, -dirty suffix
//      does not, --allow-stale downgrades to a warning;
//   5. round-trip proof in an ISOLATED tmp copy of the dictionaries:
//      build bundle -> apply -> regenerate catalog via the real exporter ->
//      generated JSON values equal the bundle (drift between the two
//      sources is excluded constructively).
import { execFileSync, execSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { buildContribution } from '../scripts/build-contribution';
import {
  applyToSource,
  isStale,
  planApply,
  unescapeTsLiteral,
} from '../scripts/apply-contribution';
import { parseContributionBundle } from '../scripts/contribution-schema';

const ROOT = join(__dirname, '..');
const TSX = join(ROOT, 'node_modules', '.bin', 'tsx');
const EN_TS = join(ROOT, 'src', 'i18n', 'en.ts');
const RU_TS = join(ROOT, 'src', 'i18n', 'ru.ts');
const REPO_EN_JSON = join(ROOT, 'src', 'i18n', 'generated', 'catalog.en.json');
const BASE_REVISION = 'b'.repeat(40);

const freshBundle = (overrides: Record<string, unknown> = {}) => ({
  schema_version: '1',
  kind: 'rimloc-ui-translation',
  locale: 'ru',
  base_catalog_revision: BASE_REVISION,
  // base_value (SF-1) = the live ru value: the fixture matches the real
  // dictionary, so only the intended gate fires in each scenario.
  changes: [{ id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] }],
  ...overrides,
});

describe('apply: strict bundle parsing', () => {
  it('accepts a schema-clean bundle', () => {
    const parsed = parseContributionBundle(freshBundle());
    expect(parsed.ok).toBe(true);
    if (parsed.ok) expect(parsed.value.locale).toBe('ru');
  });

  it('refuses foreign fields at the root and inside changes', () => {
    const dirty = freshBundle({ exec: 'curl evil.sh', changes: [{ id: 'common.close', value: 'x', nested: {} }] });
    const parsed = parseContributionBundle(dirty);
    expect(parsed.ok).toBe(false);
    if (!parsed.ok) {
      expect(parsed.errors.join('\n')).toContain('"exec"');
      expect(parsed.errors.join('\n')).toContain('"nested"');
    }
  });

  it('refuses the source locale, wrong kind/version and malformed revisions', () => {
    const cases: Array<[Record<string, unknown>, string]> = [
      [freshBundle({ locale: 'en' }), 'source locale'],
      [freshBundle({ kind: 'something-else' }), 'kind must be'],
      [freshBundle({ schema_version: '2' }), 'schema_version must be'],
      [freshBundle({ base_catalog_revision: 'not-a-sha' }), 'git sha'],
      [freshBundle({ changes: [] }), 'must not be empty'],
    ];
    for (const [bundle, needle] of cases) {
      const parsed = parseContributionBundle(bundle);
      expect(parsed.ok, JSON.stringify(needle)).toBe(false);
      if (!parsed.ok) expect(parsed.errors.join(' | ')).toContain(needle);
    }
  });
});

describe('apply: planning', () => {
  it('plans replacements and no-ops, refuses unknown ids and key creation', () => {
    const bundle = parseContributionBundle(
      freshBundle({
        changes: [
          // base_value (SF-1) = what the translator saw: the live ru value
          // for replace/noop rows; '' for rows whose key is absent here.
          { id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] }, // replace
          { id: 'common.cancel', value: 'Отмена', base_value: ru['common.cancel'] }, // identical -> noop
          { id: 'ghost.key', value: 'нет в en', base_value: '' }, // unknown id
          { id: 'common.details', value: 'Только в en, словарь ru не знает', base_value: '' }, // (ru knows it; see dedicated case below)
        ],
      }),
    );
    expect(bundle.ok).toBe(true);
    if (!bundle.ok) return;
    const partialEn = { ...en, 'common.details': en['common.details'] };
    const localeWithoutDetails = { ...ru } as Record<string, string>;
    delete localeWithoutDetails['common.details'];
    const plan = planApply(bundle.value, partialEn, localeWithoutDetails);

    const actions = Object.fromEntries(plan.entries.map((e) => [e.id, e.action]));
    expect(actions['common.close']).toBe('replace');
    expect(actions['common.cancel']).toBe('noop');
    expect(plan.errors.map((e) => e.ref)).toEqual(
      expect.arrayContaining(['ghost.key', 'common.details']),
    );
    expect(plan.errors.find((e) => e.ref === 'common.details')?.reason).toContain(
      'key creation is out of contract',
    );
  });

  it('is a no-op planner against the real dictionaries when nothing differs', () => {
    const bundle = parseContributionBundle(
      freshBundle({
        changes: [{ id: 'common.appName', value: ru['common.appName'], base_value: ru['common.appName'] }],
      }),
    );
    expect(bundle.ok).toBe(true);
    if (!bundle.ok) return;
    const plan = planApply(bundle.value, en, ru);
    expect(plan.errors).toEqual([]);
    expect(plan.entries[0].action).toBe('noop');
  });
});

describe('apply: surgical TS edit and escaping', () => {
  it('changes exactly the target lines, preserving indent and comma state', () => {
    const source = readFileSync(RU_TS, 'utf8');
    const updated = applyToSource(source, [
      { id: 'common.close', action: 'replace', from: ru['common.close'], to: 'Закрыть окно' },
      { id: 'common.appName', action: 'noop', from: ru['common.appName'], to: ru['common.appName'] },
    ]);
    const before = source.split('\n');
    const after = updated.split('\n');
    expect(after.length).toBe(before.length);
    const changedIdx = before.map((line, i) => (line !== after[i] ? i : -1)).filter((i) => i >= 0);
    expect(changedIdx.length).toBe(1); // exactly one line: common.close (the noop edits nothing)
    expect(before[changedIdx[0]]).toContain("'common.close'");
    expect(after[changedIdx[0]]).toBe("  'common.close': 'Закрыть окно',");
    expect(updated.trimEnd().endsWith('};')).toBe(true);
    expect(updated).toContain("'shortcuts.def.sourceCompare': 'Источник: сравнить с выводом'\n};");
    // noop did not perturb the untouched line
    expect(updated).toContain("'common.appName': 'RimLoc',");
  });

  it('escapes quotes, backslashes and newlines; the file still evaluates to the raw value', () => {
    const nasty = "Апостроф ' и слэш \\ и перенос\nстроки и\tтаб: Перевод {locale} — 2026";
    const source = readFileSync(RU_TS, 'utf8');
    const updated = applyToSource(source, [
      { id: 'common.close', action: 'replace', from: ru['common.close'], to: nasty },
    ]);
    expect(updated).toContain(
      "'common.close': 'Апостроф \\' и слэш \\\\ и перенос\\nстроки и\\tтаб: Перевод {locale} — 2026',",
    );
    // independent evaluation: tsx parses the edited file, the decoded value is the raw one
    const tmp = mkdtempSync(join(tmpdir(), 'rimloc-contrib-escape-'));
    try {
      const file = join(tmp, 'ru.ts');
      writeFileSync(file, updated, 'utf8');
      const out = execFileSync(
        TSX,
        [
          '-e',
          `import { ru } from ${JSON.stringify(file)}; console.log(JSON.stringify(ru['common.close']));`,
        ],
        { cwd: ROOT, encoding: 'utf8' },
      );
      expect(JSON.parse(out)).toBe(nasty);
    } finally {
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it('unescapeTsLiteral round-trips the escape set', () => {
    expect(unescapeTsLiteral("a\\'b\\\\c\\nd\\te\\rf")).toBe("a'b\\c\nd\te\rf");
  });
});

describe('apply: stale-source gate (§20)', () => {
  it('revision mismatch is stale; the -dirty suffix never is', () => {
    const sha = 'c'.repeat(40);
    expect(isStale(sha, sha)).toBe(false);
    expect(isStale(`${sha}-dirty`, sha)).toBe(false);
    expect(isStale(sha, `${sha}-dirty`)).toBe(false);
    expect(isStale('a'.repeat(40), 'b'.repeat(40))).toBe(true);
  });

  it('a stale bundle is refused with rebase guidance; --allow-stale warns instead', () => {
    const staleBundlePath = mkdtempSync(join(tmpdir(), 'rimloc-contrib-stale-')) + '/stale.json';
    writeFileSync(staleBundlePath, JSON.stringify(freshBundle({ base_catalog_revision: 'd'.repeat(40) })));
    try {
      const refuse = runApply(staleBundlePath, ['--dry-run']);
      expect(refuse.code).not.toBe(0);
      expect(refuse.out).toContain('STALE SOURCE');
      expect(refuse.out).toContain('Rebase the bundle');

      const allowed = runApply(staleBundlePath, ['--dry-run', '--allow-stale']);
      expect(allowed.code).toBe(0);
      expect(allowed.out).toContain('warning: applying a STALE bundle');
    } finally {
      rmSync(staleBundlePath, { force: true });
    }
  });
});

/** Run the apply CLI in a subprocess; returns merged output + exit code. */
function runApply(bundlePath: string, flags: string[]): { code: number; out: string } {
  const cmd = `"${TSX}" scripts/apply-contribution.ts ${JSON.stringify(bundlePath)} ${flags.join(' ')} 2>&1`;
  try {
    const out = execSync(cmd, { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
    return { code: 0, out };
  } catch (err) {
    const e = err as { status?: number; stdout?: string };
    return { code: e.status ?? 1, out: e.stdout ?? '' };
  }
}

describe('apply: round-trip in an isolated tmp copy (bundle -> apply -> export)', () => {
  it(
    'generated catalog values equal the bundle; en catalog untouched byte-for-byte',
    () => {
      const tmp = mkdtempSync(join(tmpdir(), 'rimloc-contrib-roundtrip-'));
      try {
        // fixture: standalone copy of the two dictionaries + the exporter
        mkdirSync(join(tmp, 'src', 'i18n'), { recursive: true });
        mkdirSync(join(tmp, 'scripts'), { recursive: true });
        copyFileSync(EN_TS, join(tmp, 'src', 'i18n', 'en.ts'));
        copyFileSync(RU_TS, join(tmp, 'src', 'i18n', 'ru.ts'));
        copyFileSync(
          join(ROOT, 'scripts', 'export-catalog.ts'),
          join(tmp, 'scripts', 'export-catalog.ts'),
        );
        copyFileSync(join(ROOT, 'package.json'), join(tmp, 'package.json'));
        // throwaway fixture repo (not the workspace): explicit paths, local identity
        execSync('git init -q', { cwd: tmp });
        execSync('git add package.json src scripts', { cwd: tmp });
        execSync(
          'git -c user.email=fixture@example.com -c user.name=fixture commit -qm "fixture: pristine dictionaries"',
          { cwd: tmp },
        );
        const revision = execSync('git rev-parse HEAD', { cwd: tmp, encoding: 'utf8' }).trim();

        // 1. build the bundle against the fixture revision
        const input = {
          locale: 'ru',
          contributor: { display_name: 'Round-trip' },
          changes: [
            { id: 'common.close', value: 'Закрыть (rt)' },
            {
              id: 'contractops.export.desc',
              value: 'Вывод изолирован в выбранную вами папку. Папка языка: {locale}.',
            },
          ],
        };
        const built = buildContribution(input, en, revision, ru);
        expect(built.status).toBe('READY');
        const bundle = built.bundle!;

        // 2. apply onto the tmp ru.ts (surgical edit, nothing else)
        const ruPath = join(tmp, 'src', 'i18n', 'ru.ts');
        const source = readFileSync(ruPath, 'utf8');
        const plan = planApply(bundle, en, ru);
        expect(plan.errors).toEqual([]);
        writeFileSync(ruPath, applyToSource(source, plan.entries), 'utf8');

        // 3. regenerate the catalog with the REAL exporter inside the fixture
        execFileSync(TSX, ['scripts/export-catalog.ts'], { cwd: tmp, stdio: 'pipe' });

        // 4. generated JSON values equal the bundle values
        const generatedRu = JSON.parse(
          readFileSync(join(tmp, 'src', 'i18n', 'generated', 'catalog.ru.json'), 'utf8'),
        ) as { messages: Array<{ id: string; translated: string }> };
        for (const change of bundle.changes) {
          const message = generatedRu.messages.find((m) => m.id === change.id);
          expect(message, change.id).toBeDefined();
          expect(message!.translated, change.id).toBe(change.value);
        }

        // 5. the en catalog is untouched by the whole cycle (apply never edits en)
        expect(readFileSync(join(tmp, 'src', 'i18n', 'generated', 'catalog.en.json'), 'utf8')).toBe(
          readFileSync(REPO_EN_JSON, 'utf8'),
        );

        // 6. determinism: exporting twice is byte-stable (modulo the -dirty flag
        //    that the second run gains from the already-modified ru.ts)
        const firstRu = readFileSync(join(tmp, 'src', 'i18n', 'generated', 'catalog.ru.json'), 'utf8');
        execFileSync(TSX, ['scripts/export-catalog.ts'], { cwd: tmp, stdio: 'pipe' });
        expect(readFileSync(join(tmp, 'src', 'i18n', 'generated', 'catalog.ru.json'), 'utf8')).toBe(firstRu);
      } finally {
        rmSync(tmp, { recursive: true, force: true });
      }
    },
    120_000,
  );

  it('a full real-dictionary cycle through build + plan produces only sanctioned edits', () => {
    const input = {
      locale: 'ru',
      changes: [{ id: 'common.close', value: 'Закрыть (cycle)' }],
    };
    const built = buildContribution(input, en, BASE_REVISION, ru);
    expect(built.status).toBe('READY');
    const plan = planApply(built.bundle!, en, ru);
    expect(plan.errors).toEqual([]);
    expect(plan.entries).toHaveLength(1);
    expect(plan.entries[0]).toMatchObject({ id: 'common.close', action: 'replace', to: 'Закрыть (cycle)' });
  });
});
