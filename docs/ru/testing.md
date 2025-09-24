---
title: Тестирование и отчёты
---

# Тестирование и отчёты

На этой странице — как протестировать RimLoc локально и как оформлять полезные багрепорты с нужной диагностикой.

## Быстрый старт (для разработчиков)

```bash
# Собрать весь workspace
cargo build --workspace

# Запустить все тесты (unit + integration)
cargo test --workspace -- --nocapture

# Форматирование и линт без предупреждений
cargo fmt && cargo clippy --workspace --all-targets -- -D warnings
```

Полезные флаги:

- `-- --nocapture` показывает stdout тестов (удобно для локализации help).
- Запуск одного интеграционного теста: `cargo test -p rimloc-cli scan_picks_latest_version_by_default_and_flags_work`.

## Логи и диагностика

RimLoc пишет диагностику в stderr и в лог-файл с ротацией.

- `RUST_LOG=info|debug|trace` — уровень подробности в консоли (по умолчанию `info`).
- `RIMLOC_LOG_DIR=./logs` — директория для ежедневных логов (по умолчанию `./logs`).
- `RIMLOC_LOG_FORMAT=json` — переключить файловый лог в структурированный JSON (по умолчанию `text`).
- Для чистого копипаста можно отключить украшения UI:
  - `NO_COLOR=1` — без ANSI-цветов,
  - `NO_ICONS=1` — без символов ✔/⚠/✖.

При старте RimLoc печатает баннер с версией, `RIMLOC_LOG_DIR` и текущим `RUST_LOG` — это сразу даёт контекст в отчётах.

Совет для автоматизации: используйте `--quiet` вместе с `--format json`, чтобы stdout оставался машинно‑читаемым, а диагностические сообщения шли в stderr/лог.

## Сквозные проверки CLI

Для быстрых прогонов используйте фикстуру `test/TestMod`:

```bash
# Сканировать в JSON и сохранить копию на диск
rimloc-cli scan --root ./test/TestMod --format json --out-json ./logs/scan.json

# Валидация в текстовом формате
rimloc-cli validate --root ./test/TestMod --format text

# Строгая проверка плейсхолдеров в PO
rimloc-cli validate-po --po ./test/test-en.po --strict

# Экспорт в PO и обратный импорт в XML в режиме dry-run
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
rimloc-cli import-po --po ./logs/TestMod.po --mod-root ./test/TestMod --dry-run

# Сборка мода-перевода (dry run)
rimloc-cli build-mod --po ./logs/TestMod.po --out-mod ./logs/TestMod-ru --lang ru --dry-run
```

### Версионные моды

Если мод использует папки версий (`1.4`, `1.5`, `v1.6`):

```bash
rimloc-cli scan --root ./Mods/MyMod --game-version 1.4
rimloc-cli validate --root ./Mods/MyMod --include-all-versions
rimloc-cli export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --game-version v1.6
```

## JSON для автоматизации

- `scan --format json [--out-json <FILE>]` — массив юнитов; удобно класть в артефакты CI.
- `validate --format json` — структурированные проблемы (kind, key, path, line, message).
- `validate-po --format json [--strict]` — несоответствия плейсхолдеров между msgid/msgstr.

Пример:

```bash
rimloc-cli validate --root ./test/TestMod --format json | jq '.[] | select(.kind=="duplicate")'
```

## Шаблон багрепорта

Пожалуйста, указывайте следующее — это ускорит разбор:

1) Команда и полный вызов

```
rimloc-cli <command> <args>
```

2) Версии и окружение

- `rimloc-cli --version`
- ОС и shell
- `RUST_LOG`, `RIMLOC_LOG_DIR`, `NO_COLOR`, `NO_ICONS`

3) Ожидаемое и фактическое поведение (по 1–2 предложения)

4) Вложения

- `logs/rimloc.log` и вывод консоли (по возможности с `--ui-lang en`)
- Минимально воспроизводимый пример: маленький фрагмент мода или пара XML в `Languages/...`
- Для проблем с PO: небольшой `.po`, на котором воспроизводится баг

## Превью документации

Чтобы посмотреть сайт локально:

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve
```
