---
title: Coverage и Codecov
---

# Coverage и Codecov

RimLoc использует [Codecov](https://codecov.io/gh/0-danielviktorovich-0/RimLoc) для отслеживания покрытия автоматическими тестами.

Coverage — **сигнал, а не оценка корректности**. Выполненная строка не доказывает правильность RimWorld semantics, filesystem safety, UX, accessibility или поведения в игре.

## Текущие coverage families

Workflow загружает три независимо сформированных отчёта:

- **rust** — Rust workspace под `crates/`, без Tauri GUI;
- **gui-rust** — desktop Rust/Tauri с отдельными GTK/WebKit/frontend prerequisites;
- **frontend-react** — весь production React source (`src/**/*.{ts,tsx}`), включая непроверенные экраны с 0%, а не скрывая их.

Codecov **Components** разделяют эти отчёты на области продукта без повторного прогона тестов:

- Domain & Core; Services; RimWorld parsers & validation; import/export formats;
- CLI; providers/config; plugin compatibility; desktop Rust/Tauri; React frontend.

**Flags** показывают семейства тестов, **Components** — архитектурные области.

## Формат отчёта

Rust CI использует `cargo-llvm-cov` и отправляет native Codecov coverage JSON вместо обычного LCOV. React использует Vitest V8 и загружает Istanbul JSON (`coverage/coverage-final.json`), одновременно создавая текстовый и LCOV-отчёты для локальной проверки.

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

## Доверие к конфигурации

Codecov читает repository policy только из default branch: после первых успешных Rust и desktop coverage upload на `main` включён `codecov.strict_yaml_branch: main`. Pull request может менять код, который генерирует coverage, но не может ослабить Codecov policy, которой оценивается этот же PR.

## Политика pre-beta

Statuses сначала **informational**.

Мы не ставим случайный глобальный gate "80%". Сначала собираем baseline на реальных PR, затем постепенно включаем:

1. project `target: auto` с небольшим допустимым regression;
2. разумный patch coverage для нового/изменённого кода;
3. более строгие component gates для safety-critical областей после стабилизации baseline;
4. только обоснованные исключения для generated/platform glue.

Цель — не максимизировать vanity percentage, а не допускать деградацию и видеть нетестированный код.

## React coverage

Job `frontend-react` из `.github/workflows/coverage.yml` выполняет
`npm test` в `gui/tauri-app/frontend-react` (Vitest V8, конфигурация
`test.coverage`) и загружает `coverage/coverage-final.json` под флагом
Codecov `frontend-react`. Дополнительная проверка JSON требует наличие
`App.tsx`, `Workspace.tsx`, `project.ts` и `entryUid.ts` в отчёте.

Покрытие включает **все production-файлы `src/**/*.{ts,tsx}`**, а не только
проверенные state-модули. Поэтому непроверенные экраны честно показывают
0%: это baseline, а не автоматическая оценка готовности к релизу.
При Data Safety review отдельно анализировать `project.ts` и `entryUid.ts`.
Зелёные Rust/Tauri-тесты сами по себе не доказывают покрытие React и
работоспособность настоящего собранного приложения.

## Pull Request: зелёные проверки — не конец ревью

- Проверять комментарии **Codecov и других ботов**, все статусы
  patch/project/flags/components, конкретные непокрытые изменённые строки и
  review threads на **точном HEAD PR**. `informational: true` позволяет
  зелёный статус даже ниже цели; успешный upload означает лишь очередь
  обработки, а не принятый отчёт.
- **`strict_yaml_branch: main` — намеренная политика**: до merge Codecov
  применяет YAML из `main`, а не новые настройки из PR. Проверять
  эффективные `ignore`/components/flags из `main` перед заявлениями,
  что новая область продукта учитывается.
- Исключение `ignore: "gui/tauri-app/frontend"` захватывает и
  `frontend-react`. Для legacy-папки нужно окончание `frontend/`;
  после обработки Codecov проверить, что React действительно измеряется.
  Статус "Coverage not affected" у флага требует расследования.
- Проверять фактическую таблицу **по файлам** и их перечень в JSON.
  `coverage.include` находится внутри `test.coverage`, а не в корне
  Vite config. Зелёный workflow с неправильным знаменателем не доказывает
  заявленные метрики.
- Если изменился знаменатель, baseline либо политика исключений,
  проценты «до/после» **нельзя напрямую сравнивать**. Не скрывать файлы,
  не уменьшать требования ради цифры; добавлять поведенческие и
  отрицательные тесты реальных рисков, документировать исключения.
- Покрытие дополняется реальными Tauri IPC-тестами, native WDIO,
  проверкой экспорта в RimWorld, CodeQL/зависимостей и UAT владельца.

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
