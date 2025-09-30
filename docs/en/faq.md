---
title: FAQ
---

# ❓ Frequently Asked Questions

## `rimloc-cli`: command not found

- If installed via Cargo — open a new terminal or ensure `~/.cargo/bin` (Windows: `%USERPROFILE%\.cargo\bin`) is in PATH.
- If using a downloaded binary — run it from that folder (`./rimloc-cli` or `.\rimloc-cli.exe`) or add the folder to PATH.

## `scan` vs `validate`?

- `scan` inventories translatable strings.
- `validate` performs QA and exits with `1` when errors are present.

## How do I build a translation‑only mod?

```bash
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru
```

Or from an existing `Languages/Russian`: cli/build_mod.md

## Can I preview changes first?

Yes. Use `--dry-run` where supported. For example:
```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
```

## How do I see what changed between mod versions?

```bash
rimloc-cli diff-xml --root ./Mods/MyMod --format text
```

