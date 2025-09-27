---
title: Diff XML
---

# Command Diff XML

Compare presence of keys between source and translation; optionally detect changed source strings against a baseline PO. RimLoc treats English source as `Languages/English` plus `Defs` (implicit labels/descriptions).

## Synopsis

```bash
rimloc-cli diff-xml --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--defs-dir <PATH>] [--defs-field <NAME>] [--lang <CODE>|--lang-dir <DIR>] [--baseline-po <PO>] [--format text|json] \
  [--out-dir <DIR>] [--game-version <VER>] [--strict]
```

## Options
- `--root <MOD>`: mod root (required)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: source folder (default: English)
- `--defs-dir <PATH>`: restrict English Defs scanning to this path (relative to root or absolute)
- `--defs-field <NAME>`: additional Defs field name(s) to extract (repeat or comma‑separate)
- `--lang <CODE>` / `--lang-dir <DIR>`: translation folder (default: Russian)
- `--baseline-po <PO>`: previous export to detect changed source strings
- `--format`: text (default) or json
- `--out-dir <DIR>`: write reports (ChangedData.txt, TranslationData.txt, ModData.txt)
- `--game-version <VER>`: version subfolder under the mod
- `--strict`: non-zero exit if differences are found
