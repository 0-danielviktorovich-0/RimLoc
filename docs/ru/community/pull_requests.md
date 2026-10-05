---
title: Пул-реквесты
---

# Pull Requests

RimLoc использует отдельные ветки/PR, чтобы code, docs, security и UI-evidence можно было review-ить независимо.

## Перед PR

1. Создайте branch от актуальной integration/main линии задачи.
2. Держите scope узким.
3. Прочитайте [CONTRIBUTING](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/CONTRIBUTING.md) и [AGENTS](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md), если работает coding agent.
4. Запустите проверки для затронутой области.
5. При user-facing изменении обновите EN/RU docs.

Не смешивайте случайный refactor, dependency sweep и feature в один PR.

## Проверки

### Rust/domain/services

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

### React UI

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

Для видимых изменений приложите screenshots/same-state evidence.

### Документация

~~~bash
mkdocs build --strict
~~~

### Security-sensitive изменения

Опишите safety impact и приложите релевантные containment/IPC/dependency/secret/adversarial tests.

## Architecture checklist

PR должен сохранять границы:

- domain behavior — Rust/shared services;
- React/CLI — product surfaces, не отдельные реализации домена;
- PO/CSV/XLIFF — interchange adapters, если задача не про сам формат;
- RimWorld semantics — за adapter boundary;
- game/mod source — read-only;
- automation/test hooks не попадают в production artifact.

## Описание PR

Укажите:

- problem/outcome;
- важные решения;
- tests/evidence;
- screenshots для UI;
- migration/breaking impact;
- security impact;
- linked issues.

## Большие миграции

Иногда migration checkpoint неизбежно большой. Изолируйте его от посторонних изменений, оставьте durable report и после него возвращайтесь к небольшим PR. Сотни файлов на PR не должны становиться обычным workflow.
