---
title: Coverage и Codecov
---

# Coverage и Codecov

RimLoc использует [Codecov](https://codecov.io/gh/0-danielviktorovich-0/RimLoc) как dashboard покрытия автоматическими тестами.

Coverage полезен, но **не является оценкой корректности продукта**. Выполненная строка кода не доказывает правильность RimWorld semantics, filesystem safety, UX, accessibility или поведения в игре.

## Текущие coverage families

Workflow загружает отдельные отчёты для:

- **Rust workspace** — generic crates из \`crates/\`;
- **Desktop Rust / Tauri** — Rust desktop layer с дополнительными WebKitGTK/GTK/frontend prerequisites.

Codecov Components группируют данные по подсистемам: Domain & Core, Services, RimWorld parsing/validation, formats, CLI, providers/config, plugins и desktop Rust.

## Аутентификация

Upload использует GitHub OIDC. Repository secret \`CODECOV_TOKEN\` не нужен.

Для публичных fork PR workflow рассчитан на public/tokenless upload, а не на выдачу секретов форку.

## Политика pre-beta

Project, patch и component statuses сначала **informational**, пока накапливается стабильный baseline.

Мы сознательно не ставим случайный глобальный gate «80%». После нескольких репрезентативных PR план:

1. блокировать существенное падение overall coverage через \`target: auto\`;
2. добавить разумный patch-coverage gate для нового кода;
3. сделать требования строже для safety-critical компонентов;
4. подключить React unit/component coverage, когда React test runner начнёт выдавать LCOV.

Coverage должен улучшать качество, а не превращаться в vanity metric.

## React coverage

React R1 пока имеет build/typecheck acceptance, но стабильного unit/component coverage report ещё нет.

Будущий contract:

~~~text
npm run test:coverage
→ gui/tauri-app/frontend-react/coverage/lcov.info
→ Codecov flag/component: react
~~~

Используем существующий frontend test framework; второй framework только ради Codecov не добавляем.

## Что coverage не заменяет

Отдельно нужны:

- same-corpus RimWorld differential;
- in-game/runtime verification;
- security/adversarial tests;
- CodeQL/Dependabot;
- owner visual review;
- accessibility review;
- macOS/Windows artifact acceptance.

## Локальный Rust coverage

После установки \`cargo-llvm-cov\`:

~~~bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --exclude rimloc-gui \
  --html
~~~

Точная CI-конфигурация: \`.github/workflows/coverage.yml\` и \`codecov.yml\`.
