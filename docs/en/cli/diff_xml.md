---
title: Diff XML
---

# Command Diff XML

Compare presence of keys between source and translation; optionally detect changed source strings against a baseline PO.

## Synopsis

```bash
rimloc-cli diff-xml --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--lang <CODE>|--lang-dir <DIR>] [--baseline-po <PO>] [--format text|json] \
  [--out-dir <DIR>] [--game-version <VER>] [--strict]
```

## Options
- `--root <MOD>`: mod root (required)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: source folder (default: English)
- `--lang <CODE>` / `--lang-dir <DIR>`: translation folder (default: Russian)
- `--baseline-po <PO>`: previous export to detect changed source strings
- `--format`: text (default) or json
- `--out-dir <DIR>`: write reports (ChangedData.txt, TranslationData.txt, ModData.txt)
- `--game-version <VER>`: version subfolder under the mod
- `--strict`: non-zero exit if differences are found

