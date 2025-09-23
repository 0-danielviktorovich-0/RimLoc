---
title: Scan Command
---

# Scan Command

## Description

Scan a RimWorld mod directory to extract translation units from `Languages/*/{Keyed,DefInjected}` XML files. This command helps gather all localized strings for further processing or translation.

## Synopsis

```bash
rimloc-cli scan --root <PATH> [--out-csv <FILE>] [--lang <CODE>] \
                 [--source-lang <CODE>] [--source-lang-dir <DIR>] \
                 [--format <csv|json>]
```

## Options

| Option               | Description                                         | Required |
|----------------------|-----------------------------------------------------|----------|
| `-r, --root <PATH>`  | Path to the RimWorld mod root directory to scan. **Required**. | Yes      |
| `--out-csv <FILE>`   | Save extracted entries to a CSV file (includes header). | No       |
| `--lang <CODE>`      | Language code to scan (e.g. `en`, `ru`). If omitted, all languages are scanned. | No       |
| `--source-lang <CODE>` | Source language code for cross-checks (optional). | No       |
| `--source-lang-dir <DIR>` | Explicit path to source language directory (optional). | No       |
| `--format <csv\|json>` | Output format to stdout. Default is `csv`. | No       |

### `-r, --root <PATH>`
Path to the RimWorld mod root directory to scan. **Required**.

### `--out-csv <FILE>`
Save extracted entries to a CSV file (includes header).

### `--lang <CODE>`
Language code to scan (e.g. `en`, `ru`). If omitted, all languages are scanned.

### `--source-lang <CODE>`
Source language code for cross-checks (optional).

### `--source-lang-dir <DIR>`
Explicit path to source language directory (optional).

### `--format <csv|json>`
Output format to stdout. Default is `csv`.

Output formats:

- `csv` — prints CSV to stdout (use `--out-csv` to save to a file as well).  
- `json` — prints a JSON array of translation units to stdout.

Each translation unit has the following structure:
```json
{
  "path": "<file path>",
  "line": <line number>,
  "key": "<Keyed key>",
  "value": "<string value>"
}
```

## Examples

Extract all languages and print JSON output:
```bash
rimloc-cli scan --root ./test/TestMod --format json | jq .
```
*Scans all languages and outputs translation units as JSON.*

Scan only English and save results to CSV file:
```bash
rimloc-cli scan --root ./test/TestMod --lang en --out-csv out.csv
```
*Extracts English entries and writes them to `out.csv`.*

Print CSV output directly to stdout:
```bash
rimloc-cli scan --root ./test/TestMod
```
*Scans all languages and prints CSV data to the console.*

## See also

- [Validate](validate.md)
- [Validate PO](validate_po.md)
- [Export / Import](export_import.md)

!!! note
    All user-facing CLI output (help, messages) is localized via **Fluent**.