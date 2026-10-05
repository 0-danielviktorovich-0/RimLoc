// JSON bridge guards for the self-localization foundation
// (docs/development/SELFLOC_BRIDGE.md).
//
// Covers:
//   1. drift guard — generated JSON in src/i18n/generated/ matches the live
//      TS dictionaries; hand-edits to generated files and forgotten re-runs
//      of `npm run export:catalog` both fail here (ONE AUTHORITY contract);
//   2. determinism — running the exporter twice produces byte-identical
//      output for an unchanged tree (no timestamps, stable order);
//   3. schema — every message carries id/placeholders, locale-specific text
//      fields only, meta carries schema_version and a git revision;
//   4. placeholder contract — placeholders equal the sorted {token} set of
//      the message text (future validation contract for pack translations).
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import {
  buildCatalogEn,
  buildCatalogRu,
  extractPlaceholders,
  main as exportCatalog,
  serialize,
} from '../scripts/export-catalog';

const ROOT = join(__dirname, '..');
const GENERATED = join(ROOT, 'src', 'i18n', 'generated');

type EnMessage = { id: string; source_text: string; placeholders: string[] };
type RuMessage = { id: string; translated: string; placeholders: string[] };

const enDisk = JSON.parse(readFileSync(join(GENERATED, 'catalog.en.json'), 'utf8')) as {
  messages: EnMessage[];
};
const ruDisk = JSON.parse(readFileSync(join(GENERATED, 'catalog.ru.json'), 'utf8')) as {
  messages: RuMessage[];
};
const metaDisk = JSON.parse(readFileSync(join(GENERATED, 'catalog.meta.json'), 'utf8')) as Record<
  string,
  unknown
>;

describe('catalog JSON bridge: drift guard (ONE AUTHORITY)', () => {
  it('catalog.en.json matches the live en.ts dictionary', () => {
    expect(readFileSync(join(GENERATED, 'catalog.en.json'), 'utf8')).toBe(
      serialize(buildCatalogEn(en)),
    );
  });

  it('catalog.ru.json matches the live ru.ts dictionary', () => {
    expect(readFileSync(join(GENERATED, 'catalog.ru.json'), 'utf8')).toBe(
      serialize(buildCatalogRu(ru)),
    );
  });
});

describe('catalog JSON bridge: exporter determinism', () => {
  it('two consecutive exports are byte-identical', () => {
    // SF-11 test isolation: both runs write into a TEMP copy, never into the
    // shared src/i18n/generated/ — parallel vitest workers read those files,
    // and the old mutate-then-restore approach raced against them.
    const files = ['catalog.en.json', 'catalog.ru.json', 'catalog.meta.json'];

    // The -dirty suffix on catalog_revision honestly reflects the tree state
    // at export time; sibling test workers create scratch files concurrently,
    // so the flag may legitimately flip between the two runs. Determinism is
    // therefore asserted modulo the flag: the revision SHA and every other
    // byte (catalog content, order, formatting) must match exactly.
    const normalize = (file: string, content: string): string => {
      if (file !== 'catalog.meta.json') return content;
      const meta = JSON.parse(content) as { catalog_revision: string } & Record<string, unknown>;
      meta.catalog_revision = meta.catalog_revision.replace(/-dirty$/, '');
      return serialize(meta);
    };

    const firstDir = mkdtempSync(join(tmpdir(), 'rimloc-export-det-'));
    const secondDir = mkdtempSync(join(tmpdir(), 'rimloc-export-det-'));
    try {
      exportCatalog(firstDir);
      exportCatalog(secondDir);
      for (const f of files) {
        const first = readFileSync(join(firstDir, f), 'utf8');
        const second = readFileSync(join(secondDir, f), 'utf8');
        expect(normalize(f, second), `${f} differs between runs`).toBe(normalize(f, first));
      }
    } finally {
      rmSync(firstDir, { recursive: true, force: true });
      rmSync(secondDir, { recursive: true, force: true });
    }
  });
});

describe('catalog JSON bridge: schema', () => {
  it('catalog size tracks the live dictionaries', () => {
    expect(enDisk.messages.length).toBe(Object.keys(en).length);
    expect(ruDisk.messages.length).toBe(Object.keys(ru).length);
    expect(enDisk.messages.length).toBeGreaterThan(1000);
  });

  it('declaration order of the dictionary is preserved', () => {
    expect(enDisk.messages.map((m) => m.id)).toEqual(Object.keys(en));
    expect(ruDisk.messages.map((m) => m.id)).toEqual(Object.keys(ru));
  });

  it('en messages: id + source_text only, ru messages: id + translated only', () => {
    for (const [messages, textField, foreignField] of [
      [enDisk.messages, 'source_text', 'translated'],
      [ruDisk.messages, 'translated', 'source_text'],
    ] as Array<[Array<Record<string, unknown>>, string, string]>) {
      const bad = messages.filter(
        (m) =>
          typeof m.id !== 'string' ||
          m.id.length === 0 ||
          typeof m[textField] !== 'string' ||
          (m[textField] as string).length === 0 ||
          foreignField in m,
      );
      expect(bad.map((m) => m.id), `messages violating ${textField} shape`).toEqual([]);
    }
  });

  it('placeholders are sorted, unique, string arrays on every message', () => {
    for (const messages of [enDisk.messages, ruDisk.messages]) {
      for (const m of messages) {
        const ph = m.placeholders;
        expect(Array.isArray(ph), m.id).toBe(true);
        expect(ph, m.id).toEqual([...new Set(ph)].sort());
      }
    }
  });

  it('meta carries schema_version, identity and a git revision', () => {
    expect(metaDisk.schema_version).toBe('1');
    expect(metaDisk.source_app).toBe('rimloc');
    expect(metaDisk.source_locale).toBe('en');
    expect(metaDisk.catalog_identity).toEqual({
      package: 'rimloc-gui-frontend-v2',
      version: '0.1.0',
    });
    // Revision shape only: the JSON pins the generating commit, which is one
    // commit behind the commit that lands the files — equality with HEAD can
    // never hold in a committed state.
    expect(metaDisk.catalog_revision).toMatch(/^[0-9a-f]{40}(-dirty)?$/);
  });
});

describe('catalog JSON bridge: placeholder contract', () => {
  it('placeholders equal the sorted {token} set of the message text', () => {
    const violations: string[] = [];
    for (const [messages, field] of [
      [enDisk.messages, 'source_text'],
      [ruDisk.messages, 'translated'],
    ] as Array<[Array<EnMessage | RuMessage>, string]>) {
      for (const m of messages) {
        const text = (m as Record<string, unknown>)[field] as string;
        if (JSON.stringify(m.placeholders) !== JSON.stringify(extractPlaceholders(text))) {
          violations.push(m.id);
        }
      }
    }
    expect(violations, 'placeholder set mismatch').toEqual([]);
  });

  it('the catalog exercises the placeholder contract', () => {
    expect(enDisk.messages.filter((m) => m.placeholders.length > 0).length).toBeGreaterThan(50);
  });
});
