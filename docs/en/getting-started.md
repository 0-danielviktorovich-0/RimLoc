---
title: Getting Started
---

# 🚀 Getting Started with RimLoc

RimLoc helps you inventory, validate, and ship translations for RimWorld mods. This guide is a simple step‑by‑step path for newcomers.

See also the glossary: glossary.md

## 1) Install (1–2 minutes)

- Via Cargo:
  ```bash
  cargo install rimloc-cli
  ```
- Or download a prebuilt binary from Releases and run it directly.

More: install.md · install_run.md

## 2) First run — scan and validate

Assume your mod lives in `./Mods/MyMod`.

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan.json
rimloc-cli validate --root ./Mods/MyMod
```

- `scan` inventories strings and saves them (for a quick look).
- `validate` catches duplicates, empties, and placeholder issues.

## 3) Export a `.po` for translators

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

Open it in Poedit or your preferred CAT tool and translate.

More: cli/export_import.md

## 4) Check your `.po` (recommended)

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict --format text
```

This ensures placeholders in `msgid/msgstr` match exactly.

More: cli/validate_po.md

## 5) Import translations back into the mod

Single file for review:
```bash
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml
```

Structured import (release‑ready):
```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report
```

## 6) Build a translation‑only mod (optional)

```bash
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru
```

## 7) Next steps

- From scratch: tutorials/translate_mod.md
- Updating an existing translation: tutorials/update_translations.md
- FAQ: faq.md
- Troubleshooting: troubleshooting.md

