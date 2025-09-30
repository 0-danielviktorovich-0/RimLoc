---
title: Diff XML
---

# Command Diff XML

Compare presence of keys between source and translation; optionally detect changed source strings against a baseline PO. RimLoc treats English source as `Languages/English` plus `Defs` (implicit labels/descriptions).

## Synopsis

```bash
rimloc-cli diff-xml --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--defs-dir <PATH>] [--defs-field <NAME>] [--defs-dict <PATH>] [--lang <CODE>|--lang-dir <DIR>] [--baseline-po <PO>] [--format text|json] \
  [--out-dir <DIR>] [--game-version <VER>] [--strict] [--apply-flags] [--backup]
```

## Options
- `--root <MOD>`: mod root (required)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: source folder (default: English)
- `--defs-dir <PATH>`: restrict English Defs scanning to this path (relative to root or absolute)
- `--defs-field <NAME>`: additional Defs field name(s) to extract (repeat or comma‑separate)
- `--defs-dict <PATH>`: additional Defs dictionaries (JSON: DefType → [field paths])
- `--lang <CODE>` / `--lang-dir <DIR>`: translation folder (default: Russian)
- `--baseline-po <PO>`: previous export to detect changed source strings
- `--format`: text (default) or json
- `--out-dir <DIR>`: write reports (ChangedData.txt, TranslationData.txt, ModData.txt)
- `--game-version <VER>`: version subfolder under the mod
- `--strict`: non-zero exit if differences are found
- `--apply-flags`: modify translation XML in-place: add `<!-- FUZZY -->` to changed keys (by baseline PO) and `<!-- UNUSED -->` to keys only present in translation
- `--backup`: write `.bak` files before modifying XML (default: true)
