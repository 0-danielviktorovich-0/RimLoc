// Contribution bundle builder guards (owner mandate §5/§6/§11/§21;
// docs/development/SELFLOC_BRIDGE.md, "Contribution Bundle").
//
// Covers:
//   1. schema purity — the emitted bundle carries ONLY schema fields, and
//      change entries are exactly {id, value};
//   2. sanitization — a bundle built from dirty input (paths, key-like
//      strings, oversized values, foreign fields, credential-shaped values)
//      contains only schema fields, drops foreign fields, and passes the
//      secret scan with zero hits;
//   3. readiness statuses — READY / PARTIAL-BUT-VALID / NEEDS-FIXES with
//      exact, actionable reasons and no bundle file on NEEDS-FIXES;
//   4. the §6 validation gate — id existence in en, placeholder-set
//      contract, value sanity (empty / limit / control chars).
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import {
  buildBundle,
  buildContribution,
  buildPreview,
  sanitizeInput,
  validateChange,
} from '../scripts/build-contribution';
import { scanSecrets } from '../scripts/contribution-schema';

const ROOT = join(__dirname, '..');
const BASE_REVISION = 'a'.repeat(40); // injected: keeps tests deterministic

const validChange = { id: 'common.close', value: 'Закрыть окно' };

describe('contribution bundle: schema purity', () => {
  it('bundle carries exactly the schema fields, changes exactly {id, value}', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        contributor: { display_name: 'Daniel', note: 'fixes' },
        changes: [validChange, { id: 'common.retry', value: 'Повторить снова' }],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('READY');
    const bundle = result.bundle!;
    expect(Object.keys(bundle).sort()).toEqual(
      ['base_catalog_revision', 'changes', 'contributor', 'kind', 'locale', 'schema_version'].sort(),
    );
    expect(bundle.schema_version).toBe('1');
    expect(bundle.kind).toBe('rimloc-ui-translation');
    expect(bundle.base_catalog_revision).toBe(BASE_REVISION);
    for (const change of bundle.changes) {
      // SF-1: every emitted change records the value the translator saw.
      expect(Object.keys(change).sort()).toEqual(['base_value', 'id', 'value']);
      expect(change.base_value).toBe(ru[change.id]);
    }
    // canonical order: sorted by id, not input order
    expect(bundle.changes.map((c) => c.id)).toEqual([...bundle.changes.map((c) => c.id)].sort());
  });

  it('contributor omitted from the bundle when empty', () => {
    const bundle = buildBundle(
      'ru',
      [{ ...validChange, base_value: ru['common.close'] }],
      BASE_REVISION,
    );
    expect('contributor' in bundle).toBe(false);
  });
});

describe('contribution bundle: sanitization (§11/§21)', () => {
  it('garbage input yields a schema-only bundle with zero secret hits', () => {
    const garbage = {
      locale: 'ru',
      apiKey: 'AKIAIOSFODNN7EXAMPLE',
      token: 'xoxb-123456789-abcdefghijklmnop',
      __proto_note: 'not really dangerous but foreign',
      changes: [
        { id: 'common.close', value: 'Закрыть (путь примера: /etc/passwd не часть перевода)' },
        { id: 'common.retry', value: 'retry.common.retry-like string as a VALUE is fine', smuggled: 'field' },
        { id: 'common.cancel', value: 'Отмена' },
        { id: 'common.back', value: 'leak=github_pat_' + 'A'.repeat(80) },
        { id: 'common.next', value: 'Д'.repeat(1001) },
        { id: 'common.details', value: 42 },
        'not-an-object',
        { id: 'common.reset', value: 'Сбросить настройки' },
      ],
    };
    const result = buildContribution(garbage, en, BASE_REVISION, ru);
    expect(result.status).toBe('PARTIAL-BUT-VALID');
    const serialized = JSON.stringify(result.bundle);

    // only schema fields survive — foreign root/entry fields never leak
    expect(serialized).not.toContain('apiKey');
    expect(serialized).not.toContain('AKIA');
    expect(serialized).not.toContain('xoxb');
    expect(serialized).not.toContain('smuggled');
    expect(serialized).not.toContain('__proto_note');
    expect(serialized).not.toContain('github_pat_');
    expect(result.bundle!.changes.find((c) => c.id === 'common.next')).toBeUndefined();
    expect(result.bundle!.changes.find((c) => c.id === 'common.back')).toBeUndefined();
    expect(result.issues.map((i) => i.ref)).toEqual(
      expect.arrayContaining(['common.next', 'common.back', 'common.details', 'changes[6]']),
    );

    // path-like and key-like VALUES are plain text and legitimately survive…
    expect(result.bundle!.changes.find((c) => c.id === 'common.close')!.value).toContain('/etc/passwd');
    expect(result.bundle!.changes.find((c) => c.id === 'common.retry')!.value).toContain(
      'retry.common.retry-like',
    );
    // …but the assembled bundle must be clean of credential signatures
    expect(scanSecrets(serialized)).toEqual([]);
  });

  it('preview reports dropped foreign fields explicitly', () => {
    const report = sanitizeInput({ locale: 'ru', injected: 1, changes: [{ id: 'common.reset', value: 'x', extra: 2 }] });
    expect(report.foreignRootFields).toEqual(['injected']);
    expect(report.foreignEntryFields).toEqual(['extra']);
  });
});

