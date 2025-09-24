---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/)

RimLoc helps RimWorld modders keep translations discoverable, validated, and ready for translators.

## Why RimLoc?

- Inventory every string under `Languages/*/{Keyed,DefInjected}` with one command.
- Prevent broken releases by catching duplicate keys, empty values, and placeholder drift.
- Export and import PO/CSV bundles so translators can work with familiar tooling.
- Ship CLIs localized via Fluent (English and Russian included by default).

## Quick start

```bash
cargo install rimloc-cli
rimloc-cli scan --root ./test/TestMod --format json | jq '.[0]'
rimloc-cli validate --root ./test/TestMod
```

- `scan` collects translation units and prints CSV (or JSON with `--format json`).
- `validate` performs QA checks and exits with `1` if it finds errors.
- Use the bundled `test/TestMod` fixture to experiment before touching your mod.

Need to export for translators?

```bash
rimloc-cli export-po --root ./test/TestMod --out ./logs/TestMod.po --single-po
```

## Core commands

| Command | What it does | Notes |
|---------|---------------|-------|
| `scan` | Enumerates translation units from XML. | Add `--out-csv` to save alongside stdout. |
| `validate` | Flags duplicates, empties, placeholders in XML. | Combine with `--format json` for CI parsing. |
| `validate-po` | Compares placeholders in PO `msgid`/`msgstr`. | Use `--strict` to fail on warnings. |
| `export-po` | Publishes PO/CSV packages. | `--single-po` keeps everything in one file. |
| `import-po` | Writes PO updates back into XML. | `--dry-run` previews changes without touching files. |

## Next steps

- Read the [CLI overview](cli/index.md) for command-specific options and examples.
- Jump directly to: [Scan](cli/scan.md) · [Validate](cli/validate.md) · [Validate PO](cli/validate_po.md) · [Export / Import](cli/export_import.md)
- Update docs locally with `mkdocs serve` and edit the files under `docs/en/` and `docs/ru/`.

!!! tip "Looking for the CLI source?"
    The binaries live in `crates/rimloc-cli`. Fixtures for experimenting are under `test/`.

## Contributing to docs

Found a typo or want to add examples? [Edit this page on GitHub](https://github.com/0-danielviktorovich-0/RimLoc/tree/main/docs/en/index.md) or check the contributor guide in [AGENTS.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md).
