#!/usr/bin/env node
// W2 headless regressions (no browser, no test runner): compiles the runes
// stores with the project's own toolchain (esbuild type-strip + svelte
// compileModule) and asserts the multi-target invariants from the mandate:
//
//   1. switch RU→JA swaps translation data, source inventory identical,
//      entry objects NOT recreated (no rescan);
//   2. custom language creation persists to localStorage and survives reload;
//   3. the interface language (RU/EN) never affects project targets;
//   4. stale-AI protection: a result exported at revision N is rejected after
//      a human edit (becomes a suggestion, never a write) and can never land
//      in another locale;
//   5. reopen restores the persisted active target;
//   6. detach refuses the source locale and the last remaining target;
//   7. progress ordering ru > uk > ja > 0 matches the corpus coverage.
//
// Run: node scripts/w2-regressions.mjs   (exit 0 = all green)

import { writeFileSync, rmSync, mkdirSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { transform } from 'esbuild';
import { compileModule } from 'svelte/compiler';

const ROOT = resolve(new URL('..', import.meta.url).pathname);
const failures = [];
let passed = 0;

function ok(cond, name) {
  if (cond) {
    passed += 1;
    console.log(`  ok  ${name}`);
  } else {
    failures.push(name);
    console.log(`FAIL  ${name}`);
  }
}

function eq(actual, expected, name) {
  ok(actual === expected, `${name} (expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)})`);
}

// ---------------------------------------------------------------- environment

// localStorage shim (registry + store persistence).
const memStorage = new Map();
globalThis.localStorage = {
  getItem: (k) => (memStorage.has(k) ? memStorage.get(k) : null),
  setItem: (k, v) => void memStorage.set(k, String(v)),
  removeItem: (k) => void memStorage.delete(k)
};
// i18n store touches document.documentElement.lang.
globalThis.document = { documentElement: { setAttribute() {}, lang: '' } };

// ---------------------------------------------------------------- compile set

/** source file (relative to ROOT) → temp module name. */
const MODULES = {
  'src/lib/mock/types.ts': 'types.mjs',
  'src/lib/mock/data.ts': 'data.mjs',
  'src/lib/languages/registry.ts': 'registry.mjs',
  'src/lib/languages/corpus.ts': 'corpus.mjs',
  'src/lib/stores/project.svelte.ts': 'project.mjs',
  'src/lib/stores/settings.svelte.ts': null, // not needed; kept out
  'src/i18n/ru.ts': 'ru.mjs',
  'src/i18n/en.ts': 'en.mjs',
  'src/i18n/store.svelte.ts': 'i18n.mjs',
  'src/lib/languages/store.svelte.ts': 'languages.mjs'
};

/** Additional rewrites for relative imports whose target is in the set. */
function rewriteSpecifier(spec, fromFile) {
  if (!spec.startsWith('.')) return spec;
  const resolved = resolve(dirname(join(ROOT, fromFile)), spec).replaceAll('\\', '/');
  const rel = resolved.startsWith(ROOT) ? resolved.slice(ROOT.length + 1) : resolved;
  const candidates = [rel, `${rel}.ts`, `${rel}.svelte.ts`, `${rel}/index.ts`];
  for (const key of candidates) {
    const name = MODULES[key];
    if (name) return `./${name}`;
  }
  return spec;
}

async function buildInstances(dir, suffix) {
  for (const [file, name] of Object.entries(MODULES)) {
    if (!name) continue;
    const source = await (await import('node:fs/promises')).readFile(join(ROOT, file), 'utf8');
    // 1) strip TS types (runes survive: they are plain call syntax),
    // 2) hand the plain JS to Svelte's module compiler for runes wiring.
    const stripped = await transform(source, { loader: 'ts', format: 'esm' });
    const isRunes = file.endsWith('.svelte.ts');
    const outName = name.replace('.mjs', `${suffix}.mjs`);
    let code = stripped.code.replaceAll(
      /from\s+(['"])([^'"]+)\1/g,
      (_m, _q, spec) => `from ${_q}${rewriteSpecifier(spec, file).replace('.mjs', `${suffix}.mjs`)}${_q}`
    );
    if (isRunes) {
      const compiled = compileModule(code, {
        filename: outName,
        generate: 'client'
      });
      code = compiled.js.code;
    }
    writeFileSync(join(dir, outName), code);
  }
}

// Compiled modules must live INSIDE the project so the bare import
// 'svelte/internal/client' resolves through frontend-v2/node_modules.
const tmpBase = join(ROOT, '.w2-tmp');
rmSync(tmpBase, { recursive: true, force: true });
const dirA = join(tmpBase, 'a');
mkdirSync(dirA, { recursive: true });
await buildInstances(dirA, '');

const load = async (dir, name) => await import(pathToFileURL(join(dir, name)).href);

try {
  const { project } = await load(dirA, 'project.mjs');
  const registryMod = await load(dirA, 'registry.mjs');
  const { languages } = await load(dirA, 'languages.mjs');
  const { i18n } = await load(dirA, 'i18n.mjs');

  const sourceOf = (e) => ({ id: e.id, kind: e.kind, key: e.key, source: e.source, file: e.file, line: e.line });

  // ---------------------------------------------------------------- T1 switch
  console.log('\nT1 · switch RU→JA: data swaps, source identical, no rescan');
  const before = project.entries.map((e) => ({ ref: e, src: sourceOf(e), target: e.target }));
  const ruTargetKeyed01 = project.byId('keyed-01').target;
  eq(languages.activeLocale, 'ru', 'initial active target is ru');
  languages.setActive('ja');
  eq(languages.activeLocale, 'ja', 'active target switched to ja');
  ok(
    project.byId('keyed-01').target !== ruTargetKeyed01,
    'keyed-01 translation changed after switch'
  );
  eq(project.byId('keyed-03').target, '研究', 'keyed-03 shows the Japanese corpus text');
  eq(project.byId('keyed-03').status, 'translated', 'corpus entries are translated in ja');
  eq(project.byId('keyed-02').status, 'untranslated', 'non-corpus entries are untranslated in ja');
  ok(
    before.every((b, i) => {
      const cur = project.entries[i];
      return (
        Object.is(cur, b.ref) &&
        cur.id === b.src.id &&
        cur.kind === b.src.kind &&
        cur.key === b.src.key &&
        cur.source === b.src.source &&
        cur.file === b.src.file &&
        cur.line === b.src.line
      );
    }),
    'same entry objects, source fields untouched (no rescan)'
  );
  languages.setActive('ru');
  eq(project.byId('keyed-01').target, ruTargetKeyed01, 'switching back restores the RU text');
  eq(project.byId('keyed-06').status, 'sourceChanged', 'RU dataset keeps its rich statuses');

  // ------------------------------------------------------------ T2 custom lang
  console.log('\nT2 · create custom language + persistence');
  const created = registryMod.registry.createCustom({
    displayName: 'Tok Pisin',
    nativeName: 'Tok Pisin',
    localeId: 'tpi'
  });
  ok(created.ok, 'createCustom accepts a valid new locale');
  const fallback = registryMod.registry.resolve('xq-99');
  eq(fallback.origin, 'user', 'unknown locale resolves to Generic fallback, never rejects');
  eq(fallback.capabilities.morphology, false, 'Generic capabilities: morphology off');
  eq(fallback.capabilities.translation, true, 'Generic capabilities: translation on');
  ok(
    JSON.parse(memStorage.get('rimloc.languages.custom.v1')).some((l) => l.localeId === 'tpi'),
    'custom language persisted to localStorage'
  );
  ok(languages.addTarget('tpi', 'empty'), 'custom language added as an empty target');
  languages.setActive('tpi');
  eq(project.byId('keyed-01').target, '', 'empty target starts with no translations');
  eq(languages.summary('tpi').progress, 0, 'empty target progress is 0%');
  const dup = registryMod.registry.createCustom({
    displayName: 'Dup',
    nativeName: 'Dup',
    localeId: 'ru'
  });
  eq(dup.ok, false, 'duplicate of a builtin locale is refused at creation');

  // ------------------------------------------------------- T3 UI locale split
  console.log('\nT3 · interface language is independent of project targets');
  languages.setActive('ru');
  const targetsBefore = JSON.stringify(Object.keys(languages.targets));
  const dataBefore = project.byId('keyed-03').target;
  i18n.setLocale('en');
  eq(i18n.locale, 'en', 'interface locale switched to en');
  eq(JSON.stringify(Object.keys(languages.targets)), targetsBefore, 'target set unchanged by UI locale');
  eq(languages.activeLocale, 'ru', 'active target unchanged by UI locale');
  eq(project.byId('keyed-03').target, dataBefore, 'translation data unchanged by UI locale');
  i18n.setLocale('ru');

  // ------------------------------------------------------------- T4 stale-AI
  console.log('\nT4 · stale-AI protection per target locale');
  const batch1 = languages.exportAiBatch('ru', ['keyed-01', 'keyed-03']);
  const revAtExport = batch1.revs['keyed-01'];
  // Human edit happens after export: revision moves N → N+1 via capture-back.
  project.byId('keyed-01').target = 'Обновлено человеком';
  project.byId('keyed-01').origin = 'human';
  project.byId('keyed-01').editedAt = new Date().toISOString();
  languages.setActive('uk');
  languages.setActive('ru');
  eq(languages.revisionOf('ru', 'keyed-01'), revAtExport + 1, 'human edit bumps the entry revision');
  const stale = languages.applyAiResult(batch1, 'keyed-01', 'AI-черновик');
  eq(stale.status, 'stale', 'AI result exported at rev N is stale after a human edit');
  eq(stale.currentRev, revAtExport + 1, 'stale result reports the current revision');
  eq(project.byId('keyed-01').target, 'Обновлено человеком', 'human text survives the stale AI result');
  // A RU batch can never write into JA: the batch itself is the aim point and
  // revision comparisons happen inside the batch's OWN locale only.
  eq(project.byId('keyed-01').target, 'Обновлено человеком', 'RU human edit still intact');
  const jaBefore = languages.targets.ja.entries['keyed-01'].target;
  languages.applyAiResult({ ...batch1, locale: 'ja' }, 'keyed-01', 'AI-черновик');
  eq(languages.targets.ja.entries['keyed-01'].target, jaBefore, 'a mislabelled batch cannot inject into another locale dataset');
  const batch2 = languages.exportAiBatch('ru', ['keyed-01']);
  const applied = languages.applyAiResult(batch2, 'keyed-01', 'AI-черновик 2');
  eq(applied.status, 'applied', 'fresh AI result at the current revision applies');
  eq(project.byId('keyed-01').status, 'pending_review', 'AI output lands as pending_review, never translated');
  eq(project.byId('keyed-01').origin, 'LLM', 'AI output keeps LLM provenance');
  eq(languages.applyAiResult(batch2, 'no-such-entry', 'x').status, 'unknown-entry', 'unknown entry refused');
  const jaBatch = languages.exportAiBatch('ja', ['keyed-03']);
  languages.applyAiResult(jaBatch, 'keyed-03', 'AI для JA');
  eq(languages.targets.ja.entries['keyed-03'].target, 'AI для JA', 'result writes into ITS OWN locale dataset');
  eq(project.byId('keyed-03').target, 'Исследования', 'active RU view untouched by a JA-batch result');
  eq(languages.applyAiResult({ id: 'forged-batch', locale: 'ru', exportedAt: '', revs: {} }, 'keyed-01', 'x').status, 'unknown-batch', 'a batch this store never exported is refused');

  // ---------------------------------------------------------------- T5 reopen
  console.log('\nT5 · reopen restores the persisted active target');
  languages.setActive('ja');
  const dirB = join(tmpBase, 'b');
  mkdirSync(dirB, { recursive: true });
  await buildInstances(dirB, '-r');
  const project2 = await load(dirB, 'project-r.mjs');
  const reloaded = (await load(dirB, 'languages-r.mjs')).languages;
  eq(reloaded.activeLocale, 'ja', 'fresh store instance restores the persisted active target');
  ok(project2.project.byId('keyed-03').target === '研究', 'restored instance shows the restored dataset');
  ok(Object.keys(reloaded.targets).includes('tpi'), 'custom target survives reload (registry persisted)');
  // Dataset integrity after reopen: each locale keeps its OWN content.
  eq(reloaded.targets.ru.entries['keyed-01'].target, '{0}: Пришло письмо.', 'RU dataset intact after reopen');
  eq(reloaded.targets.ru.entries['keyed-03'].target, 'Исследования', 'RU dataset holds RU text after reopen');
  eq(reloaded.targets.uk.entries['keyed-03'].target, 'Дослідження', 'UK dataset intact after reopen');
  eq(reloaded.targets.ja.entries['keyed-03'].target, '研究', 'JA dataset intact after reopen');

  // ------------------------------------------------------------ T6 detach rules
  console.log('\nT6 · detach safety: source and last-target refused');
  eq(reloaded.detach('en').reason, 'source', 'source locale can never be detached');
  const onlyRu = join(tmpBase, 'c');
  mkdirSync(onlyRu, { recursive: true });
  await buildInstances(onlyRu, '-s');
  const { languages: single } = await load(onlyRu, 'languages-s.mjs');
  for (const key of Object.keys(single.targets)) if (key !== 'ru') single.detach(key);
  eq(single.detach('ru').reason, 'last', 'the last remaining target cannot be detached');
  eq(languages.detach('tpi').ok, true, 'a regular target detaches fine');

  // -------------------------------------------------------------- T7 progress
  console.log('\nT7 · progress ordering matches corpus coverage');
  const sum = (l) => languages.summary(l);
  ok(sum('ru').progress > sum('uk').progress, 'RU coverage above UK');
  ok(sum('uk').progress > sum('ja').progress, 'UK coverage above JA');
  ok(sum('ja').progress > 0, 'JA coverage non-zero');
  ok(sum('ja').progress < 25, 'JA coverage stays in the mandated ~15% band');
  ok(sum('ru').issueCount > 0, 'RU issues counted per target');
  eq(sum('uk').issueCount, 0, 'synthetic targets start without issues');

  // ------------------------------------------------------------------- result
  console.log(`\n${passed} passed, ${failures.length} failed`);
  if (failures.length > 0) {
    for (const f of failures) console.log(`  FAILED: ${f}`);
    process.exitCode = 1;
  }
} catch (err) {
  console.error('Harness error:', err);
  process.exitCode = 1;
} finally {
  rmSync(tmpBase, { recursive: true, force: true });
}
