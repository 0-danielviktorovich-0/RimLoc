---
title: CLI Commands
---

# CLI Commands Overview

## Description

RimLoc CLI provides tools to scan, validate, and convert translations.

## Commands

| Command        | Description                                                      | Link                          |
|----------------|------------------------------------------------------------------|-------------------------------|
| Scan           | Extracts translation units from `Keyed/DefInjected` files.       | [scan](scan.md)               |
| Validate       | Checks for duplicates, empty strings, and placeholder issues.    | [validate](validate.md)       |
| Validate PO    | Compares placeholders between `msgid` and `msgstr`.              | [validate_po](validate_po.md) |
| Export & Import| Converts to/from PO/CSV formats.                                 | [export_import](export_import.md) |

---

## Examples

```bash
# Show help
cargo run -p rimloc-cli -- --help
```
Displays CLI help information.

```bash
# Scan (CSV by default)
cargo run -p rimloc-cli -- scan --root ./test/TestMod
```
Scans the specified mod directory and extracts translation units.

```bash
# Validate (JSON)
cargo run -p rimloc-cli -- validate --root ./test/TestMod --format json | jq .
```
Validates translations and outputs results in JSON format. For readability, the output is piped through `jq`.

```bash
# Strict PO validation (exit code 1 on mismatch)
cargo run -p rimloc-cli -- validate-po --po ./test/bad.po --strict
```
Validates a PO file in strict mode; returns exit code 1 if mismatches are found.

---

## See also

- [Scan](scan.md)
- [Validate](validate.md)
- [Validate PO](validate_po.md)
- [Export / Import](export_import.md)