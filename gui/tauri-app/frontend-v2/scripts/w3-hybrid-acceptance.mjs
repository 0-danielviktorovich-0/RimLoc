#!/usr/bin/env node
// W3 controlled hybrid acceptance (headless, no browser, no test runner):
// the FULL export → human-edit → import cycle over the REAL stores — the
// multi-target store (revisions, stale-AI protection) wired to the chat
// batch manager (lifecycle, retry-split). Same compile harness as
// scripts/w2-regressions.mjs (esbuild type-strip + svelte compileModule).
//
//   P0  baseline: untranslated inventory, batch-8 untouched
//   P1  TM fill (languages.applyTm): fills EMPTY targets only, origin TM,
//       revisions recorded (1 → 2); TM never overwrites existing work
//   P2  AI-chat batch: chat.copy → markWaiting while languages.exportAiBatch
//       captures per-entry revisions at export time
//   P3  human edits ONE exported entry BEFORE the reply: revision N → N+1
//   P4  stale AI import: the stale draft never writes — the human text
//       survives and the AI variant is parked as an LLM suggestion
//   P5  safe entries apply: AI text lands as pending_review/LLM with the
//       revision grown; the chat batch mirrors reality (ready 5, stale 1)
//       via chat.apply → needs_review, then chat.review → done
//   P6  chat batch lifecycle over every seeded state (waiting, exported,
//       imported, needs_review) + retry-split preserving accepted entries
//
// The two stores are linked by the scenario, not by a dependency: the chat
// batch ids (batch-8-of-8) map the conversation; the language batch id holds
// the store-side truth (locale + export revisions) the reply is judged by.
//
// Run: node scripts/w3-hybrid-acceptance.mjs   (exit 0 = ALL PASS)

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
  'src/lib/stores/chatbatch.svelte.ts': 'chatbatch.mjs',
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
const tmpBase = join(ROOT, '.w3-tmp');
rmSync(tmpBase, { recursive: true, force: true });
const dirA = join(tmpBase, 'a');
mkdirSync(dirA, { recursive: true });
await buildInstances(dirA, '');

const load = async (dir, name) => await import(pathToFileURL(join(dir, name)).href);

