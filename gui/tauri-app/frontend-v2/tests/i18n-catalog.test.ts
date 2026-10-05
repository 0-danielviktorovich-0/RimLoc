// i18n catalog hygiene — full-catalog guards that the per-wave parity tests
// (home-naming, w45-integration) only cover for their own subsets.
//
// Covers (self-localization mandate §2/§3 evidence, audit 2026-09-27):
//   1. full ru/en key parity — the whole catalog, not a wave subset;
//   2. no empty/whitespace-only values;
//   3. {placeholder} sets match between locales per key;
//   4. no duplicate keys inside a dictionary source (TS objects would
//      silently override them, so this scans the raw source);
//   5. no hardcoded user-visible text in .svelte templates: Cyrillic in a
//      template is always a violation (ru strings live in ru.ts), and
//      sentence-like English text is a violation outside the explicit
//      allowlist (data, licenses, key names — see ALLOWED).
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';

const ROOT = join(__dirname, '..');
const I18N_SRC = join(ROOT, 'src', 'i18n');

function extractPlaceholders(v: string): string[] {
  return [...v.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}

function* walkSvelte(dir: string): Generator<string> {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* walkSvelte(p);
    else if (name.endsWith('.svelte')) yield p;
  }
}

/** Template text outside {...} expressions and comments; single-line only —
 * copy lives on one template line, while multi-line spans between block tags
 * are markup, not text (keeps the guard near-zero false positives). */
function templateText(src: string): string[] {
  const noComments = src.replace(/<!--[\s\S]*?-->/g, '');
  return [...noComments.matchAll(/>([^<>\n]+)</g)]
    .map((m) => m[1].replace(/\{[^}]*\}/g, '').trim())
    .filter(Boolean);
}

// Non-UI text that legitimately lives in templates (audit 2026-09-27 §4):
// identity/data, not copy. Keyed by file name suffix, matched as substring.
const ALLOWED: Array<[string, RegExp]> = [
  ['About.svelte', /GPL|GNU|font|licen/i],
  ['Wizard.svelte', /^English$/], // source-locale name as option data
  ['Phase2Stub.svelte', /^(Ctrl|Enter|Esc|Tab)$/],
  ['LanguageManager.svelte', /Tok Pisin/], // field example placeholder
  ['ExternalEditorSettings.svelte', /editor-or-/], // field example placeholder
];

describe('i18n catalog: full ru/en parity', () => {
  it('key sets are identical', () => {
    const enKeys = new Set(Object.keys(en));
    const ruKeys = new Set(Object.keys(ru));
    const onlyEn = [...enKeys].filter((k) => !ruKeys.has(k));
    const onlyRu = [...ruKeys].filter((k) => !enKeys.has(k));
    expect(onlyEn, 'keys present in en.ts but missing in ru.ts').toEqual([]);
    expect(onlyRu, 'keys present in ru.ts but missing in en.ts').toEqual([]);
  });

  it('no empty or whitespace-only values', () => {
    for (const [dict, name] of [
      [en, 'en'],
      [ru, 'ru'],
    ] as const) {
      const empties = Object.entries(dict)
        .filter(([, v]) => !v.trim())
        .map(([k]) => k);
      expect(empties, `${name}.ts empty values`).toEqual([]);
    }
  });

  it('placeholder sets match between locales per key', () => {
    const mismatches: string[] = [];
    for (const key of Object.keys(en)) {
      const enPh = extractPlaceholders(en[key]);
      const ruPh = extractPlaceholders(ru[key] ?? '');
      if (JSON.stringify(enPh) !== JSON.stringify(ruPh)) mismatches.push(key);
    }
    expect(mismatches, 'placeholder mismatch').toEqual([]);
  });
});

describe('i18n catalog: duplicate keys in dictionary sources', () => {
  for (const [file, dict] of [
    ['en.ts', en],
    ['ru.ts', ru],
  ] as const) {
    it(`${file} defines each key exactly once`, () => {
      const src = readFileSync(join(I18N_SRC, file), 'utf8');
      const seen = new Map<string, number>();
      for (const m of src.matchAll(/^\s*'([^']+)':/gm)) {
        seen.set(m[1], (seen.get(m[1]) ?? 0) + 1);
      }
      const dupes = [...seen.entries()].filter(([, n]) => n > 1).map(([k]) => k);
      // Keys duplicated in source silently override at runtime; the runtime
      // object holds only the survivor, so compare against the real key set.
      const runtimeKeys = new Set(Object.keys(dict));
      const realDupes = dupes.filter((k) => !runtimeKeys.has(k) || (seen.get(k) ?? 0) > 1);
      expect(realDupes, `${file} duplicate keys`).toEqual([]);
    });
  }
});

describe('i18n guard: no hardcoded UI text in .svelte templates', () => {
  const files = [...walkSvelte(join(ROOT, 'src'))];

  it('found the svelte tree (sanity)', () => {
    expect(files.length).toBeGreaterThan(40);
  });

  it('no Cyrillic in template text (ru strings live in ru.ts)', () => {
    const violations: string[] = [];
    for (const f of files) {
      const rel = f.slice(ROOT.length + 1);
      for (const text of templateText(readFileSync(f, 'utf8'))) {
        if (/[\u0400-\u04FF]/.test(text)) violations.push(`${rel}: «${text.slice(0, 60)}»`);
      }
    }
    expect(violations).toEqual([]);
  });

  it('no sentence-like English outside the allowlist', () => {
    const violations: string[] = [];
    for (const f of files) {
      const rel = f.slice(ROOT.length + 1);
      const base = f.split('/').pop() ?? f;
      for (const text of templateText(readFileSync(f, 'utf8'))) {
        if (!/\b[a-z]{3,}\s+[a-z]{3,}\s+[a-z]{3,}\b/i.test(text)) continue;
        if (ALLOWED.some(([file, re]) => base === file && re.test(text))) continue;
        violations.push(`${rel}: "${text.slice(0, 60)}"`);
      }
    }
    expect(violations, 'hardcoded English UI text').toEqual([]);
  });
});
