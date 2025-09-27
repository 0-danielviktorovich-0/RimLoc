---
title: XML Health
---

# Command XML Health

Scan XML files under `Languages/` for structural/read errors. Works well in CI with `--strict`.

## Synopsis

```bash
rimloc-cli xml-health --root <MOD> [--format text|json] [--lang-dir <DIR>] [--strict] \
  [--only <CSV>] [--except <CSV>]
```

## Options
- `--root <MOD>`: mod root (required)
- `--format`: text (default) or json
- `--lang-dir <DIR>`: restrict to a specific language folder
- `--strict`: exit with non-zero code if issues are found
- `--only`: include only these categories (comma-separated)
- `--except`: exclude these categories (comma-separated)

### Categories

- `encoding` — file cannot be read as UTF-8
- `encoding-detected` — `<?xml ... encoding=...?>` is not UTF-8
- `invalid-char` — control character < 0x20 present (except TAB/LF/CR)
- `tag-mismatch` — mismatched tags reported by XML parser
- `invalid-entity` — bad entity/escaping (e.g., bare `&`)
- `unexpected-doctype` — `<!DOCTYPE ...>` present (not expected for LanguageData)

Text output includes short hints on how to fix typical issues.
