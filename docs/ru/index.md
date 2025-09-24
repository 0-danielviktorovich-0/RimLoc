---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/)

RimLoc помогает моддерам RimWorld поддерживать переводы в актуальном состоянии, проверять их и готовить к передаче переводчикам.

## Зачем нужен RimLoc?

- За один запуск собирает все строки из `Languages/*/{Keyed,DefInjected}`.
- Не допускает проблемных релизов, заранее находя дубликаты, пустые значения и ошибки плейсхолдеров.
- Экспортирует и импортирует пакеты PO/CSV, удобные для переводчиков.
- CLI уже локализован (английский и русский) и использует стек Fluent.

## Быстрый старт

```bash
cargo install rimloc-cli
rimloc-cli scan --root ./test/TestMod --format json | jq '.[0]'
rimloc-cli validate --root ./test/TestMod
```

- `scan` собирает единицы перевода и выводит CSV (или JSON при `--format json`).
- `validate` выполняет проверку качества и возвращает код `1`, если найдены ошибки.
- Для экспериментов воспользуйтесь тестовым модом `test/TestMod`.

Нужно подготовить пакет для переводчиков?

```bash
rimloc-cli export-po --root ./test/TestMod --out ./logs/TestMod.po --single-po
```

## Основные команды

| Команда | Что делает | Примечание |
|---------|-------------|------------|
| `scan` | Собирает строки из XML. | Добавьте `--out-csv`, чтобы сохранить файл вместе с выводом. |
| `validate` | Находит дубликаты, пустоты и ошибки плейсхолдеров. | С `--format json` удобно подключать к CI. |
| `validate-po` | Сравнивает плейсхолдеры в `msgid`/`msgstr` PO. | Флаг `--strict` превращает предупреждения в ошибки. |
| `export-po` | Формирует пакеты PO/CSV. | `--single-po` складывает всё в один файл. |
| `import-po` | Записывает обновления из PO обратно в XML. | `--dry-run` показывает изменения без записи. |

## Что дальше?

- Прочитайте [обзор CLI](cli/index.md), чтобы узнать об опциях и примерах.
- Переходите напрямую: [Сканирование](cli/scan.md) · [Проверка](cli/validate.md) · [Проверка PO](cli/validate_po.md) · [Экспорт / Импорт](cli/export_import.md)
- Обновляйте документацию локально через `mkdocs serve`, редактируя файлы в `docs/en/` и `docs/ru/`.

!!! tip "Где находится исходный код CLI?"
    Бинарные файлы лежат в `crates/rimloc-cli`, а фикстуры для экспериментов — в каталоге `test/`.

## Вклад в документацию

Нашли неточность или хотите добавить пример? [Отредактируйте страницу на GitHub](https://github.com/0-danielviktorovich-0/RimLoc/tree/main/docs/ru/index.md) или загляните в гайд [AGENTS.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md).
