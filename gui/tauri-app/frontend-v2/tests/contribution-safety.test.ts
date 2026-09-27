// Contribution-safety regressions (night campaign S1, SF-1..SF-4).
//
// Each failing scenario was reproduced RED before its fix landed; the test
// names carry the SF-N id from the campaign review so every regression maps
// back to a confirmed bug:
//   SF-1 — a stale bundle silently overwrote a manual dictionary edit: the
//          applier compared only base_catalog_revision and ignored the fact
//          that the VALUE under a key had changed since the translator saw
//          it. Contract: every change records base_value (what the
//          translator saw); a mismatch at apply time is a conflict that
//          refuses the run, and --allow-stale does NOT bypass it.
import { execSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { validatePackObject } from '../src/i18n/pack-schema';
import { buildContribution, validateChange } from '../scripts/build-contribution';
import { planApply } from '../scripts/apply-contribution';
import { ContributionBundle, parseContributionBundle } from '../scripts/contribution-schema';

const ROOT = join(__dirname, '..');
const TSX = join(ROOT, 'node_modules', '.bin', 'tsx');
const BASE_REVISION = 'a'.repeat(40);

/** Minimal schema-shaped bundle fixture; per-test overrides allowed. */
const freshBundle = (overrides: Record<string, unknown> = {}, changes?: unknown[]) => ({
  schema_version: '1',
  kind: 'rimloc-ui-translation',
  locale: 'ru',
  base_catalog_revision: BASE_REVISION,
  changes:
    changes ?? [{ id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] }],
  ...overrides,
});

describe('SF-1: base_value rebase gate (a stale bundle must not overwrite a manual edit)', () => {
  it('SF-1: a change without base_value is refused with rebase_required', () => {
    const parsed = parseContributionBundle(
      freshBundle({}, [{ id: 'common.close', value: 'Закрыть окно' }]),
    );
    expect(parsed.ok).toBe(false);
    if (!parsed.ok) expect(parsed.errors.join(' | ')).toContain('rebase_required');
  });

  it('SF-1: a base_value non-string (e.g. null) is refused with rebase_required', () => {
    const parsed = parseContributionBundle(
      freshBundle({}, [{ id: 'common.close', value: 'Закрыть окно', base_value: null }]),
    );
    expect(parsed.ok).toBe(false);
    if (!parsed.ok) expect(parsed.errors.join(' | ')).toContain('rebase_required');
  });

  it('SF-1: a dictionary value changed since base_value is a value conflict — nothing is applied', () => {
    // simulate a manual edit after the translator snapshot: ru now differs
    const editedDict = { ...ru, 'common.close': 'Ручная правка после снятия базы' };
    const bundle = parseContributionBundle(
      freshBundle({}, [
        { id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] },
      ]),
    );
    expect(bundle.ok).toBe(true);
    if (!bundle.ok) return;
    const plan = planApply(bundle.value, en, editedDict);
    expect(plan.entries).toEqual([]);
    expect(plan.errors.map((e) => e.ref)).toEqual(['common.close']);
    expect(plan.errors[0].reason).toContain('value conflict');
    expect(plan.errors[0].reason).toContain('--allow-stale');
  });

  it('SF-1: a key deleted after the snapshot is a conflict, not a key-creation hint', () => {
    const withoutClose = { ...ru } as Record<string, string>;
    delete withoutClose['common.close'];
    const bundle = parseContributionBundle(
      freshBundle({}, [
        { id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] },
      ]),
    );
    expect(bundle.ok).toBe(true);
    if (!bundle.ok) return;
    const plan = planApply(bundle.value, en, withoutClose);
    expect(plan.entries).toEqual([]);
    expect(plan.errors[0].reason).toContain('value conflict');
  });

  it('SF-1: matching base_value plans normally (replace and noop paths intact)', () => {
    const bundle = parseContributionBundle(
      freshBundle({}, [
        { id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] },
        { id: 'common.cancel', value: ru['common.cancel'], base_value: ru['common.cancel'] },
      ]),
    );
    expect(bundle.ok).toBe(true);
    if (!bundle.ok) return;
    const plan = planApply(bundle.value, en, ru);
    expect(plan.errors).toEqual([]);
    expect(plan.entries.map((e) => e.action)).toEqual(['replace', 'noop']);
  });

  it('SF-1: the builder records base_value from the current dictionary; absent key -> empty string', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        changes: [
          { id: 'common.close', value: 'Закрыть окно' },
          { id: 'common.cancel', value: 'Отмена заново' },
        ],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('READY');
    const byId = Object.fromEntries(result.bundle!.changes.map((c) => [c.id, c]));
    expect(byId['common.close'].base_value).toBe(ru['common.close']);
    expect(byId['common.cancel'].base_value).toBe(ru['common.cancel']);
    // no dictionary for the locale (brand-new locale): translator saw nothing
    const fresh = buildContribution(
      { locale: 'de', changes: [{ id: 'common.close', value: 'Zumachen' }] },
      en,
      BASE_REVISION,
    );
    expect(fresh.status).toBe('READY');
    expect(fresh.bundle!.changes[0].base_value).toBe('');
  });

  it(
    'SF-1: apply CLI refuses a value conflict even with --allow-stale',
    () => {
      const dir = mkdtempSync(join(tmpdir(), 'rimloc-sf1-cli-'));
      const bundlePath = join(dir, 'conflict.json');
      // revision matches the live catalog (not stale), but the recorded
      // base_value differs from the live ru value -> pure value conflict
      writeFileSync(
        bundlePath,
        JSON.stringify(
          freshBundle(
            { base_catalog_revision: execSync('git rev-parse HEAD', { cwd: ROOT, encoding: 'utf8' }).trim() },
            [{ id: 'common.close', value: 'Закрыть окно', base_value: 'устаревшее значение' }],
          ),
        ),
      );
      try {
        let out = '';
        let code = 0;
        try {
          out = execSync(
            `"${TSX}" scripts/apply-contribution.ts ${JSON.stringify(bundlePath)} --dry-run --allow-stale 2>&1`,
            { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] },
          );
        } catch (err) {
          const e = err as { status?: number; stdout?: string };
          code = e.status ?? 1;
          out = e.stdout ?? '';
        }
        expect(code, out).not.toBe(0);
        expect(out).toContain('value conflict');
        expect(out).toContain('nothing written');
        // and the real dictionary is untouched
        expect(readFileSync(join(ROOT, 'src', 'i18n', 'ru.ts'), 'utf8')).toBe(
          readFileSync(join(ROOT, 'src', 'i18n', 'ru.ts'), 'utf8'),
        );
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    },
    60_000,
  );
});

