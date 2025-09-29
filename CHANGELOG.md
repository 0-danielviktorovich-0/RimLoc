# Changelog

All notable changes to this project are documented in this file.
This changelog follows Keep a Changelog and Semantic Versioning.

## [Unreleased]
<!--
Template (copy the sections you need):

### Added
- [scope] short bullet with (#PR)

### Changed
- [scope] short bullet with (#PR)

### Fixed
- [scope] short bullet with (#PR)

### Docs
- [docs] Add README banner and move asset to `docs/assets` (#PR)

### Internal
- [internal] Introduce `rimloc-services` orchestration crate and adopt it in `scan` CLI path (#PR)
- [internal] Route `export-po`, `validate`, `import-po`, `build-mod` via services; add import/build wrappers (#PR)
-->
### Added
- [export-po] Translation Memory prefill: `--tm-root` to prefill msgstr and mark entries as `fuzzy` (#PR)
- [cli] Localized help for `--tm-root` and TM coverage summary in export output (#PR)
- [cli] New `diff-xml` command: compares source vs translation presence and, with a baseline PO, detects changed source strings; supports text/json and writing ChangedData.txt/TranslationData.txt/ModData.txt (#PR)
- [cli] New `annotate` command: add/remove source-text comments in translation XML; supports dry-run and backups (#PR)
- [cli] New `xml-health` command: scans XML files under Languages/ for structural/read errors (text/json) (#PR)
- [cli] New `init` command: generate translation skeleton under `Languages/<target>` with empty values (text/dry-run/overwrite) (#PR)

### Fixed
- [parsers-xml] Handle self-closing keyed XML elements correctly (#PR)
- [services] diff-xml baseline: honor msgctxt key extraction when computing changed entries (#PR)

### Docs
- [docs] AGENTS: add rule to reply in Russian when addressed in Russian (#PR)
- [docs] README: replace AGENTS.md link with CONTRIBUTING.md (#PR)
- [docs] AGENTS: make commit via scripts/agent-commit.sh a mandatory finish step for agents (#PR)
- [docs] AGENTS: explicitly allow using GH_TOKEN/GITHUB_TOKEN when provided by the user, with safety rules (#PR)

## [0.1.0-alpha.1] - 2025-09-25
### Added
- rimloc-cli initial prerelease: scan, validate, export-po, import-po, build-mod
- i18n (EN/RU), colored logs, JSON output and --quiet mode
- Dev release automation, artifact signing (cosign) and SBOM (Syft)

### Docs
- Install page (EN/RU), Support page with BMC/Ko-fi and crypto addresses
- Discord invite and badges

## [0.1.0] - 2025-09-25
### Added
- rimloc-core: TransUnit/PoEntry, minimal PO parser
- rimloc-parsers-xml: scan Keyed XML → TransUnit
- rimloc-export-csv: CSV exporter with optional lang column
- rimloc-export-po: PO exporter with msgctxt and references
- rimloc-import-po: PO reader and LanguageData XML writer
- rimloc-validate: empty/duplicate/placeholder checks
 
<!-- Links -->
[Unreleased]: https://github.com/0-danielviktorovich-0/RimLoc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/0-danielviktorovich-0/RimLoc/compare/v0.1.0-alpha.1...v0.1.0
[0.1.0-alpha.1]: https://github.com/0-danielviktorovich-0/RimLoc/releases/tag/v0.1.0-alpha.1
