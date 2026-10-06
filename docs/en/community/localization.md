---
title: Translating RimLoc
---

# Translating RimLoc itself

RimLoc is being moved toward **self-localization through the same canonical project model** used by other localization sources.

That means the long-term contributor experience is not “edit random UI dictionaries by hand”, but:

1. open the RimLoc application catalog as a localization project;
2. choose a target locale;
3. translate/review in the normal editor;
4. validate placeholders/select/plural rules;
5. build a contribution bundle;
6. submit the translation for review.

This is the second real adapter direction after RimWorld and helps prove that the core is not RimWorld-only.

## Current pre-beta status

The self-localization pipeline is still being integrated into the React product workflow. If the current build exposes **Translate RimLoc**, use that path.

When the UI path is unavailable on your build, contributors can still work with the repository sources, but treat that as a developer fallback rather than the final product experience.

## CLI messages

The Rust CLI uses Fluent (FTL). English remains the source locale for those CLI message catalogs.

Typical layout:

~~~text
crates/rimloc-cli/i18n/en/
crates/rimloc-cli/i18n/ru/
...
~~~

When editing FTL directly:

- translate values, not keys;
- preserve placeholders exactly;
- keep locale key sets aligned;
- run the CLI i18n tests.

~~~bash
cargo test --package rimloc-cli -- tests_i18n
~~~

## UI/application catalog

The application UI catalog has its own canonical bridge/project path. Do not assume the CLI FTL directory is the source of every React UI string.

See the internal architecture documentation for the current self-localization bridge when doing implementation work.

## Documentation translations

Public docs currently maintain English and Russian trees under <code>docs/en</code> and <code>docs/ru</code>.

For a new docs locale:

- mirror the page structure;
- translate content while keeping technical identifiers/commands accurate;
- add the locale to the MkDocs i18n configuration;
- run <code>mkdocs build --strict</code>.

## Contribution checklist

- target locale identified;
- placeholders/select/plural syntax preserved;
- no source IDs changed;
- validation passes;
- contribution contains translation data, not secrets/project-local paths;
- EN/RU docs updated together when behavior changes.
