---
title: Translating RimLoc (i18n)
---

# Translating RimLoc (i18n)

RimLoc’s CLI messages are localized via Fluent (FTL) and embedded at build time. This guide shows how to add or update translations.

## Quick guide — no coding required

You can translate directly in GitHub:

1) Open `crates/rimloc-cli/i18n/en/` (English).
2) Create a sibling folder `crates/rimloc-cli/i18n/<lang>/` (e.g., `es`, `de`, `fr`).
3) Copy `rimloc.ftl` and `rimloc-tests.ftl` from `en/` into your `<lang>/` folder.
4) Translate only the values — keep keys and placeholders unchanged.
5) Commit and open a Pull Request. Mention your language code and (optionally) add a `--help` screenshot.

Prefer local edits? See the commands below to run tests locally.

## Folder layout

- `crates/rimloc-cli/i18n/en/rimloc.ftl` — English source of truth.
- `crates/rimloc-cli/i18n/<lang>/rimloc.ftl` — other locales mirror EN keys.
- `crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl` — test messages.

Use IETF/ISO language codes for `<lang>` (e.g., `ru`, `de`, `fr`). RimLoc loads languages by language code; region tags are ignored.

## Adding a new language

1) Copy the English files:

```
crates/rimloc-cli/i18n/en/rimloc.ftl → crates/rimloc-cli/i18n/<lang>/rimloc.ftl
crates/rimloc-cli/i18n/en/rimloc-tests.ftl → crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl
```

2) Translate the values, keep keys and placeholders intact.
   - Keys: lowercase with hyphens.
   - Placeholders: keep `{name}`, `{0}`, `%s`, `%d` exactly as in EN.

3) Run tests:

```bash
cargo test --package rimloc-cli -- tests_i18n
cargo test --workspace
```

4) Verify localized help:

```bash
rimloc-cli --ui-lang <lang> --help
```

If everything passes, the language is included automatically during build (no extra registration necessary).

## Updating strings

- Update EN first (adds/removes keys), then mirror to other locales.
- Keep the same set of keys across locales — tests enforce this.
- Use `docs/en/community/issues.md` to coordinate translation changes if needed.

## Placeholder rules

See the [Placeholders guide](../guide/placeholders.md). Mismatched or malformed placeholders will fail `validate-po --strict`.

Examples (do not change tokens):

```
EN: Found {count} files
ES: Se encontraron {count} archivos

EN: Invalid value: %s
DE: Ungültiger Wert: %s
```

## Helpful editors

- PO files: Poedit (Windows/macOS/Linux), Gtranslator (GNOME), Lokalize (KDE), VS Code + gettext extensions.
- FTL (Fluent): VS Code extensions for “Fluent/FTL” provide syntax highlight and basic checks. Any text editor works.

## Friendly checklist

- [ ] Keys unchanged (translate only values)
- [ ] Placeholders intact (`{…}`, `%…`)
- [ ] Both files present: `rimloc.ftl` and `rimloc-tests.ftl`
- [ ] PR includes your language code and, if possible, a `--help` screenshot

## Also translate the documentation (optional)

- Copy pages from `docs/en/...` to `docs/<lang>/...` with the same structure.
- Keep sections aligned across languages (same headings/order).
- Preview locally:

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve
```

- For a brand‑new docs language, a maintainer needs to add it to `mkdocs.yml` (under the `i18n` plugin). Open an issue or mention it in your PR.
