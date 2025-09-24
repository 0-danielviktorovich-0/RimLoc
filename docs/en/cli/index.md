---
title: CLI Commands
---

# CLI Commands Overview

RimLoc CLI bundles everything needed to inventory, validate, and exchange RimWorld translations. Commands emit consistent diagnostics and exit codes, so you can script them or wire them into CI.

## Before you begin

- Install the CLI with `cargo install rimloc-cli`.
- Work on a clean copy of your mod; every command reads or writes within `Languages/`.
- Use the bundled `test/TestMod` fixture to rehearse the workflow before touching production data.

## Typical workflow

1. **Scan** your mod to extract translation units.
2. **Validate** XML to catch duplicates, empty values, and placeholder drift.
3. **Export PO/CSV** packages for translators or CAT tools.
4. **Review translations** and run **Validate PO** to ensure placeholders match.
5. **Import PO** back into XML, then run **Validate** again before shipping.

## Command summary

| Command | Purpose | Frequent options |
|---------|---------|------------------|
| [`scan`](scan.md) | Harvest translation units from XML. | `--lang`, `--format`, `--out-csv` |
| [`validate`](validate.md) | QA check XML for duplicates, empties, placeholders. | `--format`, `--ui-lang` |
| [`validate-po`](validate_po.md) | Compare placeholders inside PO files. | `--po`, `--strict`, `--format` |
| [`export-po`](export_import.md#export-po) | Produce PO/CSV bundles for translators. | `--out`, `--single-po`, `--dry-run` |
| [`import-po`](export_import.md#import-po) | Apply PO changes back to XML. | `--po`, `--out`, `--dry-run` |

## Helpful patterns

```bash
# Run validation in CI and fail on errors only
rimloc-cli validate --root ./path/to/mod --format text

# Emit machine-readable diagnostics
rimloc-cli validate --root ./path/to/mod --format json | jq '.[] | select(.level=="error")'

# Export and immediately validate resulting PO placeholders
rimloc-cli export-po --root ./path/to/mod --single-po --out ./out/mymod.po
rimloc-cli validate-po --po ./out/mymod.po --strict
```

## Troubleshooting

- **Unexpected `/RimLoc/` prefixes in docs or examples** – clear `SITE_URL` locally; set it only in CI before `mkdocs build`.
- **`placeholder-check` errors** – compare source vs. translation placeholders; `--format json` highlights the offending key.
- **Nothing exported/imported** – verify `Languages/<lang>/` exists and matches the language codes you pass in.

Need deeper details? Each command page lists full option tables, examples, and command-specific troubleshooting tips.
