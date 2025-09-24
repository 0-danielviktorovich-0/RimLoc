---
title: Validate Command
---

# Validate Command

## Description

The `validate` command checks the integrity and correctness of your RimWorld mods' localization files (XML Keyed files) within a specified root directory. It verifies that source language files are consistent, properly formatted, and compliant with the expected structure.

## Usage

```bash
rimloc-cli validate --root <ROOT> [OPTIONS]
```

## Options

| Option                 | Description                                                      | Required |
|------------------------|------------------------------------------------------------------|----------|
| `--root`               | Root directory containing localization files                     | Yes      |
| `--source-lang`        | Source language code (e.g., `en`)                                | No       |
| `--source-lang-dir`    | Directory of the source language files                            | No       |
| `--format`             | Output format: text \| json (default: text)                       | No       |
| `--game-version <VER>` | Version folder to operate on (e.g., `1.4`, `v1.4`). Auto-detected if omitted. | No |
| `--include-all-versions` | Validate all version subfolders instead of auto-picking the latest. | No |
| `--ui-lang`            | Language for the UI messages                                     | No       |
| `--help`               | Show help message                                                 | No       |

!!! tip
    If neither `--source-lang` nor `--source-lang-dir` is provided, RimLoc assumes the baseline files live under `Languages/English`.

## Checks performed

- *empty* — detects empty values  
- *duplicate* — finds duplicate keys  
- *placeholder-check* — verifies placeholders consistency  

## Examples

Validate localization files in the `./locales` directory with default options:

```bash
rimloc-cli validate --root ./locales
```

Specify the source language and source language directory:

```bash
rimloc-cli validate --root ./locales --source-lang en --source-lang-dir ./locales/en
```

Get the validation output in JSON format filtered by source language:

```bash
rimloc-cli validate --root ./locales --format json --source-lang en
```

Set the UI language for messages:

```bash
rimloc-cli validate --root ./locales --ui-lang en
```

## Output

The command prints a summary of warnings and errors using the requested format. In `text` mode it uses symbols such as ✖ (errors), ⚠ (warnings), and ℹ (information); in `json` mode it returns structured objects you can post-process.

## Exit codes

- `0` — validation finished without errors (warnings may still appear).
- `1` — at least one error was detected (duplicates, empty strings, placeholder issues).

## Troubleshooting

- **Unexpected duplicates** – make sure translation files are not symlinks to the same XML file or committed twice under different casing.
- **`placeholder-check` errors** – compare the source and translated value; use `--format json` to inspect the offending key/value pair.
- **Command ends with exit code 0 but issues remain** – switch to `--format json` to feed the output into scripts, or add `--ui-lang ru` if you need localized diagnostics.
