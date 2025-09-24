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

## VS Code / VSCodium setup

VS Code and VSCodium (telemetry‑free build of VS Code) work equally well for Rust. Recommended extensions:

- rust‑analyzer (official Rust language support)
- CodeLLDB (debugger)
- Even Better TOML (Cargo.toml)
- Fluent (FTL) syntax highlight (e.g., "Fluent Support")

Place these files under `.vscode/` (VSCodium also reads them).

Ready‑made examples are included in the repo:

- `.vscode/tasks.example.json`
- `.vscode/launch.example.json`

Copy them to `.vscode/tasks.json` and `.vscode/launch.json` to enable.

Example `tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    { "label": "cargo build", "type": "shell", "command": "cargo build --workspace" },
    { "label": "cargo test",  "type": "shell", "command": "cargo test --workspace" },
    { "label": "cargo clippy", "type": "shell", "command": "cargo clippy --workspace --all-targets -- -D warnings" },
    { "label": "cargo fmt",    "type": "shell", "command": "cargo fmt" },
    { "label": "mkdocs serve", "type": "shell", "command": "python -m venv .venv && . .venv/bin/activate && pip install -r requirements-docs.txt && mkdocs serve" }
  ]
}
```

Example `launch.json` (debug `rimloc-cli`):

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug rimloc-cli (scan)",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/rimloc-cli",
      "args": ["--quiet", "scan", "--root", "${workspaceFolder}/test/TestMod", "--format", "json"],
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug", "RIMLOC_LOG_DIR": "${workspaceFolder}/logs" },
      "preLaunchTask": "cargo build"
    },
    {
      "name": "Debug rimloc-cli (validate)",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/rimloc-cli",
      "args": ["--quiet", "validate", "--root", "${workspaceFolder}/test/TestMod", "--format", "text"],
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug", "RIMLOC_LOG_DIR": "${workspaceFolder}/logs" },
      "preLaunchTask": "cargo build"
    }
  ]
}
```

Tips:

- Add a "cargo test" compound task or a test launch with `program`: `${workspaceFolder}/target/debug/rimloc-cli-<hash>` if you debug test binaries.
- VSCodium users can reuse the same `.vscode/` folder.

OS note: Linux and macOS generally provide a smoother Rust developer experience (tooling parity, perf). On Windows, WSL2 is recommended if you prefer a UNIX‑like environment.

## Profiling tips

For quick CPU flamegraphs:

```bash
cargo install flamegraph
# Linux needs `perf` (sudo apt install linux-tools-...)
# macOS needs dtrace (run as root) or use Instruments

cargo flamegraph -p rimloc-cli -- --quiet scan --root ./test/TestMod --format json
```

General tips:

- Profile release builds: `cargo build --release`.
- Narrow down workloads to a single subcommand (e.g., `scan` on a bigger mod).
- Use `tracing` spans (already enabled) + `RUST_LOG=debug` to correlate hot paths with logs.

### Windows profiling (WPA/ETW)

Windows does not support `perf`/`dtrace` natively, but you can record ETW traces and analyze them:

- Install Windows Performance Toolkit (WPT) via the Windows 10/11 SDK installer (select “Windows Performance Toolkit”).

Record from a terminal:

```powershell
# Start a lightweight CPU profile
wpr -start CPU -filemode

# Run the workload in another terminal
runscript: cargo run -q -p rimloc-cli -- --quiet scan --root .\test\TestMod --format json > $null

# Stop recording and write the trace
wpr -stop rimloc.etl
```

Open `rimloc.etl` in Windows Performance Analyzer (WPA), inspect CPU Usage (Sampled) and call stacks.

Alternative: PerfView (https://github.com/microsoft/perfview)

```powershell
PerfView.exe run /NoGui /AcceptEULA -- cargo run -p rimloc-cli -- --quiet validate --root .\test\TestMod --format text
```

WSL2 option: run Linux tools (`perf`, `cargo flamegraph`) inside WSL2 and point to the same repo path.

## Debugging test binaries in VS Code/VSCodium

CodeLLDB can launch tests directly via Cargo. Example `launch.json` config to debug a specific test:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug test: cli_integration::validate_json_emits_structured_issues",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["test", "--no-run", "--package", "rimloc-cli", "--test", "cli_integration"],
        "filter": { "name": "cli_integration", "kind": "test" }
      },
      "args": ["--nocapture", "validate_json_emits_structured_issues"],
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug" },
      "console": "integratedTerminal"
    }
  ]
}
```

If your CodeLLDB version does not support the `cargo` launcher, build tests first and point `program` to the test binary under `target/debug/deps/` (hash suffix changes per build):

```bash
cargo test -p rimloc-cli --test cli_integration --no-run
ls target/debug/deps/cli_integration-*
```

Then set `program` to that path and use `args`: `["--nocapture", "validate_json_emits_structured_issues"]`.
