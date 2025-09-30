---
title: Glossary
---

# 📚 RimLoc Glossary

## Placeholder

A token inside text that the game replaces with values (numbers, names, etc.). Examples: `%d`, `%s`, `{0}`, `{PAWN_name}`. Must match exactly between source and translation. See cli/validate_po.md

## DefInjected

Translations live alongside XML definitions (ThingDef, RecipeDef, …). Keys are built from paths: `DefType.defName.field`.

## Keyed

Dictionary‑style translations under `Languages/<Lang>/Keyed/*.xml` with `key → value` pairs.

## PO (Portable Object)

Standard translation file where `msgid` is source and `msgstr` is target. RimLoc can export a single `.po` merging DefInjected and Keyed. See cli/export_import.md

## Dry‑run

Preview changes without writing to disk. Use `--dry-run`.

## Mod root

Top‑level folder of a mod (contains `About/`, `Defs/`, sometimes `Languages/`).

## Report (`--report`)

Verbose summary of changes during import/build for review.

## CSV/JSON output

Machine-readable output from `scan`, `validate`, `diff-xml`, etc. Useful for CI and analysis.

## Locale

Language/culture code like `ru`, `en`, `pt-br`. Used to select `Languages/<Lang>` and fill `.po` metadata.

## Key

Identifier of a string. For `Keyed` it’s the XML node name (e.g., `GreetingHello`); for `DefInjected` it’s a path like `ThingDef.Beer.label`.
