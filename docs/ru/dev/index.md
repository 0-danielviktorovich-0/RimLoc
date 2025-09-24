---
title: Гайд разработчика
---

# Гайд разработчика

Как собрать, протестировать и отладить RimLoc локально.

## ОС и тулчейн

- Рекомендуется: Linux или macOS (Rust stable).
- Windows: работает с MSVC; для комфортной UNIX‑среды советуем WSL2 (Ubuntu).
- Установка Rust через rustup:

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows (PowerShell): скачайте rustup-init.exe с https://rustup.rs
```

Проверка:

```bash
rustc -V
cargo -V
```

Дополнительно:

- VS Code + rust‑analyzer
- `cargo install cargo-watch`
- Python 3 + `pip` (для документации)

## Сборка и тесты

```bash
cargo build --workspace
cargo test --workspace
cargo fmt && cargo clippy --workspace --all-targets -- -D warnings
```

Запуск CLI на фикстуре:

```bash
cargo run -q -p rimloc-cli -- --quiet scan --root ./test/TestMod --format json | jq '.[0]'
```

## Окружение для отладки

- Логи:
  - `RUST_LOG=debug` (консоль в stderr)
  - `RIMLOC_LOG_DIR=./logs` (файловый лог; ежедневная ротация)
  - `RIMLOC_LOG_FORMAT=json` (структурированный лог в файл)
  - `NO_COLOR=1`, `NO_ICONS=1` — для “чистого” текста
  - `--quiet` — держит stdout чистым для JSON

Пример:

```bash
RUST_LOG=debug RIMLOC_LOG_DIR=./logs cargo run -q -p rimloc-cli -- --quiet validate --root ./test/TestMod --format json | jq .
```

Бэктрейсы и расширенные ошибки:

```bash
RUST_BACKTRACE=1 cargo run -p rimloc-cli -- validate --root ./test/TestMod
```

## Отладка через LLDB/GDB (опционально)

```bash
# lldb
rust-lldb target/debug/rimloc-cli -- --quiet scan --root ./test/TestMod

# gdb
rust-gdb target/debug/rimloc-cli --args --quiet scan --root ./test/TestMod
```

## Документация локально

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve
```

## Локализация (i18n)

- Строки CLI: `crates/rimloc-cli/i18n/en/rimloc.ftl` и зеркала по локалям.
- Смотрите Community → Localization и Translate RimLoc.
- Плейсхолдеры — `{name}`, `{0}`, `%s` оставляем без изменений (см. Guides → Плейсхолдеры).

## Типовые сценарии

### Экспорт → перевод → импорт

```bash
rimloc-cli --quiet export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --lang ru
rimloc-cli --quiet validate-po --po ./out/MyMod.po --strict
rimloc-cli --quiet import-po --po ./out/MyMod.po --out-xml ./out/MyMod.ru.xml
```

### Сборка автономного мода перевода

```bash
rimloc-cli --quiet build-mod --po ./out/MyMod.po --out-mod ./dist/MyMod-ru --lang ru --dedupe
```

## VS Code / VSCodium

VS Code и VSCodium (версия без брендинга и телеметрии) одинаково хорошо подходят для Rust. Рекомендуемые расширения:

- rust‑analyzer (официальная поддержка языка)
- CodeLLDB (отладчик)
- Even Better TOML (для Cargo.toml)
- Fluent (FTL) — подсветка и базовые проверки (например, "Fluent Support")

Сохраните файлы в `.vscode/` (VSCodium читает те же настройки).

Готовые примеры лежат в репозитории:

- `.vscode/tasks.example.json`
- `.vscode/launch.example.json`

Скопируйте их в `.vscode/tasks.json` и `.vscode/launch.json`, чтобы задействовать.

Пример `tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    { "label": "cargo build", "type": "shell", "command": "cargo build --workspace" },
    { "label": "cargo test",  "type": "shell", "command": "cargo test --workspace" },
    { "label": "cargo clippy", "type": "shell", "command": "cargo clippy --workspace --all-targets -- -D warnings" },
    { "label": "cargo fmt",    "type": "shell", "command": "cargo fmt" },
    { "label": "mkdocs serve", "type": "shell", "command": "python -m venv .venv && . .venv/bin/activate && pip install -r requirements-docs.txt && mkdocs serve" }
  ]
}
```

Пример `launch.json` (отладка `rimloc-cli`):

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug rimloc-cli (scan)",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/rimloc-cli",
      "args": ["--quiet", "scan", "--root", "${workspaceFolder}/test/TestMod", "--format", "json"],
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug", "RIMLOC_LOG_DIR": "${workspaceFolder}/logs" },
      "preLaunchTask": "cargo build"
    },
    {
      "name": "Debug rimloc-cli (validate)",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/rimloc-cli",
      "args": ["--quiet", "validate", "--root", "${workspaceFolder}/test/TestMod", "--format", "text"],
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug", "RIMLOC_LOG_DIR": "${workspaceFolder}/logs" },
      "preLaunchTask": "cargo build"
    }
  ]
}
```

Подсказки:

- Для отладки тестовых бинарников можно добавить спец‑конфигурации или компоновочные задачи.
- VSCodium использует те же файлы `.vscode/`.

Примечание по ОС: на Linux и macOS обычно чуть проще и комфортнее разрабатывать на Rust (инструменты и производительность). В Windows рекомендуем WSL2, если хочется UNIX‑среды.

## Профилирование

Быстрые flamegraph:

```bash
cargo install flamegraph
# Linux: нужен perf (sudo apt install linux-tools-...)
# macOS: dtrace (запуск от root) или Instruments

