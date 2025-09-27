---
title: Lang Update
---

# Command Lang Update

Update the official localization from a GitHub repo into `Data/Core/Languages` under your RimWorld installation.

## Synopsis

```bash
rimloc-cli lang-update --game-root <RIMWORLD> [--repo <OWNER/NAME>] \
  [--branch <BRANCH>] [--zip <FILE>] [--source-lang-dir <DIR>] \
  [--target-lang-dir <DIR>] [--dry-run] [--backup]
```

## Options
- `--game-root <RIMWORLD>`: RimWorld game root directory (must contain `Data/`).
- `--repo <OWNER/NAME>`: GitHub repo to use (default: `Ludeon/RimWorld-ru`).
- `--branch <BRANCH>`: Branch name to download from (uses default branch otherwise).
- `--zip <FILE>`: Local zip to use instead of downloading (useful for offline runs).
- `--source-lang-dir <DIR>`: Source folder inside the repo under `Core/Languages/` (default: `Russian`).
- `--target-lang-dir <DIR>`: Target folder name to create under `Data/Core/Languages/` (default: `Russian (GitHub)`).
- `--dry-run`: Print what would be written without touching the filesystem.
- `--backup`: If target exists, rename it to `.bak` before writing.

## Examples

Dry-run the update for the default repo into `Russian (GitHub)`:

```bash
rimloc-cli --quiet lang-update --game-root "/games/RimWorld" --dry-run
```

Use a local zip file (no network):

```bash
rimloc-cli lang-update --game-root "/games/RimWorld" --zip ./RimWorld-ru.zip --backup
```

