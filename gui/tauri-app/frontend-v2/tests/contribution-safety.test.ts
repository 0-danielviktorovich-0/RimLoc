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
import { buildContribution } from '../scripts/build-contribution';
import { planApply } from '../scripts/apply-contribution';
import { parseContributionBundle } from '../scripts/contribution-schema';

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
