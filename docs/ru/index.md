---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/danielviktorovich) [![Ko‑fi](https://img.shields.io/badge/Ko%E2%80%91fi-support-FF5E5B?logo=kofi&logoColor=white)](https://ko-fi.com/danielviktorovich)

RimLoc помогает моддерам RimWorld поддерживать переводы в актуальном состоянии, проверять их и готовить к передаче переводчикам.

## Зачем нужен RimLoc?

- За один запуск собирает все строки из `Languages/*/{Keyed,DefInjected}`.
- Не допускает проблемных релизов, заранее находя дубликаты, пустые значения и ошибки плейсхолдеров.
- Экспортирует и импортирует пакеты PO/CSV, удобные для переводчиков.
- Может собрать переводческий мод напрямую из готового `.po` файла.
- CLI уже локализован (английский и русский) и использует стек Fluent.

## Быстрый старт

```bash
cargo install rimloc-cli
rimloc-cli scan --root ./test/TestMod --format json | jq '.[0]'
rimloc-cli validate --root ./test/TestMod
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
rimloc-cli build-mod --po ./logs/TestMod.po --out-mod ./logs/TestMod-ru --lang ru --dry-run
```

- `scan` собирает единицы перевода и выводит CSV (или JSON при `--format json`).
- `validate` выполняет проверку качества и возвращает код `1`, если найдены ошибки.
- `export-po` формирует единый `.po` для передачи переводчикам или CAT-системам.
- `build-mod --dry-run` показывает, каким будет отдельный переводческий мод.
- Для экспериментов воспользуйтесь тестовым модом `test/TestMod`.

Нужно подготовить пакет для переводчиков?

```bash
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
```

Нужно собрать отдельный мод-перевод?

```bash
rimloc-cli build-mod --po ./logs/TestMod.po --out-mod ./logs/TestMod-ru --lang ru
```

## Основные команды

| Команда | Что делает | Примечание |
|---------|-------------|------------|
| `scan` | Собирает строки из XML. | Добавьте `--out-csv` или `--out-json`, чтобы сохранить файл вместе с выводом. |
| `validate` | Находит дубликаты, пустоты и ошибки плейсхолдеров. | `--format json` удобно подключать к CI, `--source-lang` задаёт базовый язык. |
| `validate-po` | Сравнивает плейсхолдеры в `msgid`/`msgstr` PO. | Флаг `--strict` превращает предупреждения в ошибки. |
| `export-po` | Формирует единый PO-файл. | Требуются `--root` и `--out-po`; добавьте `--lang`, чтобы заполнить заголовок. |
| `import-po` | Записывает обновления из PO обратно в XML. | `--dry-run` показывает изменения, `--single-file` складывает всё в `_Imported.xml`. |
| `build-mod` | Собирает самостоятельный мод-перевод из `.po`. | `--dry-run` печатает план, `--package-id` и `--rw-version` легко кастомизировать. |

## Что дальше?

- Прочитайте [обзор CLI](cli/index.md), чтобы узнать об опциях и примерах.
- Переходите напрямую: [Сканирование](cli/scan.md) · [Проверка](cli/validate.md) · [Проверка PO](cli/validate_po.md) · [Экспорт / Импорт](cli/export_import.md) · [Сборка мода](cli/build_mod.md)
- Обновляйте документацию локально через `mkdocs serve`, редактируя файлы в `docs/en/` и `docs/ru/`.

!!! tip "Помогите перевести RimLoc"
    Хотите видеть RimLoc на своём языке? Загляните в раздел [Localization](community/localization.md). Перевод можно сделать прямо через веб‑редактор GitHub без локальной настройки.

!!! tip "Где находится исходный код CLI?"
    Бинарные файлы лежат в `crates/rimloc-cli`, а фикстуры для экспериментов — в каталоге `test/`.

## Вклад в документацию

Нашли неточность или хотите добавить пример? [Отредактируйте страницу на GitHub](https://github.com/0-danielviktorovich-0/RimLoc/tree/main/docs/ru/index.md) или загляните в гайд [AGENTS.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/AGENTS.md).
