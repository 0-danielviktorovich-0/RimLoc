---
title: Init
---

# Command Init

Create a translation skeleton under `Languages/<target>` with empty values based on the source language.

## Synopsis

```bash
rimloc-cli init --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--lang <CODE>|--lang-dir <DIR>] [--overwrite] [--dry-run] [--game-version <VER>]
```

## Options
- `--root <MOD>`: mod root (required)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: source folder (default: English)
- `--lang <CODE>` / `--lang-dir <DIR>`: translation folder (default: Russian)
- `--overwrite`: overwrite files if they already exist
- `--dry-run`: print the plan without writing
- `--game-version <VER>`: restrict to specific version folder

