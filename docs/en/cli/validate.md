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

| Option               | Description                                            | Required |
|----------------------|--------------------------------------------------------|----------|
| `--root`             | Root directory containing localization files           | Yes      |
| `--source-lang`      | Source language code (e.g., `en`)                       | No       |
| `--source-lang-dir`  | Directory of the source language files                  | No       |
| `--format`           | Output format: text \| json (default: text)             | No       |
| `--ui-lang`          | Language for the UI messages                             | No       |
| `--help`             | Show help message                                       | No       |

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

The command outputs a summary of validation results, including errors and warnings, formatted according to the selected output format. In `text` format, symbols such as ✖ (errors), ⚠ (warnings), and ℹ (information) are used to indicate message severity. In `json` format, structured objects representing the validation results are provided. If no issues are found, a success message is displayed. Exit codes: `0` if validation passes without errors; `1` if any errors are found.