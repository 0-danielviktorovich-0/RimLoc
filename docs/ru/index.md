---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/danielviktorovich) [![Ko-fi](https://img.shields.io/badge/Ko-fi-support-FF5E5B?logo=kofi&logoColor=white)](https://ko-fi.com/danielviktorovich) [![Discord](https://img.shields.io/badge/discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/g8w4fJ8b)

RimLoc помогает моддерам RimWorld поддерживать переводы в актуальном состоянии, проверять их и готовить к передаче переводчикам.

[:material-play-circle: Начать перевод](getting-started.md){ .md-button .md-button--primary }
[:material-cog: Конфигурация (rimloc.toml)](guide/configuration.md){ .md-button }

## 🚀 Быстрый старт

Новичкам — сюда: getting-started.md. Там пошаговый гайд, примеры команд и советы.


## Зачем нужен RimLoc?

- За один запуск собирает все строки из `Languages/*/{Keyed,DefInjected}`.
- Не допускает проблемных релизов, заранее находя дубликаты, пустые значения и ошибки плейсхолдеров.
- Экспортирует и импортирует пакеты PO/CSV, удобные для переводчиков.
- Может собрать переводческий мод напрямую из готового `.po` файла.
- CLI уже локализован (английский и русский) и использует стек Fluent.

## Команды в двух словах

См. обзор CLI: cli/index.md. Полные страницы с примерами: Scan · Validate · Validate PO · Export/Import · Build Mod.

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
