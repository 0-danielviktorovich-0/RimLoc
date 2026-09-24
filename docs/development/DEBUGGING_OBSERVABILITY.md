# Debugging & Observability (gate L)

Status: implemented (backend lane, L) · 2026-09-25
Scope: `crates/rimloc-services/src/observability.rs`, `crates/rimloc-services/src/util.rs`
(shared boundary helpers), `crates/rimloc-cli/src/commands/doctor.rs`.
Acceptance bar (AUTONOMOUS_STATUS.md §W4.5 п.5-6): diagnostics value is
**root-cause usefulness** (a controlled known failure → causal context →
sanitized bundle → an independent reviewer/AI can name the likely cause), not
"bundle generated". A small relevant causal trace beats raw logs.

## `rimloc doctor`

```bash
rimloc doctor --game-root "/path/to/RimWorld" --root /path/to/MyMod --out-dir . [--format json]
```

Checks and statuses (`ok` / `warning` / `error` / `not_checked`):

| Check | Needs | Meaning |
|---|---|---|
| `rw_install` | `--game-root` | `Data/` and `Mods/` exist under the game root |
| `rw_version` | `--game-root` | `Version.txt` readable → reported game version |
| `mod_dirs` | `--game-root` | `Mods/` listable, N mod directories |
| `mod_root` | `--root` (optional) | mod root exists, `Languages/` folders listable |
| `provider_config` | — | env presence of `RIMLOC_ANTHROPIC_API_KEY` / `RIMLOC_OPENAI_API_KEY` / `RIMLOC_ZAI_API_KEY`. **Presence only — values are never read or printed.** Keychain (service `rimloc-llm`) is not probed by the CLI build (no `keychain` feature) and the output says so |
| `output_writable` | `--out-dir` (default `.`) | refuses destinations inside the game/mod trees (canonical view, symlink aliases + parent traversal included) BEFORE mkdir; then a unique-name `create_new` probe (an existing file can never be truncated or followed through a symlink) |

Missing inputs produce `not_checked` with a remediation hint, not an error.
Exit code is always 0: doctor is a report, not a gate. `--format json`
emits `{"checks": [...], "summary": {ok, warning, error, not_checked}}`.
User-facing strings go through FTL (`doctor-*` keys in `i18n/{en,ru}`) and
the `ui_*` macros; the project guard
`no_hardcoded_user_strings_anywhere` enforces this.

## Shared boundary helpers (`services::util`)

- `canonical_view(path) -> io::Result<PathBuf>` — real symlink resolution:
  `std::fs::canonicalize` short-circuits existing paths; otherwise each
  existing component is checked with `symlink_metadata`, targets are
  resolved iteratively, `..` applies to the resolved prefix, Windows
  drive/UNC prefixes are preserved verbatim. Only `NotFound` is a benign
  "non-existing tail"; any other IO error fails closed with `Err`.
- `is_within(candidate, root) -> bool` — equal-or-nested containment on
  canonical views. **Fail-closed**: an unresolvable candidate counts as
  contained so write guards reject instead of guessing.

Used by both `rimloc doctor` (output destination) and the support-bundle
collector (bundle output vs scanned source). Windows drive/UNC
discrimination has focused `#[cfg(windows)]` regressions — they compile
everywhere but only RUN on a Windows host.

## Operation log (`services::observability::OperationLog`)

Structured journal for one operation: `operation_id`, stages with
started/finished timestamps and `duration_ms`, named counters per stage,
and errors with their full `source()` chain. IDs are generated without new
dependencies: `op-<hex millis>-<counter>-<pid>` (sortable, unique per
process). Lifecycle is honest: `end_stage` only closes a stage;
`finished_at` / `total_duration_ms` are set exclusively by `finish()`, and
`begin_stage` after `finish` re-opens the operation. A serialized log
without `finish` reports no `finished_at` — a crashed operation stays
looking like one.

## Sanitizer (`services::observability::Sanitizer`)

One composable text pipeline (`sanitize_text`): mask secret fragments
inside free text **and** normalize absolute home paths to `~`, both applied
in sequence (compound values get both transforms). Whole-value secrets are
`Redacted` (`[REDACTED]`, key kept). Decisions:

