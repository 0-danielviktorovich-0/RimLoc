

# RimLoc

Toolkit for working with RimWorld translations.

**What it does**

- Scan RimWorld XML (`Keyed/DefInjected`) and produce translation units.
- Validate translations: duplicate keys, empty strings, placeholder mismatches, etc.
- Export translations to **PO** / **CSV**.
- Import translations from **PO** into a target mod (supports a dry‑run preview).
- All CLI output is localized via **Fluent**; resources are embedded with **rust-embed**.

---

## Quick start

```bash
# Build the CLI
cargo build -p rimloc-cli

# See available commands
cargo run -p rimloc-cli -- --help
```

### Typical commands

```bash
# Scan XML and print CSV to stdout
cargo run -p rimloc-cli -- scan --xml-root ./test/TestMod

# Validate a mod's XML (shows categories like duplicate/empty/placeholder-check)
cargo run -p rimloc-cli -- validate --xml-root ./test/TestMod

# Export PO from XML
cargo run -p rimloc-cli -- export-po --xml-root ./test/TestMod --out ./test/out.po

# Import PO into a target mod (dry run)
cargo run -p rimloc-cli -- import-po --po ./test/ok.po --target ./test/TestMod --dry-run
```

### UI language

CLI messages are localized with **Fluent** and bundled in the binary.

- Default locale: auto-detected from the OS.
- Override for the current run:
  ```bash
  RIMLOC_UI_LANG=en cargo run -p rimloc-cli -- --help
  ```
- If your terminal has issues with ANSI/emoji, you can disable colors:
  ```bash
  cargo run -p rimloc-cli -- --no-color --help
  ```

---

## i18n stack (what’s used)

- **Fluent** (`.ftl`) – message format and grammar.
- **i18n-embed** + **i18n-embed-fl** – runtime loading and the `fl!` macro.
- **rust-embed** – embeds `.ftl` files into the CLI binary.
- Tests ensure:
  - All locales have the same keys and order as `en`.
  - No user-facing strings are hardcoded in the codebase (even in tests).
  - CLI help and diagnostics are localized.

Translation resources live in:

```
crates/rimloc-cli/i18n/<lang>/rimloc.ftl
crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl
```

---

## Development

Requirements: latest stable Rust toolchain.

Format, lint, run tests:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p rimloc-cli --test cli_integration
```

Useful one‑offs:

```bash
# Run a single integration test with output
cargo test -p rimloc-cli --test cli_integration -- --nocapture validate_detects_issues_in_bad_xml
```

**Policy:** Any code that prints or otherwise shows messages to the user **must** go through i18n keys (Fluent). No hardcoded user strings.

---

## Repository layout (excerpt)

```
crates/
  rimloc-cli/         # CLI binary with i18n resources
  rimloc-core/        # core types & helpers reused across crates
  rimloc-parsers-xml/ # XML parsing utilities
  rimloc-export-po/   # PO export
  rimloc-export-csv/  # CSV export
  rimloc-import-po/   # PO import
  rimloc-validate/    # validation logic reused by CLI
test/                 # sample XML/PO data used by tests
gui/                  # placeholder for future Tauri GUI
```

---

## Contributing

PRs are welcome. Please keep i18n rules intact (no hardcoded UI strings), and run `fmt`, `clippy`, and tests before submitting.

---

## License

GNU GPL — see `LICENSE`.