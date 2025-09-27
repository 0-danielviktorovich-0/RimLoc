---
title: Annotate
---

# Command Annotate

Add or remove comments with source text in translation XML files (Keyed). Useful for manual review in editors.

## Synopsis

```bash
rimloc-cli annotate --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--lang <CODE>|--lang-dir <DIR>] [--dry-run] [--backup] [--strip] [--game-version <VER>]
```

## Options
- `--root <MOD>`: mod root to operate on (required)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: where to read original strings from (default: English)
- `--lang <CODE>` / `--lang-dir <DIR>`: target translation folder (default: Russian)
- `--dry-run`: print planned changes without writing
- `--backup`: create .bak before overwriting
- `--strip`: remove existing comments instead of adding
- `--game-version <VER>`: restrict to a version subfolder

