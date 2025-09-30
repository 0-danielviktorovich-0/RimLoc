---
title: Translate a Mod
---

# 🧭 Translate a Mod from Scratch

This tutorial covers exporting a `.po`, translating it, validating, and importing back.

## 1) Prep

- Install RimLoc: `cargo install rimloc-cli`
- Find your mod root (folder with `About/`, `Defs/`, sometimes `Languages/`). Example: `./Mods/MyMod`.

## 2) Scan and validate

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan.json
rimloc-cli validate --root ./Mods/MyMod --format text
```

## 3) Export PO

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

Open the `.po` in Poedit and translate.

## 4) Validate PO

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict
```

## 5) Import back

One file (review‑friendly):
```bash
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml
```

Or structured import:
```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report
```

## 6) Build a translation‑only mod (optional)

```bash
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru
```

