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

