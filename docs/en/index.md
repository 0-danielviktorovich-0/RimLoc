---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/)

RimLoc is a toolkit for working with RimWorld translations.

- Scans XML (`Keyed/DefInjected`) and extracts translation units.
- Validation (duplicate keys, empty strings, placeholder mismatches).
- Export to **PO/CSV**, import **PO** back into the target mod (supports dry-run).
- All CLI output is localized via **Fluent**; resources are embedded using **rust-embed**.

## Documentation

Explore the CLI, starting with the [overview](cli/index.md) or jump straight in:

- [Scan XML](cli/scan.md)
- [Validate](cli/validate.md)
- [Validate PO](cli/validate_po.md)
- [Export / Import](cli/export_import.md)

!!! tip "Quick Start"
    ```bash
    cargo build -p rimloc-cli
    cargo run -p rimloc-cli -- --help
    ```

### Handy CLI snippets

```bash
# Scan a mod and print JSON
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json | jq .

# Validate a mod (human‑readable)
cargo run -p rimloc-cli -- validate --root ./test/TestMod

# Validate placeholders in PO (JSON)
cargo run -p rimloc-cli -- validate-po --po ./test/bad.po --format json | jq .
```

---
## Contributing to docs

Found a typo or want to add examples? [Edit this page on GitHub](https://github.com/0-danielviktorovich-0/RimLoc/tree/main/docs/en/index.md).
