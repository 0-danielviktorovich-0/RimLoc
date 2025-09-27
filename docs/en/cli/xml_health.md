---
title: XML Health
---

# Command XML Health

Scan XML files under `Languages/` for structural/read errors. Works well in CI with `--strict`.

## Synopsis

```bash
rimloc-cli xml-health --root <MOD> [--format text|json] [--lang-dir <DIR>] [--strict]
```

## Options
- `--root <MOD>`: mod root (required)
- `--format`: text (default) or json
- `--lang-dir <DIR>`: restrict to a specific language folder
- `--strict`: exit with non-zero code if issues are found