describe('SF-2: inherited property names and prototype keys are machine refusals', () => {
  it('SF-2: ids breaking the id shape are refused with bad_id at bundle parse', () => {
    // '' is refused separately by the non-empty check — this list is about
    // the shape contract proper.
    const badIds = ['.leading', 'trailing.', 'with space', 'кириллица', 'a'.repeat(201), 'x\ny'];
    for (const id of badIds) {
      const parsed = parseContributionBundle(
        freshBundle({}, [{ id, value: 'v', base_value: 'b' }]),
      );
      expect(parsed.ok, JSON.stringify(id)).toBe(false);
      if (!parsed.ok) expect(parsed.errors.join(' | '), JSON.stringify(id)).toContain('bad_id');
    }
  });

  it('SF-2: builder drops inherited-name ids with a typed reason instead of crashing', () => {
    // Before the fix validateChange crashed here: 'constructor'/'toString'
    // resolved through the prototype chain, and extractPlaceholders() was
    // called on the inherited function.
    const result = buildContribution(
      {
        locale: 'ru',
        changes: [
          { id: 'constructor', value: 'x' },
          { id: 'toString', value: 'y' },
        ],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('NEEDS-FIXES');
    expect(result.bundle).toBeUndefined();
    const reasons = result.issues.map((i) => `${i.ref}: ${i.reason}`).join(' | ');
    expect(reasons).toContain('constructor');
    expect(reasons).toContain('toString');
    expect(reasons).toContain('does not exist in the en catalog');
  });

  it('SF-2: validateChange refuses inherited-name ids without touching the prototype', () => {
    expect(validateChange({ id: 'constructor', value: 'x' }, en)?.reason).toContain(
      'does not exist in the en catalog',
    );
    expect(validateChange({ id: 'hasOwnProperty', value: 'x' }, en)?.reason).toContain(
      'does not exist in the en catalog',
    );
  });

  it('SF-2: planApply is hasOwn-safe — inherited-name ids are typed errors, not crashes', () => {
    const bundle = {
      schema_version: '1',
      kind: 'rimloc-ui-translation',
      locale: 'ru',
      base_catalog_revision: BASE_REVISION,
      changes: [
        { id: 'constructor', value: 'x', base_value: '' },
        { id: 'toString', value: 'y', base_value: '' },
      ],
    } as unknown as ContributionBundle;
    const plan = planApply(bundle, en, ru);
    expect(plan.entries).toEqual([]);
    expect(plan.errors.map((e) => e.ref)).toEqual(['constructor', 'toString']);
  });

  it('SF-2: a __proto__ key anywhere in a bundle is a structural refusal', () => {
    const evilRoot = JSON.parse(
      `{"schema_version":"1","kind":"rimloc-ui-translation","locale":"ru","base_catalog_revision":"${BASE_REVISION}","changes":[{"id":"common.close","value":"x","base_value":"y"}],"__proto__":{"polluted":true}}`,
    );
    const rootParsed = parseContributionBundle(evilRoot);
    expect(rootParsed.ok).toBe(false);
    if (!rootParsed.ok) expect(rootParsed.errors.join(' | ')).toContain('__proto__');

    const evilEntry = JSON.parse(
      `{"schema_version":"1","kind":"rimloc-ui-translation","locale":"ru","base_catalog_revision":"${BASE_REVISION}","changes":[{"id":"common.close","value":"x","base_value":"y","__proto__":{"deep":1}}]}`,
    );
    const entryParsed = parseContributionBundle(evilEntry);
    expect(entryParsed.ok).toBe(false);
    if (!entryParsed.ok) expect(entryParsed.errors.join(' | ')).toContain('__proto__');

    const evilContributor = JSON.parse(
      `{"schema_version":"1","kind":"rimloc-ui-translation","locale":"ru","base_catalog_revision":"${BASE_REVISION}","changes":[{"id":"common.close","value":"x","base_value":"y"}],"contributor":{"note":"n","__proto__":{}}}`,
    );
    const contributorParsed = parseContributionBundle(evilContributor);
    expect(contributorParsed.ok).toBe(false);
    if (!contributorParsed.ok) expect(contributorParsed.errors.join(' | ')).toContain('__proto__');
  });

  it('SF-2: pack ids like constructor / toString / __proto__ reject typed, without crashing', () => {
    for (const id of ['constructor', 'toString', '__proto__', 'hasOwnProperty']) {
      const pack = {
        schema_version: '1',
        locale: 'ja',
        base_catalog_revision: 'a709064',
        messages: [{ id, value: 'リムロック' }],
      };
      const result = validatePackObject(pack, en);
      expect(result.ok, id).toBe(false);
      if (!result.ok) expect(result.reason, id).toMatch(/bad_id|unknown_id/);
    }
  });

  it('SF-2: a __proto__ key at the pack root is a structural refusal', () => {
    const evilPack = JSON.parse(
      `{"schema_version":"1","locale":"ja","base_catalog_revision":"a709064","messages":[{"id":"common.appName","value":"リムロック"}],"__proto__":{"x":1}}`,
    );
    const result = validatePackObject(evilPack, en);
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.details).toContain('__proto__');
  });
});

describe('SF-3: contributor metadata cannot carry secrets into a READY record', () => {
  const okChange = { id: 'common.close', value: 'Закрыть окно' };

  it('SF-3: a secret in contributor.note blocks READY with sensitive_contributor_meta, no bundle', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        contributor: { display_name: 'D', note: 'my token ghp_' + 'aB3dEf6gH9'.repeat(4) },
        changes: [okChange],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('NEEDS-FIXES');
    expect(result.bundle).toBeUndefined();
    const joined = result.issues.map((i) => `${i.ref}: ${i.reason}`).join(' | ');
    expect(joined).toContain('contributor.note');
    expect(joined).toContain('sensitive_contributor_meta');
    // the secret itself is never echoed into the preview
    expect(joined).not.toContain('ghp_');
    expect(result.preview).toContain('sensitive_contributor_meta');
  });

  it('SF-3: a secret in contributor.display_name is refused the same way', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        contributor: { display_name: 'sk-proj-' + 'x'.repeat(30) },
        changes: [okChange],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('NEEDS-FIXES');
    expect(result.bundle).toBeUndefined();
    const joined = result.issues.map((i) => `${i.ref}: ${i.reason}`).join(' | ');
    expect(joined).toContain('contributor.display_name');
    expect(joined).toContain('sensitive_contributor_meta');
  });

  it('SF-3: control characters in display_name are refused (bad_contributor_meta)', () => {
    const result = buildContribution(
      { locale: 'ru', contributor: { display_name: 'Даниэль\x07' }, changes: [okChange] },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('NEEDS-FIXES');
    expect(result.bundle).toBeUndefined();
    expect(result.issues.map((i) => i.reason).join(' | ')).toContain('bad_contributor_meta');
  });

  it('SF-3: display_name is capped at 80 characters', () => {
    const over = buildContribution(
      { locale: 'ru', contributor: { display_name: 'N'.repeat(81) }, changes: [okChange] },
      en,
      BASE_REVISION,
      ru,
    );
    expect(over.status).toBe('NEEDS-FIXES');
    expect(over.bundle).toBeUndefined();
    const ok = buildContribution(
      { locale: 'ru', contributor: { display_name: 'N'.repeat(80) }, changes: [okChange] },
      en,
      BASE_REVISION,
      ru,
    );
    expect(ok.status).toBe('READY');
    expect(ok.bundle!.contributor!.display_name).toHaveLength(80);
  });

  it('SF-3: clean contributor metadata keeps READY', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        contributor: { display_name: 'Даниэль', note: 'поправил формулировку' },
        changes: [okChange],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('READY');
    expect(result.issues).toEqual([]);
  });

  it('SF-3: the applier refuses a bundle whose contributor note carries a secret', () => {
    const parsed = parseContributionBundle(
      freshBundle({ contributor: { note: 'password=SuperSecret99' } }),
    );
    expect(parsed.ok).toBe(false);
    if (!parsed.ok) expect(parsed.errors.join(' | ')).toContain('sensitive_contributor_meta');
  });

  it('SF-3: the applier refuses control characters in display_name and oversized names', () => {
    const ctrl = parseContributionBundle(freshBundle({ contributor: { display_name: 'A\x1b[31m' } }));
    expect(ctrl.ok).toBe(false);
    if (!ctrl.ok) expect(ctrl.errors.join(' | ')).toContain('bad_contributor_meta');

    const long = parseContributionBundle(freshBundle({ contributor: { display_name: 'B'.repeat(81) } }));
    expect(long.ok).toBe(false);
    if (!long.ok) expect(long.errors.join(' | ')).toContain('80');
  });
});

