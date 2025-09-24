---
title: Testing & Reporting
---

# Testing & Reporting

This page explains how to test RimLoc locally and how to file actionable bug reports with the right diagnostics.

## Quick Start (Developers)

```bash
# Build the entire workspace
cargo build --workspace

# Run all tests (unit + integration)
cargo test --workspace -- --nocapture

# Ensure formatting and lint cleanliness before review
cargo fmt && cargo clippy --workspace --all-targets -- -D warnings
```

Useful flags:

- `-- --nocapture` shows test stdout (handy for CLI help localization).
- Run a single integration test: `cargo test -p rimloc-cli scan_picks_latest_version_by_default_and_flags_work`.

## Logging & Diagnostics

RimLoc emits diagnostics to stderr and to a rolling log file.

- `RUST_LOG=info|debug|trace` — controls console verbosity (default: `info`).
- `RIMLOC_LOG_DIR=./logs` — directory for daily log files (default: `./logs`).
- `RIMLOC_LOG_FORMAT=json` — switch file logs to structured JSON (default: `text`).
- Disable UI decorations for clean copy/paste:
  - `NO_COLOR=1` — remove ANSI colors.
  - `NO_ICONS=1` — remove symbols like ✔/⚠/✖.

On startup, RimLoc prints a banner that includes version, `RIMLOC_LOG_DIR`, and the `RUST_LOG` level so bug reports capture the environment context.

Tip for automation: combine `--quiet` with `--format json` to keep stdout machine‑readable and route diagnostics to stderr/logs.

## End‑to‑End CLI Checks

Use the bundled fixture `test/TestMod` to exercise the CLI:

```bash
# Scan to JSON and keep a copy on disk
rimloc-cli scan --root ./test/TestMod --format json --out-json ./logs/scan.json

# Validate with text output
rimloc-cli validate --root ./test/TestMod --format text

# Validate PO placeholders strictly
rimloc-cli validate-po --po ./test/test-en.po --strict

# Export a PO file and import it back into XML in dry-run mode
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
rimloc-cli import-po --po ./logs/TestMod.po --mod-root ./test/TestMod --dry-run

# Build a translation-only mod (dry run)
rimloc-cli build-mod --po ./logs/TestMod.po --out-mod ./logs/TestMod-ru --lang ru --dry-run
```

### Versioned Mods

If your mod uses per‑version folders (`1.4`, `1.5`, `v1.6`):

```bash
rimloc-cli scan --root ./Mods/MyMod --game-version 1.4
rimloc-cli validate --root ./Mods/MyMod --include-all-versions
rimloc-cli export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --game-version v1.6
```

## JSON for Automation

- `scan --format json [--out-json <FILE>]` — prints an array of units; persist to a file for CI.
- `validate --format json` — emits structured issues (kind, key, path, line, message).
- `validate-po --format json [--strict]` — lists placeholder mismatches between msgid/msgstr.

Example:

```bash
rimloc-cli validate --root ./test/TestMod --format json | jq '.[] | select(.kind=="duplicate")'
```

## Bug Report Template

Please include the following for actionable reports:

1) Command and full invocation

```
rimloc-cli <command> <args>
```

2) Versions and environment

- `rimloc-cli --version`
- OS and shell
- `RUST_LOG`, `RIMLOC_LOG_DIR`, `NO_COLOR`, `NO_ICONS`

3) Expected vs actual behavior (1–2 sentences each)

4) Attachments

- `logs/rimloc.log` and console output (with `--ui-lang en` if possible)
- Minimal reproducible example: a small mod snippet or a couple of `Languages/...` XML files
- For PO‑related issues: a short `.po` that reproduces the behavior

## Docs Preview

To preview this documentation locally:

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve
```
