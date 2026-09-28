# Changelog

All notable changes to this project are documented in this file.
This changelog follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

### Added
- [gui] Native folder picker on the create panel and on every output-path field (build/export/diagnostics): paths are chosen through the OS dialog instead of being typed by hand; cancelling is silent, errors are visible.
- [gui] Full mod-package build from the GUI (`project_build_mod`): a drop-in `ModMetaData` About plus Languages, the same guards as export (absolute-path refusal, source-tree containment, symlink aliasing, case-collision and control-char checks) — the result folder can be dropped into RimWorld Mods without the CLI.
- [gui] "Translate RimLoc" beta card: opens the app's own UI catalog (~1.2k messages) as an ORDINARY translation project through the standard create flow; a repeat click reopens the existing catalog project instead of duplicating it.
- [gui] Language pack preview (dev panel): data-only packs load through a strict schema (unknown ids, duplicate ids, placeholder mismatches and oversized values are refused whole), preview is in-memory and reversible, missing messages fall back to the built-in locale.
- [i18n] Deterministic build-time JSON bridge for the UI catalog (one authority: TS dictionaries are hand-edited, JSON is generated and drift-guarded by tests).
- [scripts] Offline contribution bundle: `build:contribution` (READY / PARTIAL-BUT-VALID / NEEDS-FIXES gate, sanitizer, secret scan) and `apply:contribution` (surgical apply onto the authoritative dictionaries with dry-run, stale-catalog gate and base-value conflict protection — a manual dictionary edit can never be silently overwritten).
- [services] First-party UI-catalog source adapter: a directory containing `catalog.en.json` opens as a normal project (Keyed-kind entries with `ui-catalog` provenance); the catalog participates in M3 source-drift; DefInjected machinery never touches application messages.
- [gui] Live Validate, Export and Diagnostics panels: translations can be validated, written into the mod tree and diagnosed directly from the GUI, through the same guarded pipeline as the CLI; problems appear as error/warning badges on the affected entries.
- [gui] Guided onboarding: a first-run tour over the real workspace plus a built-in demo project to explore without touching your mods.
- [gui] The app now shows which live backend features the current project actually supports and gates the rest, instead of offering buttons that dead-end.
- [gui] New projects pin the RimWorld game version they were created for.
- [services] Binding seam, first slice (lead 033): framework-neutral `contract` module (UI_CONTRACT_VERSION, typed DTOs — snapshot/summary/apply-intents/intents, ContractError with stable snake_case codes, honest capability report) and `session` module (opaque durable projectId, session epoch, durable revision in the same atomic record, per-project serialized apply-intents with identity/eligibility resolution, persist-before-ack, save_failed dirty semantics, external-change content-hash detection, cancel-next, restart recovery). Intents carry full structural SourceEntryIds — the UI never sends a whole Project.
- [services/store] Additive binding envelope in the v2 container: optional project_id/revision/display_name ride the same atomic record; bare saves stay legacy-shaped and old files load unchanged.
- [domain/services] Identity fix (pre-freeze blocker): `SourceEntryId` carries an optional def-type discriminator (participates in equality/order/hash). Two def types sharing one `{defName}.{field}` key are two distinct entries end-to-end — build, scoped pack/PO import, save/reload, native output in separate DefInjected catalogs, and full-identity source-change detection. Keyed stays unscoped; unknown types stay `None` (never a guessed "Misc").
- [services/project_store] Project container v2 with typed migration: v1 files load compatibly (discriminator derived only from unanimous evidence — TKey metadata, consistent contexts, or the legacy canonical DefInjected path); collapsed v1 collisions and contradictory evidence are REJECTED with typed actionable diagnostics and untouched bytes; v2 files are validated for identity uniqueness, translation-reference integrity and (identity, locale) slot uniqueness; save validates before writing, so an unloadable state can never replace the last good file.
- [core/services] Per-entry source provenance (pre-freeze, Source Inspector mandate): `TransUnit` gains `src` (real effective source file + parser-guaranteed line for Defs-derived units whose `path` is the canonical output path), `selected_by` (winner reason stamped at the scan decision point: first-file-wins / keyed-last-wins / keyed-first-in-file / definjected-setoradd / tkey-last-assignment / patch-applied) and `conditional` (unit from an `IfModActive` LoadFolders dir); canonical `SourceContext` now points at the real source file, and `SourceProvenance.selected_by` / `conditional_branch` are filled per entry.
- [parsers-xml] `TKeyMeta.locations`: real per-node source locations of same-file shared TKey identities (Primary + Other usages, Source Inspector §14), captured in the same parser pass; the canonical bridge emits the Primary (effective last assignment) plus earlier nodes as contexts.
- [services/scan] `scan_units_effective_full` / `scan_units_with_defs_and_dict_full` / `scan_units_effective_view`: the single scan pipeline also surfaces the real patch report and the resolved mod view; LoadFolders first-registration semantics now hold ACROSS content dirs (view order decides, not path sort); version-only layouts go through the same version resolver.
- [services/scan] `retain_source_language_units`: the canonical project never treats a foreign target pack (Languages/<other>) as English source — even for target-only or TKey-only mods.
- [gui] Source Inspector (W7, mock scope): SOURCE tab with effective location, nullable line/column, provenance "why" facts, primary + other usages, read-only viewer with search/folding/copy, advanced source browser with ACTIVE/shadowed candidates, compare source/translation/generated, context menu and palette actions, external-editor launch plan (structural argv, demo only — no editor is launched and no files are touched).
- [parsers-xml] Normalize nested DefInjected dotted keys by dropping leading def type segment when it matches the file's DefInjected/<DefType> folder; preserves mixed types and flat keys.
- [parsers-xml] Extended Defs DSL: multiple attribute predicates (name[@a=v&@b=w]) and indexed selection (name[2], name[#2], name[index=2]).
- [cli/scan] New `--fuzzy` flag to include heuristic string fields from Defs not covered by dicts (RIMLOC_FUZZY=1).
- [services] Merge Keyed service: merges English→target Keyed with EN: comments and UNUSED block.
- [cli] `merge-keyed` command to run the above.
- [cli] `coverage` command to report translated/missing counts (text/json).
- [export/import-xliff] Minimal XLIFF 1.2 exporter/importer crates and CLI (`export-xliff`, `import-xliff`).
- [cli/doctor] New `doctor` command: environment diagnostics (RimWorld install, version, mod dirs, provider configuration presence, output writability) with OK/WARNING/ERROR/NOT CHECKED statuses, remediation hints and `--format json`.
- [cli/validate] New `--support-bundle <OUT_DIR>` flag: captures the actual validation results into a sanitized support bundle (report.md, diagnostics.json, sanitized environment.json, sha256 manifest) for root-cause diagnosis; a run with issues is preserved as a failed operation.
- [cli/validate] Typed `severity` (error/warning/info) on every validation finding: only errors fail the run (warnings/info stay successful while preserved in output and support bundles); additive `severity` field in the `--format json` output.
- [services] Support-bundle API (`collect_support_bundle*`): secret-aware sanitizer with Included/Redacted/Excluded preview (credentials, tokens, home paths, env allowlist), structured operation log with operation IDs and causal error chains, output boundary guards against writing into read-only source trees.
- [services] Symlink-safe atomic writes: staging files are created with unique names via `create_new` (a pre-planted temp symlink can no longer be truncated), so user and game files cannot be overwritten through temp paths.
- [gui/tauri] Backend commands to load/list dynamic parser plugins (for future UI hookup).
- [parsers-xml] New `read_keyed_file_map(_with_comments)` API: robust Keyed reader with `<li>`/`<LineBreak/>`/CDATA and optional EN: comment override.
- [export-po] Translation Memory prefill: `--tm-root` to prefill msgstr and mark entries as `fuzzy` (#PR)
- [cli] Localized help for `--tm-root` and TM coverage summary in export output (#PR)
- [cli] New `diff-xml` command: compares source vs translation presence and, with a baseline PO, detects changed source strings; supports text/json and writing ChangedData.txt/TranslationData.txt/ModData.txt (#PR)
- [cli] New `annotate` command: add/remove source-text comments in translation XML; supports dry-run and backups (#PR)
- [cli] New `xml-health` command: scans XML files under Languages/ for structural/read errors (text/json) (#PR)
- [cli] New `init` command: generate translation skeleton under `Languages/<target>` with empty values (text/dry-run/overwrite) (#PR)
- [services/plugins] Built-in ModSettingsFramework plugin: extracts `<id>` label/tooltip from `Patches/` operations into Keyed units (#PR)
- [gui] Scan table: toggle to show only entries from `Patches/` (#PR)
- [gui] Preview TM: load Baseline PO + TM folders, show suggestion and quick insert into target field (#PR)
- [plugins] JSON/YAML keyed scanners: flatten string/object/array-of-strings into dotted keys (#PR)
 - [cli/scan] Flags for PatchOperations/plugins: `--with-patches`, `--patch-min-len`, `--patch-strict-xpath`, and `--with-plugins` (#PR)
 - [cli] New `learn-defs` and `learn-keyed` commands: suggest DefInjected fields and Keyed keys; produce learned datasets/reports (#PR)
 - [cli] New `morph` command: generate Case/Plural/Gender via Morpher API or local pymorphy2; supports filters/limits/timeouts and caching (#PR)
 - [cli] New `schema` command: dump JSON Schemas for domain types (#PR)
 - [gui] New panels: Learn DefInjected, Learn Keyed, Learn Patches, Morph, and JSON Schemas (#PR)
 - [gui] Structured log viewer with filters (source/level/text) and JSONL export; Debug Console replaces legacy modal (#PR)
 - [tauri] Backend commands: `validate_po_gui`, `learn_patches_cmd`; expose CLI i18n to GUI (#PR)

### Changed
- [gui] Project state is tracked honestly: external changes to project files are detected, unsaved drafts survive a refresh and the data-mode badge reflects the real state of a live project.
- [services/scan] Optionally merge fuzzy candidates; deterministic sort preserved.
- [cli/scan] `--lang` now filters the scan to a single `Languages/<dir>` (English additionally includes Defs-derived strings); previously the flag only affected the CSV column, so mods with several language folders produced mixed, colliding results.
- [cli/validate] Per-row validation now targets the translation (`--lang-dir`, `--lang`, or config `target_lang`) instead of silently validating the English source picked up from config defaults.
- [parsers-xml] Support path markers in dict (`li{h}`) to hint pseudo-handles for list segments; markers are ignored during value traversal and stripped from produced keys (#PR)
- [services/learn] Generate EN: comments alongside suggested DefInjected entries to help translators (#PR)
- [parsers-xml/assets] Expand `defs_fields.json` with common fields from Core/popular mods; add handle-hinted list paths (e.g., `ingredients.li{h}.label`, `degreeDatas.li{h}.description`, `SoundDef.subSounds.li{h}.name`) (#PR)
 - [parsers-xml] Defs DSL: support simple attribute predicates in selectors (e.g., `node[@attr='value']`) (#PR)
- [cli/scan] Add `--no-inherit` to disable ParentName inheritance when scanning Defs (for strict modes) (#PR)
- [parsers-xml] Parallelize Keyed/DefInjected scan (env `RIMLOC_PARALLEL=1` or `--parallel`); deterministic order preserved (#PR)
 - [parsers-xml] Parallelize Defs scan when `RIMLOC_PARALLEL=1` (deterministic order preserved) (#PR)
- [validate] Report cross-file duplicates as `duplicate-global` with file list summary (#PR)
- [validate] Warn on invisible/bi-di control characters in values (e.g., ZWSP/LRM/RLM/RLO/FSI) (#PR)
 - [cli/scan] Experimental: treat nested keyed elements as dotted keys via `--keyed-nested` (#PR)
 - [gui/ux] Quality-of-life: default export path, single‑file import output picker, auto‑fill paths and sane defaults on root change; layout tweaks (grid 260px) (#PR)
 - [gui/i18n] Remove hardcoded UI strings; centralize i18n (EN/RU) and add simple i18n linter; localize placeholders and common labels (#PR)
 - [gui/tauri] Validate/Diff panels support extended inputs (`defs_dict`, `defs_type_schema`, extra fields); mirror CLI outputs and report saving (#PR)

### Fixed
- [contribution] Safety hardening confirmed by an independent review: stale bundles can no longer overwrite fresh manual edits (mandatory `base_value` with value-conflict refusal); prototype-property ids (`constructor`, `__proto__`) are machine-refused instead of crashing; secret-like contributor metadata blocks the bundle; duplicate ids are refused instead of last-write-wins.
- [gui] Honesty wave: fake-success statuses replaced with honest "not wired in this build" notices (update check, rescan, log/config buttons, provider connectivity, demo build actions); the review overview in live mode now computes from the real snapshot and marks non-computable dimensions as dashes instead of plausible zeros; the logs hint shows the real per-OS path.
- [gui/legacy] Legacy write commands refuse CWD-relative output paths (previously wrote silently relative to the app's working directory); `apply_translation` gained the locale form guard.

- [gui/tauri] LEGACY surface (`RIMLOC_LEGACY_COMMANDS=1`, operator opt-in) no longer writes to caller-RELATIVE paths that silently landed relative to the process working directory: `merge_keyed_gui`, `export_xliff_gui`, `import_xliff_gui`, `dump_schemas` and `learn_patches_cmd` now require absolute out paths with an explicit error instead (same form policy as the contract surface). Legacy `apply_translation` validates the language-folder form of `lang_dir` before joining it into the output path, so traversal or absolute values can no longer escape the mod tree.
- [release] Workspace MSRV declaration corrected from `1.70` (false — it predated `is_multiple_of` 1.87 and dependency floors up to 1.89) to `1.89` in every crate manifest; tested toolchain is `1.96.0`.
- [gui/services] Contract export and diagnostics refuse a RELATIVE output directory with a typed `invalid_output_path` error before anything is written (a relative path silently landed relative to the app's working directory); the GUI out-dir fields now start empty with an absolute-path hint instead of pre-filling a decorative `…/RimLoc-Export/…` literal, and the run buttons stay disabled until the path is absolute.
- [cli] Language-folder flags (`init --lang-dir`, `import-po --lang-dir`, `build-mod --lang`, `merge-keyed`, `morph`) reject absolute paths and `..` values that would write outside the mod folder.
- [scan] A mod's LoadFolders entries can no longer make RimLoc scan directories outside the mod root (absolute paths and `..` are refused).
- [export] Control characters that XML 1.0 forbids are refused when applying or exporting, instead of producing files the game silently drops; a byte-level check guards every written `.xml`.
- [export] About.xml is written safely: fields are escaped and the packageId becomes a clean lowercase slug, so mod folders with `&`, `<` or non-Latin names no longer break the game's mod parsing.
- [gui] A session now survives an app restart with its source mod folder intact — previously export and diagnostics refused until the project was re-created.
- [cli] `scan` and `validate` fail with a clear message when `--root` points to a missing folder, instead of quietly returning an empty success.
- [gui] Corrupt project files are reported as unopenable (with a reason) instead of silently disappearing, and project-list errors are visible instead of looking like "no projects yet".
- [validate] Invisible or bi-directional control characters in translation keys now raise a warning (previously only values were checked).
- [services/project] Pack/PO import matching is scope-aware (kind + def type from the pack's own folder); a Keyed line now imports as the Keyed entry, and alias/TKey-suffix proofs never cross kinds or def types. `detect_source_changes` compares full identities and keeps the RESOLVED version on rescanned entries.
- [parsers-xml] TKey duplicate rejection is def-type-scoped: two def types sharing defName+TKey are distinct identities, not duplicates.
- [services/scan] DefInjected precedence is def-type-scoped: two def types sharing `{defName}.{field}` no longer destroy each other at unit level (canonical kind+key collapse stays a documented limitation).
- [services/scan] LoadFolders effective scan no longer drops same-file Keyed duplicates (kept as Gate H diagnostics / Overridden contexts).
- [services/project] `build_project` no longer fakes patch coverage from "a Patches dir exists" and no longer asserts EXACT with unresolved `IfModActive` dirs or partial patch coverage; `version_selected` records the resolved, not the requested, version.
- [gui/security] Hardened the Tauri trust boundary: Content-Security-Policy replaces `csp: null`; arbitrary file writes (`save_text_file`, diagnostics out-path) now go through a native save dialog opened on the Rust side; dynamic plugin loading is opt-in via `~/.rimloc/plugins-allow.json`; `open_path` uses the platform open API instead of a shell interpreter; removed the unused shell plugin and asset protocol.
- [services/lang-update] Fixed zip-slip: archive entry names are sanitized before extraction (rejects `..`, absolute paths, backslashes); malformed archives return an error instead of panicking.
- [services/validate] `duplicate-global` no longer reports the same key in different language folders (a translated Def is not a duplicate); duplicates are scoped per language folder.
- [services/validate] `coverage` and cross-language checks accept full paths for language directory arguments (previously compared against bare folder names, so coverage always reported 0).
- [cli/i18n] Removed duplicate Fluent key `diffxml-summary` that logged an ERROR on every CLI launch.
- [repo] Added a CI workflow running fmt, clippy (-D warnings), workspace tests on Linux/macOS/Windows, a Tauri GUI build, and cargo-deny; license fields added to crates missing them; deny.toml with advisories/license policy (transitive unmaintained deps pinned by tauri documented as ignored).
- [parsers-xml] Handle self-closing keyed XML elements correctly (#PR)
- [services] diff-xml baseline: honor msgctxt key extraction when computing changed entries (#PR)
- [parsers-xml] Aggregate <li> list items and <LineBreak/> into a single value for LanguageData keys; improves DefInjected/Keyed lists handling (#PR)
- [services/merge-keyed] Read <li> and <LineBreak/> correctly when merging English→target; preserves multi-line values and list semantics (#PR)
- [parsers-xml] Resolve Defs inheritance across files via Name/ParentName (fallback to defName), respect Inherit="false"; improves DefInjected candidates discovery (#PR)
 - [cli/logging] Validate `RIMLOC_LOG_DIR` against traversal/absolute paths; reject unsafe values (#PR)
 - [gui] Lang update: resolve macOS .app bundle to `Resources`; enforce Accept: zip and header check for downloads (#PR)
 - [gui/frontend] Use backend `open_path` to avoid plugin-shell URL regex warnings; fix save-report wrappers (#PR)

### Docs
- Release-candidate docs: RELEASE_READINESS_REPORT (gate matrix, honest limitations, owner gates), BETA_TEST_CHECKLIST for testers, REVIEW_SCREEN_MAP for GUI review.
- README: honest project status, screenshots section and a GUI build guide; real GUI screenshots added.
- AGENTS: add rule to reply in Russian when addressed in Russian (#PR)
- README: replace AGENTS.md link with CONTRIBUTING.md (#PR)
- AGENTS: make commit via scripts/agent-commit.sh a mandatory finish step for agents (#PR)
- AGENTS: explicitly allow using GH_TOKEN/GITHUB_TOKEN when provided by the user, with safety rules (#PR)
- AGENTS: add final guard step with scripts/agent-ensure-commit.sh (#PR)
 - [docs/schemas] Refresh generated JSON Schemas via `rimloc-cli schema`; document `out_dir` in configuration guide (#PR)
 - [docs/gui] Expand GUI guide with Morph and Tools (schemas) sections (#PR)

### Internal
- [gui/i18n] Build-time JSON bridge for the self-localization foundation: `npm run export:catalog` deterministically exports the UI catalog (`src/i18n/{en,ru}.ts`) into committed versioned JSON (`src/i18n/generated/catalog.{en,ru,meta}.json`, schema_version 1, git-revision provenance, `{name}` placeholder contract); drift-guard tests keep TS as the single hand-edited authority. See `docs/development/SELFLOC_BRIDGE.md`.

## [0.1.0-alpha.1] - 2025-09-25
### Added
- rimloc-cli initial prerelease: scan, validate, export-po, import-po, build-mod
- i18n (EN/RU), colored logs, JSON output and --quiet mode
- Dev release automation, artifact signing (cosign) and SBOM (Syft)

### Docs
- Install page (EN/RU), Support page with BMC/Ko-fi and crypto addresses
- Discord invite and badges

## [0.1.0] - 2025-09-25
### Added
- rimloc-core: TransUnit/PoEntry, minimal PO parser
- rimloc-parsers-xml: scan Keyed XML → TransUnit
- rimloc-export-csv: CSV exporter with optional lang column
- rimloc-export-po: PO exporter with msgctxt and references
- rimloc-import-po: PO reader and LanguageData XML writer
- rimloc-validate: empty/duplicate/placeholder checks
 
<!-- Links -->
[Unreleased]: https://github.com/0-danielviktorovich-0/RimLoc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/0-danielviktorovich-0/RimLoc/compare/v0.1.0-alpha.1...v0.1.0
[0.1.0-alpha.1]: https://github.com/0-danielviktorovich-0/RimLoc/releases/tag/v0.1.0-alpha.1
