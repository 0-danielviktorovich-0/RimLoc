---
title: Developer Guide
---

# Developer Guide

Everything you need to build, test, and debug RimLoc locally.

## Supported OS and toolchain

- Recommended: Linux or macOS (Rust stable).
- Windows: works with MSVC toolchain; WSL2 (Ubuntu) is recommended for a smoother UNIX‑like workflow.
- Install Rust via rustup:

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows (PowerShell): download rustup-init.exe from https://rustup.rs
```

Verify:

```bash
rustc -V
cargo -V
```

Optional tools:

- VS Code + rust‑analyzer
- `cargo install cargo-watch`
- Python 3 + `pip` (for docs)

## Build & test

```bash
cargo build --workspace
cargo test --workspace
cargo fmt && cargo clippy --workspace --all-targets -- -D warnings
```

Run the CLI against a sample mod:

```bash
cargo run -q -p rimloc-cli -- --quiet scan --root ./test/TestMod --format json | jq '.[0]'
```

## Environment for debugging

- Logging:
  - `RUST_LOG=debug` (console to stderr)
  - `RIMLOC_LOG_DIR=./logs` (file log; daily rotation)
  - `RIMLOC_LOG_FORMAT=json` for structured file logs
  - `NO_COLOR=1`, `NO_ICONS=1` for plain text
  - `--quiet` keeps stdout clean for JSON output

Examples:

```bash
RUST_LOG=debug RIMLOC_LOG_DIR=./logs cargo run -q -p rimloc-cli -- --quiet validate --root ./test/TestMod --format json | jq .
```

Backtraces and rich errors:

```bash
RUST_BACKTRACE=1 cargo run -p rimloc-cli -- validate --root ./test/TestMod
```

## Debugging with LLDB/GDB (optional)

```bash
# lldb
rusr-lldb target/debug/rimloc-cli -- --quiet scan --root ./test/TestMod

# gdb
rust-gdb target/debug/rimloc-cli --args --quiet scan --root ./test/TestMod
```

## Running docs locally

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve
```

## i18n (localization)

- CLI strings live in `crates/rimloc-cli/i18n/en/rimloc.ftl` and mirrors per locale.
- See Community → Localization and Translate RimLoc for adding a new language.
- Placeholders: keep `{name}`, `{0}`, `%s` intact — see Guides → Placeholders.

## Typical workflows

### Export → translate → import

```bash
# export PO
rimloc-cli --quiet export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --lang ru
# validate placeholders
rimloc-cli --quiet validate-po --po ./out/MyMod.po --strict
# import back (single file or full structure)
rimloc-cli --quiet import-po --po ./out/MyMod.po --out-xml ./out/MyMod.ru.xml
```

### Build a translation‑only mod

```bash
rimloc-cli --quiet build-mod --po ./out/MyMod.po --out-mod ./dist/MyMod-ru --lang ru --dedupe
```

