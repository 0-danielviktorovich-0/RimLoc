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
- [docs] short bullet with (#PR)

### Internal
- [internal] short bullet with (#PR)
-->
### Added
- [export-po] Translation Memory prefill: `--tm-root` to prefill msgstr and mark entries as `fuzzy` (#PR)
- [cli] Localized help for `--tm-root` and TM coverage summary in export output (#PR)

### Fixed
- [parsers-xml] Handle self-closing keyed XML elements correctly (#PR)

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