cargo flamegraph -p rimloc-cli -- --quiet scan --root ./test/TestMod --format json
```

Советы:

- Профилируйте сборку `--release`.
- Суужайте сценарий до одной команды (`scan` на большом моде и т.п.).
- Используйте `tracing` + `RUST_LOG=debug`, чтобы сопоставлять горячие места с логами.

## Публикация dev‑пререлиза (GitHub Actions)

Есть два способа выложить мультиархивные dev‑сборки:

1) Вручную (Release — dev pre‑release):
   - GitHub → Actions → «Release (dev pre-release)» → «Run workflow».
   - Тег вычисляется из версии Cargo и короткого SHA (например, `v0.0.1-dev.ab12c34`).
   - Артефакты для Linux/macOS/Windows (x86_64 + aarch64; для Linux есть ещё x86_64‑musl).
2) Автоматически (Nightly Dev Pre-release):
   - Пуш в ветку `develop`.
   - CI создаёт/обновляет пререлиз с такой же схемой тега.

В тело релиза добавляются SHA коммита и список целей. При необходимости можно отредактировать описание после завершения CI.

Скоро добавим скриншоты/GIF процесса «Run workflow» (встроим в доки, когда будут готовы).

### Проверка подписи (cosign, keyless)

Каждый архив подписан через Sigstore/cosign по OIDC GitHub (без приватных ключей). Рядом с артефактом лежат `.sig` и `.pem`.

Базовая проверка:

```bash
cosign verify-blob \
  --certificate dist/<ASSET>.pem \
  --signature   dist/<ASSET>.sig \
  dist/<ASSET>
```

Строгая проверка (с проверкой идентичности workflow в GitHub):

```bash
cosign verify-blob \
  --certificate-identity-regexp "https://github.com/.+/.+/.github/workflows/(release-dev|release-dev-auto).yml@.*" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  --certificate dist/<ASSET>.pem \
  --signature   dist/<ASSET>.sig \
  dist/<ASSET>
```

### SBOM

Для каждого артефакта генерируется SPDX JSON SBOM (`.spdx.json`) при помощи Syft. Его можно использовать для просмотра зависимостей/лицензий и сканирования уязвимостей (`grype`, `trivy`).

### Контрольные суммы

К каждому артефакту публикуется SHA256‑сумма (`.sha256`). Проверка:

```bash
# Linux
sha256sum -c dist/<ASSET>.sha256

# macOS
shasum -a 256 -c dist/<ASSET>.sha256

# Windows (PowerShell)
Get-Content dist\<ASSET>.sha256
Get-FileHash dist\<ASSET> -Algorithm SHA256
```

### Профилирование в Windows (WPA/ETW)

В Windows нет нативного `perf`/`dtrace`, но можно писать ETW‑трейсы и смотреть их в WPA:

- Установите Windows Performance Toolkit (WPT) через установщик Windows SDK (выберите компонент «Windows Performance Toolkit»).

Запись из PowerShell:

```powershell
# Запустить лёгкий CPU‑профиль
wpr -start CPU -filemode

# В другом окне — выполнить нагрузку
cargo run -q -p rimloc-cli -- --quiet scan --root .\test\TestMod --format json > $null

# Остановить запись и сохранить трейc
wpr -stop rimloc.etl
```

Откройте `rimloc.etl` в Windows Performance Analyzer (WPA) и изучите CPU Usage (Sampled) и стеки вызовов.

Альтернатива: PerfView (https://github.com/microsoft/perfview)

```powershell
PerfView.exe run /NoGui /AcceptEULA -- cargo run -p rimloc-cli -- --quiet validate --root .\test\TestMod --format text
```

Вариант через WSL2: используйте Linux‑инструменты (`perf`, `cargo flamegraph`) внутри WSL2, указав путь к репозиторию.

## Отладка тестов в VS Code/VSCodium

CodeLLDB умеет запускать тесты через Cargo. Пример `launch.json` для отладки конкретного теста:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug test: cli_integration::validate_json_emits_structured_issues",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["test", "--no-run", "--package", "rimloc-cli", "--test", "cli_integration"],
        "filter": { "name": "cli_integration", "kind": "test" }
      },
      "args": ["--nocapture", "validate_json_emits_structured_issues"],
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug" },
      "console": "integratedTerminal"
    }
  ]
}
```

Если версия CodeLLDB не поддерживает `cargo`‑launcher, сначала соберите тесты и укажите путь к бинарнику в `target/debug/deps/`:

```bash
cargo test -p rimloc-cli --test cli_integration --no-run
ls target/debug/deps/cli_integration-*
```

Затем пропишите этот путь в `program` и передайте `args`: `["--nocapture", "validate_json_emits_structured_issues"]`.
