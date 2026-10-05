# RimLoc

<p align="center">
  <img src="docs/assets/RIMLOC-baner.png" alt="RimLoc banner" />
</p>

<p align="center">
  <strong>RimWorld-first localization workstation — local-first, open-source, and designed to grow through adapters.</strong>
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="docs/readme/ru/README.md">Русский</a>
</p>

<p align="center">
  <a href="https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/ci.yml"><img src="https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://0-danielviktorovich-0.github.io/RimLoc/"><img src="https://img.shields.io/badge/docs-GitHub%20Pages-blue" alt="Docs" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="GPL-3.0" /></a>
  <a href="https://github.com/sponsors/0-danielviktorovich-0"><img src="https://img.shields.io/badge/Sponsor-GitHub-%23ea4aaa?logo=github-sponsors" alt="GitHub Sponsors" /></a>
  <a href="https://buymeacoffee.com/danielviktorovich"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black" alt="Buy Me a Coffee" /></a>
  <a href="https://ko-fi.com/danielviktorovich"><img src="https://img.shields.io/badge/Ko--fi-support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi" /></a>
</p>

> **Pre-beta.** RimLoc is under active development. RimWorld is the first production target; the new desktop UI is being converged on React R1. There is no stable public desktop release yet.

[Documentation](https://0-danielviktorovich-0.github.io/RimLoc/) ·
[Getting started](docs/en/getting-started.md) ·
[Security](SECURITY.md) ·
[Contributing](CONTRIBUTING.md) ·
[Support the project](docs/en/community/support.md)

---

## What RimLoc is

RimLoc helps you translate and maintain RimWorld content without turning localization into a pile of one-off scripts or editing the original Workshop files in place.

The project has two faces:

- **Desktop workstation (Tauri 2 + React 19)** for day-to-day project work: create/open a project, edit translations, switch target languages, validate, use a glossary, inspect context, build/export, and diagnose problems.
- **Rust CLI and service layer** for deterministic automation, CI, batch operations, format interchange, and debugging.

The core idea is broader than RimWorld: game-specific knowledge belongs in a localization adapter; project state, translations, TM, glossary, validation, revisions and review stay generic. RimWorld is the first production adapter, not the permanent boundary of the product.

## PO is optional, not the project format

RimLoc does **not** use PO as its canonical internal model.

The desktop workflow is intended to be:

~~~text
discover source
→ create/open RimLoc project
→ edit translations in the app
→ validate/review
→ build/export translation output
~~~

No PO round-trip is required for that workflow.

PO remains useful as an **interchange format**:

- hand work to Poedit or another CAT tool;
- move translations into/out of existing translator workflows;
- use format-specific CLI commands such as <code>export-po</code> and <code>import-po</code>.

The CLI still exposes those explicit PO commands because they are useful and stable tools. It can also build a translation mod from an existing <code>Languages/&lt;locale&gt;</code> tree with <code>build-mod --from-root</code>. The long-term architecture keeps PO/CSV/XLIFF/XML as adapters around the canonical project model rather than making any one of them mandatory.

## What already exists

| Area | Current state |
| --- | --- |
| RimWorld-aware source discovery / canonical inventory | Implemented in the Rust pipeline |
| Manual project editing | Implemented in the desktop workflow |
| Validation / findings | Implemented |
| Multi-target project state | Implemented |
| Project glossary | Implemented with persistence |
| PO interchange | Implemented as an optional CLI/service workflow |
| Translation-only build/export | Implemented in the Rust toolchain |
| Existing-translation update | Pre-beta hardening in progress |
| Translation Memory | Pre-beta work: auto reuse + import + manual management |
| AI/providers | Architecture/UI exists; production integration is still being hardened |
| In-game Runtime Bridge | Test-lab capability, not a normal production feature |
| Other games/apps | Adapter architecture only; no public support claim yet |

We deliberately avoid presenting roadmap work as shipped functionality.

## RimWorld scope

RimWorld 1.6 is the primary target. The codebase also handles versioned mod layouts and keeps compatibility with older structures where practical.

The product direction includes workflows for mods, Core/DLC localization sources, language packs, existing translations, source updates, and multiple target locales.

## Desktop UI

The intended production frontend is:

<code>gui/tauri-app/frontend-react/</code>

The old Svelte frontend remains temporarily at:

<code>gui/tauri-app/frontend-v2/</code>

as a frozen fallback/reference during convergence. It is not the long-term UI direction.

Build the React candidate explicitly:

~~~bash
cd gui/tauri-app/frontend-react
npm install
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
~~~

The base Tauri config still points to the frozen fallback while migration/acceptance is in progress, so the React config matters when producing a React test artifact.

## CLI quick start

For deterministic/headless work:

~~~bash
cargo build -p rimloc-cli

cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
~~~

Optional CAT/PO handoff:

~~~bash
cargo run -p rimloc-cli -- export-po \
  --root ./test/TestMod \
  --out-po ./logs/TestMod.po \
  --lang ru
~~~

Build directly from an existing translated Languages tree — no PO required:

~~~bash
cargo run -p rimloc-cli -- build-mod \
  --from-root ./Mods/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

See the [CLI documentation](docs/en/cli/index.md) for the full command set.

## Local-first and source safety

Normal translation workflows treat game/mod sources as read-only. Writes go to project storage or explicit output locations.

Pre-beta hardening focuses on:

- output-path containment and symlink/path traversal;
- Tauri capabilities and IPC;
- provider secret handling;
- redacted diagnostics;
- keeping automation/test bridges out of production artifacts;
- dependency and CodeQL reconciliation.

Please report security issues privately as described in [SECURITY.md](SECURITY.md).

## Architecture direction

~~~text
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
~~~

A future contributor should be able to add another game/application by implementing a bounded adapter and passing conformance tests, without rewriting the editor, TM, glossary or project model.

See [Localization adapters](docs/architecture/LOCALIZATION_ADAPTERS.md).

## Pre-beta priorities

Before the first public graphical beta, the project is prioritizing:

- React R1 workflow and visual convergence;
- real RimWorld existing/update workflows;
- TM and glossary integration;
- practical differential testing against established RimWorld localization tools;
- security/dependency/code-scanning reconciliation;
- macOS and Windows acceptance;
- current, honest documentation.

The goal is to ship a strong RimWorld beta and learn from real users — not to recreate every enterprise localization platform before anyone can try the app.

## Long-term north star

If RimLoc proves useful, the ambition is broader:

- game context and production ergonomics inspired by **Gridly**;
- professional translator depth inspired by **Trados**;
- continuous/community localization ideas inspired by **Crowdin**;
- while remaining local-first, open-source and adapter-oriented.

Future adapters may target other games, mod ecosystems and ordinary applications when there is a practical reason to build them.

## Documentation and community

- [Documentation site](https://0-danielviktorovich-0.github.io/RimLoc/)
- [Getting started](docs/en/getting-started.md)
- [Translator workflow](docs/en/guide/translators.md)
- [GUI](docs/en/guide/gui.md)
- [CLI reference](docs/en/cli/)
- [Developer guide](docs/en/dev/)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)
- [Support](SUPPORT.md)
- [Support development / donations](docs/en/community/support.md)
- [Changelog](CHANGELOG.md)

## Contributing

Contributions are welcome — especially real RimWorld fixtures, adapter work, validation, docs, accessibility and reproducible bug reports.

Please read [CONTRIBUTING.md](CONTRIBUTING.md). Coding agents should also read [AGENTS.md](AGENTS.md).

## License

GNU GPL v3 — see [LICENSE](LICENSE).
