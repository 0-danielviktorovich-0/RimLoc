---
title: Tips & Tricks
---

# 🧠 Tips & Tricks

- Start with `validate` to catch duplicates/empties early.
- Use `--dry-run` before any writing command.
- Keep a `rimloc.toml` in your repo to avoid repeating flags. See guide/configuration.md
- Always check placeholders in PO via `validate-po --strict`.
- Combine `--format json` with `jq` to filter errors:
  ```bash
  rimloc-cli validate --root ./Mods/MyMod --format json | jq '.[] | select(.level=="error")'
  ```
- Build translation‑only mods with `build-mod` for distribution.
- Update official Core localization with cli/lang_update.md

