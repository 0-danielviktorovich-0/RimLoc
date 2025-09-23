---
title: Validate PO Command
---

# Validate PO Command

## Description

The `validate-po` command checks the correctness and consistency of PO (Portable Object) translation files used in RimWorld mods. It validates placeholder correctness as a key part and ensures overall quality of localization files, especially when strict mode is enabled.

## Checks performed

- **Placeholder mismatch** — detects inconsistencies in placeholders between source and translated strings.

## Usage

```
rimloc-cli validate-po --po <PO> [OPTIONS]
```

## Options

| Option           | Description                                                   | Required |
|------------------|---------------------------------------------------------------|----------|
| `--po <PO>`      | Specify the PO file to validate.                              | Yes      |
| `--strict`       | Enable strict validation mode, treating warnings as errors.  | No       |
| `--format`       | Output format of the validation report (default: text).      | No       |
| `--ui-lang <LANG>`| Set the language for UI messages.                            | No       |
| `--help`         | Show help message for the validate-po command.               | No       |

## Examples

Validate a PO file with default settings:

```
rimloc-cli validate-po --po Mods/MyMod/Translations/en.po
```
*Basic validation to check for common issues.*

Validate a PO file in strict mode with JSON output:

```
rimloc-cli validate-po --po Mods/MyMod/Translations/en.po --strict --format json
```
*Strict validation treating warnings as errors and outputting results in JSON format.*

## Output

The command outputs a detailed report of errors and warnings found in the PO file, helping maintain high-quality and consistent translations for your mod. In `text` format, the output uses symbols such as ✖ for errors, ⚠ for warnings, and ℹ for informational messages. In `json` format, the output consists of structured objects representing the validation results.
