---
title: Developer Guide
---

# Developer guide

RimLoc is a Rust localization core/services stack with a Tauri 2 desktop application.

## Current architecture

~~~text
source
  ↓
LocalizationAdapter
  ↓
canonical project / SourceEntry inventory
  ↓
translations · revisions · TM · glossary · validation
  ↓
rimloc-services
  ↓
RimLocClient / CLI
  ↓
React desktop / automation / future integrations
~~~

Key directories:

- <code>crates/</code> — Rust domain/core/services/adapters/import/export/CLI;
- <code>gui/tauri-app/frontend-react/</code> — intended production React frontend;
- <code>gui/tauri-app/frontend-v2/</code> — frozen Svelte fallback;
- <code>gui/tauri-app/src-tauri/</code> — Tauri bridge/config;
- <code>docs/</code> — canonical MkDocs sources;
- <code>test/</code>, <code>testlab/</code> — fixtures and acceptance infrastructure.

Read [AGENTS.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md) before agent-driven repository changes.

## Rust checks

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

Some desktop/Linux builds need Tauri/WebKit/GTK system packages; CI keeps GUI-specific dependencies in the GUI job.

## React frontend

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

Run Tauri with the React config:

~~~bash
cd ../src-tauri
cargo tauri dev --config tauri.react.conf.json
~~~

The Svelte fallback is intentionally frozen during convergence; do not modify it casually.

## CLI

~~~bash
cargo run -p rimloc-cli -- --help
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
~~~

PO commands are interchange adapters, not the canonical project architecture.

## Adding a localization adapter

Start with [Localization Adapters](adapters.md) and the deeper [architecture document](../../architecture/LOCALIZATION_ADAPTERS.md).

The important rule: environment/game semantics stay in the adapter. Generic project/TM/glossary/editor behavior stays generic.

## Documentation

~~~bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs build --strict
~~~

The documentation site is the canonical public documentation source. GitHub Wiki is intentionally not used as a second technical source of truth.

## Security-sensitive changes

Changes involving filesystem writes, Tauri permissions/IPC, network/provider credentials, shell execution, parsing untrusted input, or automation bridges need explicit security review and adversarial tests.

See:

- [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md)
- <code>docs/security/</code>
- <code>deny.toml</code>

## CI / releases

CI is intended to validate current Rust + React code while avoiding unnecessary runner use for docs-only changes.

Release publication is currently controlled/manual during pre-beta. Do not create tags, publish crates, sign/notarize, or create a GitHub Release unless the owner explicitly requests a release operation.

## More developer docs

- [Adapter authoring](adapters.md)
- [Coverage & Codecov](coverage.md)
- [Legacy scan plugins](plugins.md)
- [Testing](../testing.md)
- [Docs style](docs_style.md)
- [Frontend boundary](../../architecture/FRONTEND_UI_BOUNDARY.md)
