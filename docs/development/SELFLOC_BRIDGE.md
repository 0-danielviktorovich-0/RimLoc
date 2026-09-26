# Self-Localization JSON Bridge

Status: foundation slice (wave B, 2026-09-27). Mandate: "RimLoc translates RimLoc" —
the app catalog becomes an ordinary inventory the engine can scan, translate and
package. This document specifies the build-time JSON bridge between the frontend
i18n catalog and the canonical project model.

Evidence base: self-localization audit 2026-09-27 (`/tmp/rimloc-selfloc-i18n-audit.md`
at decision time; §1 catalog format, §8 integration surface).

## One Authority

The TS dictionaries are the **only** hand-edited source of catalog content:

- `gui/tauri-app/frontend-v2/src/i18n/en.ts` — source texts (authoritative);
- `gui/tauri-app/frontend-v2/src/i18n/ru.ts` — the one shipped translation.

The JSON files under `gui/tauri-app/frontend-v2/src/i18n/generated/` are
**generated artifacts** (`npm run export:catalog`) and are committed so the
engine and downstream tooling can consume them without a TypeScript toolchain:

| File | Content |
|---|---|
| `catalog.en.json` | source messages, declaration order |
| `catalog.ru.json` | Russian translations, same order |
| `catalog.meta.json` | schema/identity/revision metadata |

Rules:

1. **Never hand-edit `generated/*.json`.** The drift-guard test
   (`tests/export-catalog.test.ts`) compares the committed files against the
   live dictionaries byte-for-byte; a manual edit or a forgotten re-export
   fails `npm test` with the fix spelled out: run `npm run export:catalog`.
2. Editing the catalog means editing `en.ts` / `ru.ts`, then re-running the
   exporter and committing both together.
3. `catalog.meta.json.catalog_revision` pins the git commit the JSON was
   generated from (with `-dirty` when the tree had local changes at export
   time). Note the pinned commit is necessarily the commit *before* the one
   that lands the files — a test cannot assert equality with HEAD, only the
   revision's shape.

## Regeneration

```bash
cd gui/tauri-app/frontend-v2
npm run export:catalog   # tsx scripts/export-catalog.ts
```

Deterministic by construction: message order follows dictionary declaration
order, no timestamps are emitted, and two runs over the same tree produce
byte-identical output (covered by a test that exports twice and compares).
The only variable is the `-dirty` suffix, which honestly reports the tree
state the export came from.

## File Schema

`schema_version` starts at `"1"`. Additive changes are minor; renaming or
removing fields requires a version bump and a note here.

### `catalog.en.json` / `catalog.ru.json`

```jsonc
{
  "messages": [
    {
      "id": "contractops.export.desc",
      "source_text": "Isolated output into a directory YOU choose ... Language folder: {locale}.", // en only
      "translated": "...",                                                                         // ru only
      "placeholders": ["locale"] // sorted, deduplicated {name} tokens of THIS text
    }
  ]
}
```

- `id` — flat catalog key `<screen>.<block>.<element>`; unique; equals the TS
  object key.
- `source_text` (en) / `translated` (ru) — verbatim dictionary value,
  `{name}`-interpolated at runtime by `src/i18n/store.svelte.ts`.
- `placeholders` — sorted unique token names extracted with `/\{(\w+)\}/g`.
  This is the validation contract for future language packs: a translation
  must interpolate exactly the same placeholder set as its source text (the
  same rule the mod validator already enforces for `{name}` tokens).

### `catalog.meta.json`

```jsonc
{
  "schema_version": "1",
  "source_app": "rimloc",
  "catalog_identity": { "package": "rimloc-gui-frontend-v2", "version": "0.1.0" },
  "catalog_revision": "7a3cfb2…[-dirty]",
  "source_locale": "en"
}
```

There is deliberately **no `generated_at` timestamp** — it would make runs
non-deterministic; the git revision is the provenance.

## Mapping to the Canonical Model

The audit (§8) checked this against `crates/rimloc-domain/src/canonical.rs`;
the JSON bridge makes the mapping mechanical:

- each message becomes a `SourceEntry`:
  - `id` → `SourceEntryId` with `kind` in the Keyed family and `key = id`
    (`common.appName` → `Keyed { key: "common.appName" }`). Whether the
    catalog deserves a dedicated `EntryKind::Application` (to stay filterable
    against mod Keyed entries) is a decision for the next wave — the JSON
    does not encode a kind, so introducing one later is additive;
  - `source_text` → `SourceEntry.text` with `source_locale = "en"`;
  - `ru` file → `Translation { locale: "ru", text }` slots;
  - `placeholders` → the existing `{name}` validation rules apply unchanged;
  - no plurals/select constructs exist in the catalog — the inventory is
    minimally complex;
- `catalog.meta.json` supplies project-level identity; the revision field
  gives the contribution/provenance layer a stable "generated from" anchor;
- per-entry context (`SourceContext { file, line }`) is a later, optional
  refinement: the exporter could emit dictionary line numbers, but nothing
  consumes them yet.

Path forward (catalog → language pack): `generated/*.json` is opened as a
small keyed project (plugin over the JSON, patterned after
`crates/rimloc-plugin-jsonftl` — the existing non-XML-source precedent), a
translator edits translations in the ordinary project/session flow, and the
result is exported as a language pack the app runtime can load. The runtime
side (loading external packs into `DICTS`, today compiled into the bundle)
is out of scope for this slice.

Reverse path (contribution → catalog), design only: an accepted language pack
(or an in-repo contribution) is applied back onto the TS dictionaries by
re-keying translations onto the catalog ids and writing `ru.ts` (or a new
`<locale>.ts`) — the TS layer stays the authority, so application is a
generated *source* file produced from reviewed data, in the same spirit as
this bridge but in the opposite direction. Conflict handling keys off
`catalog_revision`: a contribution built against revision N is applied only
after a rebase against the current dictionary (changed source texts flagged
per id, stale translations never silently dropped). Both directions are
future work; nothing in this slice consumes or writes packs.

## Boundaries

- The exporter imports **exactly** `src/i18n/en.ts` and `src/i18n/ru.ts` —
  repository-owned code. It must never be pointed at arbitrary TS/JS: it
  executes what it imports, so untrusted input is out of contract. A native
  scanner for unknown TS is explicitly NOT built here (the JSON bridge is the
  chosen alternative to putting a TS parser into the engine).
- The bridge is build-time only. The frontend runtime still compiles the
  dictionaries into the bundle; nothing reads `generated/` at runtime today.
- Determinism is a hard requirement: no timestamps, no locale-dependent
  ordering, stable JSON formatting (`JSON.stringify(…, 2)` + newline).
