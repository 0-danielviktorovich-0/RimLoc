# RimLoc

<p align="center">
  <img src="docs/assets/RIMLOC-baner.png" alt="RimLoc banner" />
</p>

<p align="center">
  <strong>RimWorld-first localization workstation with a local-first Rust core and an adapter-ready architecture.</strong>
</p>

[![CI](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/ci.yml/badge.svg)](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/core-Rust-orange)](Cargo.toml)
[![Desktop](https://img.shields.io/badge/desktop-Tauri%202-blue)](gui/tauri-app)

> **Status: pre-beta.** RimLoc is under active development. The RimWorld workflow is the first production target; the desktop UI is being converged on the new React R1 frontend. No stable public desktop release is claimed yet.

[Русская документация](https://0-danielviktorovich-0.github.io/RimLoc/ru/) ·
[Documentation](https://0-danielviktorovich-0.github.io/RimLoc/) ·
[Issues](https://github.com/0-danielviktorovich-0/RimLoc/issues) ·
[Security](SECURITY.md) ·
[Contributing](CONTRIBUTING.md)

---

## What RimLoc is

RimLoc is a localization toolchain for translating and maintaining RimWorld content without editing the original game or Workshop sources in place.

The current product combines:

- **RimWorld-aware source discovery** for Core/DLC/mod layouts and versioned content;
- **translation project state** with revisions and multiple target locales;
- **manual editing, validation, glossary and diagnostics** in a desktop workstation;
- **CLI workflows** for scanning, validating, PO interchange and building translation-only mods;
- **local-first storage** and isolated output paths;
- an **adapter-oriented architecture** so future games and applications can be added without turning the core into RimWorld-specific code.

RimWorld is the first production adapter, not the long-term limit of the architecture.

## Why it exists

RimWorld localization is more than copying XML strings. Real projects need to survive mod updates, preserve human work, understand `Keyed`/`DefInjected` and versioned layouts, catch placeholder or structural errors, and produce a clean translation mod.

RimLoc aims to put that workflow in one place:

1. discover the source;
2. build a canonical translation inventory;
3. translate and review;
4. validate;
5. update safely when the source changes;
6. export/build without writing back into the source tree.

## Desktop application

The desktop client uses **Tauri 2** over the same Rust service layer as the CLI.

### React R1

The intended production frontend is:

`gui/tauri-app/frontend-react/`

It currently provides the new workstation shell and project routes used by the UI R1 campaign: project creation/opening, workspace/editor flows, validation, glossary, multi-target project state, build/export, diagnostics, settings and related tooling.

### Svelte fallback

The previous frontend remains in:

`gui/tauri-app/frontend-v2/`

It is intentionally kept as a **frozen legacy/fallback** during React convergence and is not the long-term UI direction.

The two frontends must not be confused when producing test or owner-review artifacts.

## CLI

The Rust CLI remains useful for deterministic and headless workflows.

Typical commands include:

- `scan`
- `validate`
- `validate-po`
- `export-po`
- `import-po`
- `build-mod`
- `diff-xml`
- `annotate`
- `xml-health`
- `init`

Example:

```bash
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
cargo run -p rimloc-cli -- export-po \
  --root ./test/TestMod \
  --out-po ./logs/TestMod.po \
  --lang ru
cargo run -p rimloc-cli -- build-mod \
  --po ./logs/TestMod.po \
  --out-mod ./logs/TestMod-ru \
  --lang ru \
  --dry-run
```

## Current capability status

This table is deliberately conservative. A feature being designed or present in a UI mock is not treated as production proof.

| Area | Current direction |
| --- | --- |
| RimWorld source scanning / canonical inventory | Implemented and covered by the Rust pipeline |
| Manual translation editing | Implemented in the desktop project workflow |
| Validation / findings | Implemented |
| PO interchange | Implemented in the CLI/service stack |
| Translation-only build/export | Implemented in the Rust toolchain |
| Multi-target project state | Implemented in the current project model |
| Project glossary | Implemented with persistence |
| Translation Memory | Active pre-beta work: automatic reuse + import + manual management |
| Existing-translation update workflow | Active pre-beta hardening |
| Source Inspector / provenance | Active integration/hardening |
| AI / provider workflows | UI and provider architecture exist; production integration is still being hardened |
| Runtime/game verification | Test-lab capability; not presented as a normal production feature |
| Other games/applications | Architecture target only; no support is claimed yet |

For detailed engineering evidence, tests and current limitations, see the repository documentation rather than assuming every roadmap item is already shipped.

## RimWorld scope

The near-term goal is a high-quality RimWorld localization workstation before expanding the product surface.

The project is designed around modern RimWorld layouts, including versioned mod content and the workflows needed for Core, DLC, mods, language packs and existing translations where the relevant adapter capability is available.

**RimWorld 1.6 is the primary target.** Compatibility with older layouts is maintained where practical, but the pre-beta campaign prioritizes current RimWorld behavior and real-corpus testing.

## Local-first and source safety

RimLoc is designed so the source remains authoritative and read-only during normal translation workflows.

Project output is expected to go to isolated locations, with validation around unsafe paths and source-tree escapes.

Security-sensitive development areas include:

- filesystem containment and symlink/path traversal checks;
- Tauri capability and IPC review;
- secret handling for optional providers;
- sanitized diagnostics;
- production exclusion of test/automation hooks.

See [SECURITY.md](SECURITY.md) for the reporting policy. The pre-beta security audit is being refreshed together with the current dependency graph and GitHub security findings.

## Build from source

### Rust workspace

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### React desktop candidate

Prerequisites: Rust, Node.js 20+ and the Tauri platform prerequisites.

```bash
cd gui/tauri-app/frontend-react
npm install
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
```

The base Tauri configuration still points at the frozen Svelte fallback during the migration, so use the React configuration explicitly when testing the React R1 candidate.

## Architecture direction

The generic core should understand localization concepts, not game-specific names.

Conceptually:

```text
Localization source
      ↓
LocalizationAdapter
      ↓
canonical SourceEntry inventory
      ↓
Project / translations / revisions
      ↓
TM / glossary / validation / AI
      ↓
RimLocClient
      ↓
CLI / desktop GUI / future integrations
```

RimWorld-specific behavior belongs in the RimWorld adapter. RimLoc self-localization is a second distinct adapter direction. Future adapters may target other games or applications, but they are roadmap items rather than pre-beta promises.

The long-term developer experience should make adding a new adapter a bounded task: declare capabilities, map source content into canonical entries, implement relevant import/export/build behavior, add fixtures, and pass an adapter conformance suite.

## Pre-beta priorities

Before the first public graphical beta, the project is prioritizing:

- React R1 visual and workflow convergence;
- real RimWorld update/existing-translation workflows;
- Translation Memory and glossary integration;
- practical differential testing against established RimWorld localization tools;
- security/dependency/code-scanning reconciliation;
- macOS and Windows acceptance;
- current documentation and repository hygiene.

The goal is not to copy every enterprise localization platform before users can try RimLoc.

## Long-term direction

If the project proves useful, the north star is a broader localization workstation:

- game-context quality inspired by **Gridly**;
- professional translator workflows inspired by **Trados**;
- continuous/community localization ideas inspired by **Crowdin**;
- while staying local-first, open-source and adapter-oriented.

Future adapters could cover other games, mod ecosystems and ordinary applications. They will be added only when there is a practical use case and adequate testing.

## Documentation

- [Documentation site](https://0-danielviktorovich-0.github.io/RimLoc/)
- [Getting started](docs/en/getting-started.md)
- [CLI reference](docs/en/cli/)
- [GUI guide](docs/en/guide/gui.md)
- [Developer documentation](docs/en/dev/)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

The documentation is currently being audited against the React/Rust pre-beta architecture; pages that describe the older GUI should not be treated as authoritative when they conflict with current code.

## Contributing

Contributions are welcome, especially around RimWorld compatibility, real-world fixtures, validation, documentation and future adapter ergonomics.

Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) before making repository changes.

## License

RimLoc is licensed under the [GNU General Public License v3.0](LICENSE).
