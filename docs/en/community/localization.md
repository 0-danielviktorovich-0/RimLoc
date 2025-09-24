---
title: Translating RimLoc (i18n)
---

# Translating RimLoc (i18n)

RimLoc’s CLI messages are localized via Fluent (FTL) and embedded at build time. This guide shows how to add or update translations.

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

