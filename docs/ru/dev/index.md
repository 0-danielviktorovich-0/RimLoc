---
title: Гайд разработчика
---

# Гайд разработчика

RimLoc — Rust localization core/services + desktop-приложение на Tauri 2.

## Текущая архитектура

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

Основные каталоги:

- <code>crates/</code> — Rust domain/core/services/adapters/import/export/CLI;
- <code>gui/tauri-app/frontend-react/</code> — будущий production React UI;
- <code>gui/tauri-app/frontend-v2/</code> — замороженный Svelte fallback;
- <code>gui/tauri-app/src-tauri/</code> — Tauri bridge/config;
- <code>docs/</code> — канонические MkDocs docs;
- <code>test/</code>, <code>testlab/</code> — fixtures/acceptance.

Перед agent-driven изменениями прочитайте [AGENTS.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md).

## Rust checks

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

GUI/Linux build может требовать Tauri/WebKit/GTK системные зависимости.

## React frontend

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

React Tauri dev:

~~~bash
cd ../src-tauri
cargo tauri dev --config tauri.react.conf.json
~~~

Svelte fallback заморожен на время convergence.

## CLI

~~~bash
cargo run -p rimloc-cli -- --help
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
~~~

PO-команды — interchange adapters, а не каноническая project architecture.

## Новый LocalizationAdapter

Начните с [гайда по адаптерам](adapters.md) и [architecture document](../../architecture/LOCALIZATION_ADAPTERS.md).

Главное правило: game/environment semantics живут в adapter, а project/TM/glossary/editor остаются generic.

## Документация

~~~bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs build --strict
~~~

MkDocs — канонический публичный источник документации. GitHub Wiki не используется как второй технический source of truth.

## Security-sensitive изменения

Filesystem writes, Tauri permissions/IPC, network/provider credentials, shell execution, parsing untrusted input и automation bridges требуют отдельного security review и adversarial tests.

См.:

- [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md)
- <code>docs/security/</code>
- <code>deny.toml</code>

## CI / releases

CI должен проверять актуальный Rust + React stack и не тратить runner minutes на docs-only изменения.

Публикация release сейчас контролируемая/manual. Не создавайте tag/Release и не публикуйте crates без отдельного owner request.

## Ещё

- [Adapter authoring](adapters.md)
- [Coverage & Codecov](coverage.md)
- [Legacy scan plugins](plugins.md)
- [Testing](../testing.md)
- [Docs style](docs_style.md)
- [Frontend boundary](../../architecture/FRONTEND_UI_BOUNDARY.md)