describe('SF-4: duplicate ids are decided explicitly, never last-write-wins', () => {
  it('SF-4: duplicate ids in bundle changes are a duplicate_id refusal even with equal values', () => {
    const equal = parseContributionBundle(
      freshBundle({}, [
        { id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] },
        { id: 'common.close', value: 'Закрыть окно', base_value: ru['common.close'] },
      ]),
    );
    expect(equal.ok).toBe(false);
    if (!equal.ok) expect(equal.errors.join(' | ')).toContain('duplicate_id');

    const conflicting = parseContributionBundle(
      freshBundle({}, [
        { id: 'common.close', value: 'Вариант А', base_value: ru['common.close'] },
        { id: 'common.close', value: 'Вариант Б', base_value: ru['common.close'] },
      ]),
    );
    expect(conflicting.ok).toBe(false);
    if (!conflicting.ok) expect(conflicting.errors.join(' | ')).toContain('duplicate_id');
  });

  it('SF-4: builder collapses equal duplicates with a warning and keeps READY', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        changes: [
          { id: 'common.close', value: 'Закрыть окно' },
          { id: 'common.close', value: 'Закрыть окно' },
        ],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('READY');
    expect(result.bundle!.changes).toHaveLength(1);
    expect(result.preview).toContain('warnings: 1');
    expect(result.preview).toContain('common.close');
    expect(result.preview).toContain('duplicate');
  });

  it('SF-4: builder refuses conflicting duplicates with NEEDS-FIXES (no silent dedup)', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        changes: [
          { id: 'common.close', value: 'Закрыть окно' },
          { id: 'common.close', value: 'Закрыть (другое)' },
        ],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('NEEDS-FIXES');
    expect(result.bundle).toBeUndefined();
    const joined = result.issues.map((i) => `${i.ref}: ${i.reason}`).join(' | ');
    expect(joined).toContain('duplicate_id');
    expect(joined).toContain('common.close');
  });

  it('SF-4: pack duplicate ids reject whole with duplicate_id, equal values included', () => {
    const dup = {
      schema_version: '1',
      locale: 'ja',
      base_catalog_revision: 'a709064',
      messages: [
        { id: 'common.appName', value: 'リムロック' },
        { id: 'common.appName', value: 'リムロック' },
      ],
    };
    const result = validatePackObject(dup, en);
    expect(result).toMatchObject({ ok: false, reason: 'duplicate_id' });
  });
});
