---
title: Export to .po
---

# 📤 Export strings to a single .po

Goal: produce a translator‑friendly `.po` for Poedit/CAT tools. Works for first‑time translations and updates.

## Step 1. Prepare and validate

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan.json
rimloc-cli validate --root ./Mods/MyMod --format text
```

💡 Tip: fix issues before export so you don’t carry noise into the `.po`.

## Step 2. Export `.po`

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

⚠️ Important: do not change [placeholders](../glossary.md#placeholder) in translations.

## Step 3. Check the `.po` (recommended)

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict --format text
```

## Step 4. Import back when ready

```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
```

Looks good? Rerun without `--dry-run`.

## See also

- CLI export/import — ../cli/export_import.md
- Placeholder checks — ../cli/validate_po.md
- Glossary — ../glossary.md