describe('contribution bundle: readiness statuses', () => {
  it('READY when every change passes the gate', () => {
    const result = buildContribution({ locale: 'ru', changes: [validChange] }, en, BASE_REVISION, ru);
    expect(result.status).toBe('READY');
    expect(result.issues).toEqual([]);
    expect(result.bundle!.changes).toHaveLength(1);
  });

  it('PARTIAL-BUT-VALID: bundle holds only the valid subset, rejects enumerated', () => {
    const result = buildContribution(
      {
        locale: 'ru',
        changes: [
          validChange,
          { id: 'does.not.exist', value: 'фантом' },
          { id: 'common.details', value: 'Подробности тут' },
        ],
      },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('PARTIAL-BUT-VALID');
    expect(result.bundle!.changes.map((c) => c.id).sort()).toEqual(['common.close', 'common.details']);
    const unknown = result.issues.find((i) => i.ref === 'does.not.exist');
    expect(unknown?.reason).toContain('id does not exist in the en catalog');
  });

  it('NEEDS-FIXES when zero valid entries: no bundle, exact reasons', () => {
    const result = buildContribution(
      { locale: 'ru', changes: [{ id: 'nope.nope', value: 'x' }, { id: 'common.close', value: 'Лишний {token}' }] },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('NEEDS-FIXES');
    expect(result.bundle).toBeUndefined();
    expect(result.issues).toHaveLength(2);
    expect(result.issues[0].ref).toBe('nope.nope');
    expect(result.issues[1].reason).toContain('placeholder set mismatch');
    expect(result.preview).toContain('validation: failed');
  });

  it('NEEDS-FIXES on structural breakage: bad root, bad locale, missing changes', () => {
    expect(buildContribution('a string', en, BASE_REVISION).status).toBe('NEEDS-FIXES');
    expect(buildContribution({ locale: 'de', changes: [{ id: 'common.close', value: 'Zumachen' }] }, en, BASE_REVISION).status).toBe(
      'READY', // a new (non-source) locale is legitimate: all its values are "new"
    );
    const noChanges = buildContribution({ locale: 'ru' }, en, BASE_REVISION);
    expect(noChanges.status).toBe('NEEDS-FIXES');
    const enLocale = buildContribution({ locale: 'en', changes: [{ id: 'common.close', value: 'Close' }] }, en, BASE_REVISION);
    expect(enLocale.status).toBe('NEEDS-FIXES');
    expect(enLocale.issues[0].reason).toContain('not contributable');
  });

  it('duplicate ids (SF-4): equal values collapse with a warning, conflicting values break', () => {
    const equal = sanitizeInput({
      locale: 'ru',
      changes: [validChange, { ...validChange }],
    });
    expect(equal.changes).toEqual([validChange]);
    expect(equal.warnings).toHaveLength(1);
    expect(equal.warnings[0]).toContain('duplicate id');
    expect(equal.conflictingDuplicate).toBe(false);

    const conflicting = sanitizeInput({
      locale: 'ru',
      changes: [validChange, { id: 'common.close', value: 'Другое' }],
    });
    expect(conflicting.conflictingDuplicate).toBe(true);
    expect(conflicting.dropped[0].reason).toContain('duplicate_id');
  });
});

describe('contribution bundle: §6 validation gate', () => {
  it('placeholder set must equal the en source contract', () => {
    const id = 'contractops.export.desc'; // source: "... Language folder: {locale}."
    expect(validateChange({ id, value: 'Без токена' }, en)?.reason).toContain(
      JSON.stringify(['locale']),
    );
    expect(validateChange({ id, value: 'Лишний {extra} и верный {locale}' }, en)?.reason).toContain(
      'placeholder set mismatch',
    );
    expect(validateChange({ id, value: 'Папка: {locale}.' }, en)).toBeNull();
  });

  it('value sanity: empty, over-limit, control characters', () => {
    expect(validateChange({ id: 'common.close', value: '' }, en)?.reason).toContain('value is empty');
    expect(validateChange({ id: 'common.close', value: 'x'.repeat(1001) }, en)?.reason).toContain(
      '1000-character limit',
    );
    expect(validateChange({ id: 'common.close', value: 'ок\x07' }, en)?.reason).toContain(
      'control characters',
    );
    expect(validateChange({ id: 'common.close', value: 'x'.repeat(1000) }, en)).toBeNull();
  });

  it('secret patterns are detected by name and never echoed', () => {
    const hits = validateChange({ id: 'common.close', value: 'key AKIAIOSFODNN7EXAMPLE внутри' }, en);
    expect(hits?.reason).toContain('aws-access-key');
    expect(hits?.reason).not.toContain('AKIAIOSFODNN7EXAMPLE');
    for (const [text, pattern] of [
      ['ghp_' + 'a'.repeat(36), 'github-token'],
      ['xoxb-123-abcdef', 'slack-token'],
      ['sk-proj-' + 'x'.repeat(30), 'api-key-prefix'],
      ['password=SuperSecret99', 'credential-assignment'],
      ['-----BEGIN RSA PRIVATE KEY-----', 'private-key-block'],
    ] as Array<[string, string]>) {
      expect(scanSecrets(text), text).toContain(pattern);
    }
    // ordinary words must not false-positive
    expect(scanSecrets('Введите пароль заново')).toEqual([]);
    expect(scanSecrets('Сбросить настройки доступа')).toEqual([]);
  });
});

describe('contribution bundle: preview (§5 preview-before-send)', () => {
  it('preview carries counts, id list, validation verdict and base revision', () => {
    const result = buildContribution(
      { locale: 'ru', changes: [validChange, { id: 'common.cancel', value: 'Отмена' }] },
      en,
      BASE_REVISION,
      ru,
    );
    expect(result.status).toBe('READY');
    expect(result.preview).toContain('locale: ru');
    expect(result.preview).toContain(`base catalog revision: ${BASE_REVISION}`);
    expect(result.preview).toContain('changes: 2 valid');
    expect(result.preview).toContain('1 improved, 1 identical (no-op)');
    expect(result.preview).toContain('ids: common.cancel, common.close');
    expect(result.preview).toContain('validation: passed');
    expect(result.preview).not.toContain('rejected');
  });

  it('preview is plain text built by buildPreview for every status', () => {
    const preview = buildPreview(
      {
        locale: 'ru',
        baseCatalogRevision: BASE_REVISION,
        changes: [],
        issues: [{ ref: 'x', reason: 'y' }],
        foreignRootFields: [],
        foreignEntryFields: [],
      },
      'NEEDS-FIXES',
    );
    expect(preview).toContain('status: NEEDS-FIXES');
    expect(preview).toContain('NEEDS FIX x: y');
  });

  it('the committed catalogs actually back the fixtures used above', () => {
    expect(en['contractops.export.desc']).toContain('{locale}');
    expect(readFileSync(join(ROOT, 'src', 'i18n', 'generated', 'catalog.en.json'), 'utf8')).toContain(
      'contractops.export.desc',
    );
  });
});