- **Key name** contains a credential word (`key`, `token`, `secret`,
  `password`, `authorization`; plus `auth`, `credential` for auth headers)
  → `Redacted`.
- **Value looks secret** (`sk-…`, `Bearer …`, PEM, `ghp_`/`github_pat_`/
  `xox*-`/`AKIA`, ≥32-char hex, ≥40-char opaque tokens) → `Redacted`
  regardless of key name.
- **Secret inside free text** (`Authorization: Bearer x`, `apiKey=…`
  mid-string) → kept with the token masked in place; surrounding
  diagnostic context survives.
- **Home paths** (`HOME`/`USERPROFILE`) anywhere in a value are rewritten
  to `~` (boundary-safe, PATH-like lists included).
- **Env vars**: a small explicit allowlist only (`LANG`, `LC_ALL`, `TERM`,
  `TMPDIR`); everything else — including `USER`/`USERNAME` and the whole
  `RIMLOC_*`/`CARGO_*`/`RUST_*` families — is `Excluded`, with the
  exclusion kept visible in the preview.

Preview before saving: `classify(&fields) -> ClassifyReport { included,
redacted, excluded }`, where `included` carries the exact values that would
be written. `sanitize_json(&Value)` applies the same rules to arbitrary
(nested) JSON, including array leaves (`log[1]` style paths in the report)
— so adversarial metadata (`apiKey = "sk-…"`, compound path+token text)
never reaches a file.

## Support bundle

```rust
use rimloc_services::{collect_support_bundle_for, SupportBundleInputs, ProjectMeta};
let bundle = collect_support_bundle_for(
    &SupportBundleInputs {
        scan_root,
        project_meta,
        // The operation being diagnosed (typically the FAILED one) is
        // preserved verbatim: its id, stages, timings, errors and causal
        // chains — never replaced by a synthetic success.
        operation: Some(failed_log),
        // Affected entry ids; sanitized before writing.
        affected: vec![...],
    },
    &out_dir, // must NOT be inside scan_root (canonical containment, fail-closed)
)?;
```

Files, all via the shared symlink-safe `write_atomic`:

- `report.md` — human summary: diagnosed operation id + "NOT finished"
  state, stage table, errors with causal chains, affected entries, counts
  (`xml_files`, `languages`, `units_scanned`), redaction summary (incl.
  excluded env var names), sanitized environment.
- `diagnostics.json` — `operation` (the preserved failing log, sanitized),
  `collection` (collector's own complete log), `counts`, `affected`,
  `project_meta` (every field sanitized).
- `environment.json` — built raw, sanitized **once as a whole payload**
  (versions included): OS/arch, RimLoc/RW versions, locales, allowlisted
  env vars, `~`-rewritten scan root.
- `manifest.json` — sha256 + size of the three payload files (standard
  `sha2` crate, already in the dependency tree).

Boundary guard: `collect_support_bundle_for/in` rejects an output folder
equal-or-nested inside `scan_root` (symlink aliases and parent-traversing
non-existing paths included) BEFORE any mkdir/write; the default-location
`collect_support_bundle` refuses to run when the current directory is
inside the source. Scanning problems are recorded in the log as errors —
the bundle still builds, because a failure context is exactly when you need
it.

Regressions (`observability.rs` tests): adversarial metadata with
compound path+token values asserting zero synthetic keys and zero home
prefixes across ALL four output files; a controlled validator failure whose
operation id, stage, counters, causal chain and unfinished state survive
into the bundle (an independent reader can name the cause from `report.md`
alone); sibling-traversal, equal, symlink-alias and recursive
sentinel/hash invariance for rejected destinations.

## Deliberately deferred

- CLI `rimloc bundle` command and Copy-for-AI — bundle is a services API
  consumed by the GUI diagnostics lane (W5) first; doctor covers CLI needs.
- Keychain probing from the CLI (requires enabling `rimloc-llm/keychain`).
- Wiring operation IDs through existing commands' logs (GUI binding work).
- Executing the `#[cfg(windows)]` prefix regressions on a Windows host.
