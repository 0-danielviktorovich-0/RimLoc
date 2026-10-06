---
title: Coverage и Codecov
---

# Coverage и Codecov

RimLoc использует [Codecov](https://codecov.io/gh/0-danielviktorovich-0/RimLoc) для отслеживания покрытия автоматическими тестами.

Coverage — **сигнал, а не оценка корректности**. Выполненная строка не доказывает правильность RimWorld semantics, filesystem safety, UX, accessibility или поведения в игре.

## Текущие coverage families

Workflow загружает два независимых Rust-отчёта:

- **rust** — workspace под `crates/`, без Tauri GUI crate;
- **gui-rust** — desktop Rust/Tauri слой с отдельными GTK/WebKit/frontend prerequisites.

Codecov **Components** режут эти отчёты по архитектурным подсистемам без повторного запуска тестов для каждого crate:

- Domain & Core;
- Services;
- RimWorld parsers & validation;
- import/export formats;
- CLI;
- providers/config;
- plugin compatibility;
- desktop Rust/Tauri.

То есть **Flags** показывают test families, а **Components** — продуктовые подсистемы.

## Формат отчёта

Rust CI использует `cargo-llvm-cov` и отправляет native Codecov coverage JSON вместо обычного LCOV.

Причины:

- сохраняется более богатая LLVM region information;
- мы не включаем нестабильный `--branch` режим cargo-llvm-cov только ради цифры;
- Codecov получает формат напрямую без преждевременного сведения к line-only LCOV.

Локально удобнее HTML:

~~~bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --exclude rimloc-gui \
  --html
~~~

Формат как в CI:

~~~bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --exclude rimloc-gui \
  --codecov \
  --output-path codecov.json
~~~

## Аутентификация

Upload использует **GitHub OIDC**, поэтому repository secret `CODECOV_TOKEN` не нужен.

Same-repo PR/push получают OIDC identity. Fork PR не получает repository secrets; workflow допускает public/tokenless путь Codecov и не должен ломать внешний PR только из-за отсутствия upload-auth.

## Политика pre-beta

Statuses сначала **informational**.

Мы не ставим случайный глобальный gate "80%". Сначала собираем baseline на реальных PR, затем постепенно включаем:

1. project `target: auto` с небольшим допустимым regression;
2. разумный patch coverage для нового/изменённого кода;
3. более строгие component gates для safety-critical областей после стабилизации baseline;
4. только обоснованные исключения для generated/platform glue.

Цель — не максимизировать vanity percentage, а не допускать деградацию и видеть нетестированный код.

## React coverage

React R1 уже имеет build/typecheck acceptance, но пока нет стабильного unit/component coverage report в package scripts.

Когда test stack будет готов:

~~~text
npm run test:coverage
→ frontend-react/coverage/lcov.info (или другой Codecov-supported report)
→ Codecov flag: react
→ Codecov component: frontend-react
~~~

Не добавляем второй test framework только ради Codecov.

## Test Analytics — позже

Codecov умеет принимать JUnit-style test results и показывать duration/failure rate/flaky tests. Подключать это стоит, когда Rust/React runner уже выдаёт нормальный JUnit.

Не надо повторно гонять весь suite только ради Test Analytics.

## Что coverage не заменяет

Отдельно нужны:

- same-corpus RimWorld differential;
- in-game/runtime verification;
- path/symlink/adversarial filesystem tests;
- CodeQL, dependency review, Dependabot и cargo-deny;
- owner visual review;
- accessibility review;
- macOS/Windows artifact acceptance.

## CI files

- `.github/workflows/coverage.yml` — генерация/upload;
- `codecov.yml` — policy, components, flags, PR comments.

Workflow сначала валидирует `codecov.yml`, затем запускает coverage.
