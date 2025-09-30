---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/danielviktorovich) [![Ko‑fi](https://img.shields.io/badge/Ko%E2%80%91fi-support-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/danielviktorovich) [![Discord](https://img.shields.io/badge/discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/g8w4fJ8b)

RimLoc helps RimWorld modders keep translations discoverable, validated, and ready for translators.

[:material-play-circle: Start Translating](getting-started.md){ .md-button .md-button--primary }
[:material-cog: Configuration (rimloc.toml)](guide/configuration.md){ .md-button }

## 🚀 Quick Start

New? Start here: getting-started.md — step‑by‑step guide with commands and tips.


## Why RimLoc?

- Inventory every string under `Languages/*/{Keyed,DefInjected}` with one command.
- Prevent broken releases by catching duplicate keys, empty values, and placeholder drift.
- Export and import PO/CSV bundles so translators can work with familiar tooling.
- Build translation-only RimWorld mods straight from a curated `.po` file.
- Ship CLIs localized via Fluent (English and Russian included by default).

## Commands at a glance

See the CLI overview: cli/index.md. Jump directly: Scan · Validate · Validate PO · Export/Import · Build Mod.

## Core commands

| Command | What it does | Notes |
|---------|---------------|-------|
| `scan` | Enumerates translation units from XML. | Use `--out-csv` or `--out-json` to persist alongside stdout. |
| `validate` | Flags duplicates, empties, placeholders in XML. | Combine with `--format json` for CI parsing. |
| `validate-po` | Compares placeholders in PO `msgid`/`msgstr`. | Use `--strict` to fail on warnings. |
| `export-po` | Publishes a single PO hand-off file. | Requires `--root` and `--out-po`; add `--lang` for the target locale. |
| `import-po` | Writes PO updates back into XML. | `--dry-run` previews changes; `--single-file` routes everything to `_Imported.xml`. |
| `build-mod` | Builds a translation-only RimWorld mod from a PO file. | `--dry-run` prints the plan; adjust `--package-id` and `--rw-version` before release. |

## Next steps

- Read the [CLI overview](cli/index.md) for command-specific options and examples.
- Jump directly to: [Scan](cli/scan.md) · [Validate](cli/validate.md) · [Validate PO](cli/validate_po.md) · [Export / Import](cli/export_import.md) · [Build Mod](cli/build_mod.md)
- Update docs locally with `mkdocs serve` and edit the files under `docs/en/` and `docs/ru/`.

!!! tip "Help translate RimLoc"
    Want RimLoc in your language? Check the [Localization guide](community/localization.md). You can translate via GitHub web editor with no local setup.

!!! tip "Looking for the CLI source?"
    The binaries live in `crates/rimloc-cli`. Fixtures for experimenting are under `test/`.

## Contributing to docs

Found a typo or want to add examples? [Edit this page on GitHub](https://github.com/0-danielviktorovich-0/RimLoc/tree/main/docs/en/index.md) or check the contributor guide in [AGENTS.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md).
