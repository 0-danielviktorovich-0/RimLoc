/**
 * Build-time export of the UI catalog (src/i18n/en.ts + ru.ts) into a
 * versioned JSON bridge for the self-localization foundation
 * (docs/development/SELFLOC_BRIDGE.md).
 *
 * ONE AUTHORITY: the TS dictionaries are the only hand-edited source. Files
 * under src/i18n/generated/ are produced by this script and must never be
 * edited by hand — a drift-guard test compares them against the live
 * dictionaries and fails when they disagree.
 *
 * Trust boundary: this script imports exactly the two repository-owned
 * dictionary modules (en.ts, ru.ts) and nothing else. It must NEVER be
 * pointed at untrusted TS/JS — it executes what it imports.
 *
 * Determinism: repeated runs are byte-for-byte identical given the same
 * catalog content and the same git state. Message order follows dictionary
 * declaration order (Object.entries preserves insertion order for string
 * keys), no timestamps are emitted; the only variable part is the `-dirty`
 * suffix on catalog_revision, which reflects the actual git tree state.
 */
import { execSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url)); // scripts/
const ROOT = dirname(SCRIPT_DIR); // frontend-v2 package root
const OUT_DIR = join(ROOT, 'src', 'i18n', 'generated');

/** {name} interpolation tokens, same syntax the i18n store replaces. */
const PLACEHOLDER_RE = /\{(\w+)\}/g;

/** Sorted, deduplicated placeholder names referenced by a message text. */
export function extractPlaceholders(text: string): string[] {
  return [...new Set([...text.matchAll(PLACEHOLDER_RE)].map((m) => m[1]))].sort();
}

export interface EnMessage {
  id: string;
  source_text: string;
  placeholders: string[];
}

export interface RuMessage {
  id: string;
  translated: string;
  placeholders: string[];
}

export interface CatalogFile {
  messages: EnMessage[] | RuMessage[];
}

export interface CatalogMeta {
  schema_version: string;
  source_app: string;
  catalog_identity: { package: string; version: string };
  catalog_revision: string;
  source_locale: string;
}

/** English catalog: declaration order kept, text under source_text. */
export function buildCatalogEn(dict: Record<string, string>): { messages: EnMessage[] } {
  return {
    messages: Object.entries(dict).map(([id, text]) => ({
      id,
      source_text: text,
      placeholders: extractPlaceholders(text),
    })),
  };
}

/** Russian catalog: declaration order kept, text under translated. */
export function buildCatalogRu(dict: Record<string, string>): { messages: RuMessage[] } {
  return {
    messages: Object.entries(dict).map(([id, text]) => ({
      id,
      translated: text,
      placeholders: extractPlaceholders(text),
    })),
  };
}

/**
 * git HEAD of the generating tree, with a `-dirty` suffix when the working
 * tree (including untracked files) differs from HEAD. No timestamp is
 * emitted: the revision is the only provenance and it keeps runs
 * deterministic for a fixed tree state.
 */
export function resolveCatalogRevision(): string {
  const run = (cmd: string) =>
    execSync(cmd, { encoding: 'utf8', cwd: ROOT });
  const sha = run('git rev-parse HEAD').trim();
  const dirty = run('git status --porcelain').length > 0;
  return dirty ? `${sha}-dirty` : sha;
}

/** Fixed-field meta; object key order is part of the stable output. */
export function buildMeta(identity: { name: string; version: string }, revision: string): CatalogMeta {
  return {
    schema_version: '1',
    source_app: 'rimloc',
    catalog_identity: { package: identity.name, version: identity.version },
    catalog_revision: revision,
    source_locale: 'en',
  };
}

/** Deterministic serialization: 2-space indent + trailing newline. */
export function serialize(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

/** Regenerate the generated catalog JSON from the live dictionaries.
 * `outDir` defaults to src/i18n/generated/; tests pass a TEMP copy so the
 * determinism check never mutates the shared checkout (SF-11: parallel test
 * workers read those files concurrently). */
export function main(outDir: string = OUT_DIR): void {
  const pkg = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')) as {
    name: string;
    version: string;
  };
  const revision = resolveCatalogRevision();

  mkdirSync(outDir, { recursive: true });
  writeFileSync(join(outDir, 'catalog.en.json'), serialize(buildCatalogEn(en)));
  writeFileSync(join(outDir, 'catalog.ru.json'), serialize(buildCatalogRu(ru)));
  writeFileSync(join(outDir, 'catalog.meta.json'), serialize(buildMeta(pkg, revision)));

  console.log(
    `exported catalog: ${Object.keys(en).length} en / ${Object.keys(ru).length} ru messages -> ${outDir} (revision ${revision})`,
  );
}

/** Write only when invoked as the entry script, not when imported by tests. */
const invokedDirectly =
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) main();
