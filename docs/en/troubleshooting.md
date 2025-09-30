---
title: Troubleshooting
---

# 🛠️ Troubleshooting

## Command not found

- Check PATH: `~/.cargo/bin` (Linux/macOS), `%USERPROFILE%\.cargo\bin` (Windows).
- On macOS for downloaded builds:
  ```bash
  chmod +x ./rimloc-cli
  ./rimloc-cli --help
  ```

## Placeholder issues after import

- Validate the `.po` first:
  ```bash
  rimloc-cli validate-po --po ./MyMod.ru.po --strict
  ```
- Ensure placeholders in `msgid/msgstr` match exactly.

## Empty or duplicate keys

- Validate XML sources:
  ```bash
  rimloc-cli validate --root ./Mods/MyMod --format text
  ```

## Encoding problems

- Ensure UTF‑8 (no BOM) in your editor.
- See xml health: cli/xml_health.md

## Import does nothing

- `.po` has empty `msgstr` values only.
- Keys/paths don’t match XML.
- Use `--report --dry-run` to inspect mapping.