try {
  const { project } = await load(dirA, 'project.mjs');
  const { languages } = await load(dirA, 'languages.mjs');
  const { chat } = await load(dirA, 'chatbatch.mjs');

  // Active target is RU; the whole scenario runs over the RU dataset.
  const ds = (id) => languages.targets.ru.entries[id];
  const rev = (id) => languages.revisionOf('ru', id);

  // The AI-chat batch: six untranslated entries → batch-8 (not_started, 6).
  const B8 = ['keyed-02', 'keyed-09', 'keyed-12', 'di-03b', 'di-05', 'tk-02'];
  // TM hits for the remaining cheap wins (all still empty at baseline).
  const TM = {
    'keyed-04': 'Изгнать {PAWN_nameDef}?',
    'keyed-11': 'Переохлаждение',
    'di-08': 'катаракта',
    'tk-04': 'Вашему племени предстоит пережить грядущую зиму.'
  };
  // The mock AI reply, mapped back by stable entry ids.
  const AI = {
    'keyed-02': '{0}: Пришло письмо. (AI-черновик)',
    'keyed-09': 'Рядом приземляется кластер механоидов.',
    'keyed-12': '{0} пытается сбежать из {1}.',
    'di-03b': 'Длинный обоюдоострый клинок для дистанции и рычага.',
    'di-05': 'Жилет с пластинами из дьявольской кожи.',
    'tk-02': 'Вы прибываете с щедрыми запасами.'
  };
  const HUMAN = 'keyed-02'; // the one entry the human edits before the reply
  const HUMAN_TEXT = '{0}: Пришло письмо — правка человека.';

  // ------------------------------------------------------------------- P0
  console.log('\nP0 · baseline: inventory and batch state');
  eq(languages.activeLocale, 'ru', 'active target starts at ru');
  for (const id of B8) {
    eq(ds(id).target, '', `baseline ${id} has an empty target`);
    ok(
      ds(id).status === 'untranslated' || ds(id).status === 'todo',
      `baseline ${id} is untranslated/todo`
    );
    eq(rev(id), 1, `baseline ${id} revision is 1`);
  }
  const b8 = chat.byId('batch-8-of-8');
  ok(b8, 'batch-8-of-8 exists');
  eq(b8.status, 'not_started', 'batch-8 starts not_started');
  eq(b8.size, 6, 'batch-8 holds six entries');
  eq(b8.parent, null, 'batch-8 is a root batch');
  const progressBefore = languages.summary('ru').progress;

  // ------------------------------------------------------------------- P1
  console.log('\nP1 · TM fill: empty targets only, revisions recorded');
  const tmRes = languages.applyTm('ru', TM);
  eq(tmRes.applied, 4, 'TM fill applies all four hits');
  eq(tmRes.skipped, 0, 'TM fill skips nothing on a clean run');
  for (const [id, text] of Object.entries(TM)) {
    eq(rev(id), 2, `TM fill recorded revision 1 → 2 for ${id}`);
    eq(ds(id).status, 'translated', `TM-filled ${id} is translated`);
    eq(ds(id).origin, 'TM', `TM-filled ${id} keeps TM provenance`);
    eq(ds(id).target, text, `TM-filled ${id} holds the TM text`);
    eq(project.byId(id).target, text, `live entry ${id} mirrors the TM fill`);
  }
  ok(languages.summary('ru').progress > progressBefore, 'progress grew after the TM fill');
  // TM never overwrites existing work:
  const overwrite = languages.applyTm('ru', { 'keyed-03': 'ЛАТ', 'no-such-id': 'x' });
  eq(overwrite.applied, 0, 'TM fill refuses to overwrite translated entries');
  eq(overwrite.skipped, 2, 'overwritten + unknown entries are skipped');
  eq(ds('keyed-03').target, 'Исследования', 'existing human text untouched by TM');
  eq(rev('keyed-03'), 1, 'skipped entry keeps its revision');

  // ------------------------------------------------------------------- P2
  console.log('\nP2 · AI-chat batch export: copy → waiting, revisions captured');
  chat.copy('batch-8-of-8');
  eq(b8.status, 'exported', 'copy marks the batch exported');
  eq(chat.copiedId, 'batch-8-of-8', 'clipboard receipt remembers the batch');
  const langBatch = languages.exportAiBatch('ru', B8);
  ok(langBatch, 'language store exported a trusted batch');
  for (const id of B8) {
    eq(langBatch.revs[id], rev(id), `export captured the current revision of ${id}`);
  }
  ok(!('keyed-03' in langBatch.revs), 'entries outside the batch are not in the revision map');
  chat.markWaiting('batch-8-of-8');
  eq(b8.status, 'waiting', 'the prompt is in the chat — batch waits');

  // ------------------------------------------------------------------- P3
  console.log('\nP3 · human edits ONE exported entry before the reply');
  eq(rev(HUMAN), 1, 'the edited entry sat at revision 1 (N)');
  ok(languages.applyHumanEdit('ru', HUMAN, HUMAN_TEXT), 'human edit accepted at the store boundary');
  eq(rev(HUMAN), 2, 'human edit bumped the revision N → N+1');
  eq(ds(HUMAN).target, HUMAN_TEXT, 'dataset holds the human text');
  eq(ds(HUMAN).origin, 'human', 'dataset marks the entry human-owned');
  eq(ds(HUMAN).status, 'translated', 'human edit lands as translated');
  eq(project.byId(HUMAN).target, HUMAN_TEXT, 'live entry mirrors the human edit');

  // ------------------------------------------------------------------- P4
  console.log('\nP4 · stale AI import: human text survives, AI → suggestion');
  chat.importReply('batch-8-of-8');
  eq(b8.status, 'imported', 'the reply was pasted back');
  const staleRes = languages.applyAiResult(langBatch, HUMAN, AI[HUMAN]);
  eq(staleRes.status, 'stale', 'result exported at rev N is stale after the human edit');
  eq(staleRes.currentRev, 2, 'stale result reports the current revision N+1');
  eq(ds(HUMAN).target, HUMAN_TEXT, 'human text survives the stale AI result');
  eq(ds(HUMAN).origin, 'human', 'human provenance survives');
  eq(ds(HUMAN).status, 'translated', 'status stays translated — no overwrite');
  ok(
    (ds(HUMAN).suggestions ?? []).some((s) => s.source === 'LLM' && s.text === AI[HUMAN]),
    'the stale AI draft is parked as an LLM suggestion on the entry'
  );
  ok(
    (project.byId(HUMAN).suggestions ?? []).some((s) => s.text === AI[HUMAN]),
    'live entry shows the parked suggestion (active locale)'
  );
  b8.stale = 1; // the mapped reply says: 1 of 6 entries is stale

  // ------------------------------------------------------------------- P5
  console.log('\nP5 · safe entries apply; the chat batch mirrors reality');
  for (const id of B8.filter((x) => x !== HUMAN)) {
    const r = languages.applyAiResult(langBatch, id, AI[id]);
    eq(r.status, 'applied', `safe entry applies: ${id}`);
    eq(ds(id).target, AI[id], `${id} holds the AI text`);
    eq(ds(id).status, 'pending_review', `${id} lands as pending_review, never translated`);
    eq(ds(id).origin, 'LLM', `${id} keeps LLM provenance`);
    eq(rev(id), langBatch.revs[id] + 1, `${id} revision grew past the export snapshot`);
    eq(project.byId(id).target, AI[id], `live entry ${id} mirrors the applied result`);
  }
  const pv = chat.preview(b8);
  eq(pv.ready, 5, 'preview: five entries ready to apply');
  eq(pv.stale, 1, 'preview: one stale entry held back');
  eq(pv.review, 1, 'preview: one entry needs review');
  chat.apply('batch-8-of-8');
  eq(b8.accepted, 5, 'apply accepted exactly the ready entries');
  eq(b8.status, 'needs_review', 'apply sends the batch to review while something is held');

  // ------------------------------------------------------------------- P6
  console.log('\nP6 · batch lifecycle + retry-split preserving accepted');
  chat.review('batch-8-of-8');
  eq(b8.accepted, 5, 'review keeps the accepted count');
  eq(b8.skipped, 1, 'review parks the stale entry as skipped');
  eq(b8.problems, 0, 'review clears problems');
  eq(b8.stale, 0, 'review clears stale');
  eq(b8.status, 'done', 'hybrid batch closes done');

  // Seeded needs_review batch: review accepts problems, preserves stale as skipped.
  const b3 = chat.byId('batch-3-of-8');
  chat.review('batch-3-of-8');
  eq(b3.accepted, 9, 'seeded review accepted the problem entries');
  eq(b3.skipped, 1, 'seeded review preserved the stale entry as skipped');
  eq(b3.status, 'done', 'seeded batch closes done');

  // Seeded imported batch: apply alone closes it.
  const b4 = chat.byId('batch-4-of-8');
  chat.apply('batch-4-of-8');
  eq(b4.accepted, 10, 'imported batch applies all ready entries');
  eq(b4.status, 'done', 'imported batch closes done');

  // Seeded waiting batch: full loop import → apply.
  const b6 = chat.byId('batch-6-of-8');
  chat.importReply('batch-6-of-8');
  eq(b6.status, 'imported', 'waiting batch imports');
  chat.apply('batch-6-of-8');
  eq(b6.status, 'done', 'waiting batch closes done');

  // Seeded exported batch: full loop waiting → import → apply.
  const b7 = chat.byId('batch-7-of-8');
  chat.markWaiting('batch-7-of-8');
  eq(b7.status, 'waiting', 'exported batch waits');
  chat.importReply('batch-7-of-8');
  chat.apply('batch-7-of-8');
  eq(b7.accepted, 10, 'exported batch applies fully');
  eq(b7.status, 'done', 'exported batch closes done');

  // Retry-split of a partial batch: accepted stay put, children run the cycle.
  const b5 = chat.byId('batch-5-of-8');
  eq(b5.status, 'partial', 'the split candidate is the seeded partial batch');
  eq(b5.accepted, 7, 'partial batch has accepted work before the split');
  chat.split('batch-5-of-8');
  ok(b5.split, 'split recorded on the parent');
  eq(b5.split.preserved, 7, 'split preserves the accepted entries');
  eq(b5.split.aId, 'batch-5a-of-8', 'child A keeps the stable id shape');
  eq(b5.split.bId, 'batch-5b-of-8', 'child B keeps the stable id shape');
  eq(b5.accepted, 7, 'parent accepted count untouched by the split');
  const kids = chat.childrenOf('batch-5-of-8');
  eq(kids.length, 2, 'split created two children');
  for (const kid of kids) {
    eq(kid.parent, 'batch-5-of-8', `${kid.id} points back at the parent`);
    eq(kid.status, 'not_started', `${kid.id} starts fresh`);
    chat.copy(kid.id);
    eq(kid.status, 'exported', `${kid.id} copied`);
    chat.markWaiting(kid.id);
    eq(kid.status, 'waiting', `${kid.id} waits`);
    chat.importReply(kid.id);
    eq(kid.status, 'imported', `${kid.id} imports`);
    chat.apply(kid.id);
    eq(kid.accepted, kid.size, `${kid.id} applies fully`);
    eq(kid.status, 'done', `${kid.id} closes done`);
  }
  chat.split('batch-5-of-8'); // guard: already split
  eq(chat.childrenOf('batch-5-of-8').length, 2, 'double split is a no-op');

  // Guards: unknown ids are safe no-ops everywhere.
  chat.copy('nope');
  chat.markWaiting('nope');
  chat.importReply('nope');
  chat.apply('nope');
  chat.review('nope');
  chat.split('nope');
  ok(true, 'unknown batch ids are safe no-ops');

  // Derived wiring over the whole run.
  eq(chat.rootBatches.length, 8, 'root batches stay eight after the split');
  const sumAccepted = chat.batches.reduce((s, b) => s + b.accepted, 0);
  eq(chat.acceptedTotal, sumAccepted, 'acceptedTotal derives from all batches');
  eq(chat.acceptedTotal, 81, 'acceptedTotal matches the scenario arithmetic');
  eq(chat.reviewTotal, 0, 'nothing is left for review after the run');
  ok(chat.nextToCopy === b5, 'the partial parent remains the next copyable batch');
  ok(languages.summary('ru').progress > progressBefore, 'RU progress grew over the whole cycle');

  // ----------------------------------------------------------------- result
  console.log(
    failures.length === 0
      ? `\nALL PASS · ${passed} checks`
      : `\n${passed} passed, ${failures.length} failed`
  );
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
