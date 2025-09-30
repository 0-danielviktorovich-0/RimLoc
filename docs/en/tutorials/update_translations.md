---
title: Update Translations
---

# ♻️ Update Translations (when the mod updates)

Find what changed and update your `.po` safely.

## 1) Refresh and validate

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan_after.json
rimloc-cli validate --root ./Mods/MyMod --format text
```

## 2) See what changed

```bash
rimloc-cli diff-xml --root ./Mods/MyMod --format text
```

## 3) Export a fresh `.po`

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

Translate newly added strings in the `.po`.

## 4) Validate `.po`

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict
```

## 5) Import with a report

```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
```

If the plan looks good, rerun without `--dry-run`.

## 6) (Optional) Rebuild the translation‑only mod

```bash
rimloc-cli build-mod --from-root ./Mods/MyMod --out-mod ./MyMod_RU --lang ru --dry-run
rimloc-cli build-mod --from-root ./Mods/MyMod --out-mod ./MyMod_RU --lang ru
```

