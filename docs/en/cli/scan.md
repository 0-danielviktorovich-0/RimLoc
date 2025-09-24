---
title: Scan Command
---

# Scan Command

## Description

Scan a RimWorld mod directory to extract translation units from `Languages/*/{Keyed,DefInjected}` XML files. This command helps gather all localized strings for further processing or translation.

## Synopsis

```bash
rimloc-cli scan --root <PATH> [--out-csv <FILE>] [--out-json <FILE>] [--lang <CODE>] \
                 [--source-lang <CODE>] [--source-lang-dir <DIR>] \
                 [--format <csv|json>]
```

## Options

| Option               | Description                                         | Required |
|----------------------|-----------------------------------------------------|----------|
| `-r, --root <PATH>`  | Path to the RimWorld mod root directory to scan. **Required**. | Yes      |
| `--out-csv <FILE>`   | Save extracted entries to a CSV file (includes header). | No       |
| `--out-json <FILE>`  | Save extracted entries to a JSON file (requires `--format json`). | No       |
| `--lang <CODE>`      | Language code to scan (e.g. `en`, `ru`). If omitted, all languages are scanned. | No       |
| `--source-lang <CODE>` | Source language code for cross-checks (optional). | No       |
| `--source-lang-dir <DIR>` | Explicit path to source language directory (optional). | No       |
| `--format <csv\|json>` | Output format to stdout. Default is `csv`. | No       |

### `-r, --root <PATH>`
Path to the RimWorld mod root directory to scan. **Required**.

### `--out-csv <FILE>`
Save extracted entries to a CSV file (includes header).

### `--out-json <FILE>`
Persist JSON output to disk (requires `--format json`). The command still prints results to stdout unless you redirect it.

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
- `json` — prints a JSON array of translation units to stdout; combine with `--out-json` to keep a copy on disk.

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

Persist JSON output alongside stdout:
```bash
rimloc-cli scan --root ./test/TestMod --format json --out-json ./logs/scan.json
```
*Writes `scan.json` to disk while still emitting JSON to stdout.*

Print CSV output directly to stdout:
```bash
rimloc-cli scan --root ./test/TestMod
```
*Scans all languages and prints CSV data to the console.*

## Troubleshooting

- **"0 rows scanned"** – confirm `Languages/<lang>/Keyed` or `DefInjected` exists and the language code matches `--lang`.
- **Malformed CSV output** – remember that stdout defaults to UTF-8 without BOM; pass `--out-csv <file>` if your spreadsheet tool struggles with pipes.
- **JSON missing placeholders** – placeholders stay in the original XML; include `--source-lang`/`--source-lang-dir` to compare against a specific language during downstream processing.

## See also

- [Validate](validate.md)
- [Validate PO](validate_po.md)
- [Export / Import](export_import.md)

!!! note
    All user-facing CLI output (help, messages) is localized via **Fluent**.
