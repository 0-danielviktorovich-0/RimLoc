---
title: Validate Command
---

# Validate Command

Validate mod XML and report issues such as duplicate keys, empty values, and placeholder problems. Optionally compare placeholders EN → target by key.

## Synopsis

```
rimloc-cli validate --root <PATH> [--format <text|json>] [--game-version <VER>] [--include-all-versions] \
                    [--source-lang <CODE>] [--source-lang-dir <DIR>] \
                    [--defs-dir <PATH>] [--defs-dict <PATH>] [--defs-type-schema <PATH>] [--defs-field <NAME>] \
                    [--compare-placeholders] [--lang <CODE>] [--lang-dir <DIR>]
```

## Notable options

- `--compare-placeholders` — compares placeholder sets between source (EN) and target language entries matched by key. Produces additional `placeholder-check` messages when sets differ.
- `--lang`, `--lang-dir` — target translation (ISO code or folder name) for `--compare-placeholders`. Defaults to `Russian` if omitted.

## JSON output

When `--format json` is set, the command emits a JSON array of messages with fields:

```
{
  "schema_version": 1,
  "kind": "placeholder-check",
  "key": "SomeKey",
  "path": "/Mods/My/Languages/Russian/Keyed/A.xml",
  "line": 42,
  "message": "Placeholder mismatch vs source"
}
```

The schema version is stable across minor releases. When adding new fields, we will bump `OUTPUT_SCHEMA_VERSION` in the CLI and update the docs.

## See also

- [Scan](scan.md)
- [Validate PO](validate_po.md)

