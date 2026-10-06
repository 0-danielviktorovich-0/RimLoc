---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/ru/)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-GitHub-%23ea4aaa?logo=github-sponsors)](https://github.com/sponsors/0-danielviktorovich-0)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/danielviktorovich)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-support-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/danielviktorovich)

RimLoc — **RimWorld-first рабочая станция локализации** с local-first Rust-ядром, desktop UI и архитектурой адаптеров для будущих игр и приложений.

[:material-play-circle: Начало работы](getting-started.md){ .md-button .md-button--primary }
[:material-monitor: Desktop GUI](guide/gui.md){ .md-button }
[:material-console: CLI](cli/index.md){ .md-button }

!!! warning "Pre-beta"
    Новый десктопный интерфейс ещё доводится на React R1. Стабильных публичных установщиков пока нет; для тестирования используйте актуальную сборку из исходников/owner artifact.

## Два основных способа работы

### Проект в GUI

Для обычной работы PO **не обязателен**.

1. Создайте или откройте RimLoc-проект.
2. Выберите источник RimWorld и целевой язык(и).
3. Переводите прямо в workspace.
4. Запустите validation/review.
5. Соберите или экспортируйте результат.

Project state, переводы, glossary, TM, revisions и review живут в канонической модели RimLoc, а не в PO-файле.

### CLI / обмен форматами

CLI удобен для автоматизации и внешних CAT-процессов:

- <code>scan</code>, <code>validate</code>, <code>diff-xml</code>;
- опциональные <code>export-po</code> / <code>import-po</code>;
- <code>build-mod</code> из PO **или из готового Languages-дерева**;
- диагностика и maintenance-команды.

Используйте PO, когда нужен Poedit/CAT или внешний обмен. Это не обязательный формат проекта.

## Текущий фокус

Первая публичная beta сознательно RimWorld-first:

- современные RimWorld layout;
- Core/DLC/mod/language-pack источники;
- обновление существующего перевода;
- multi-target;
- glossary и Translation Memory;
- validation и безопасная сборка/экспорт;
- security hardening;
- практические сравнения с существующими RimWorld-инструментами.

Будущие адаптеры — архитектурная цель, а не текущая заявка на поддержку.

## Куда идти дальше

- [Начало работы](getting-started.md)
- [Гайд переводчика](guide/translators.md)
- [Desktop GUI](guide/gui.md)
- [CLI](cli/index.md)
- [Решение проблем](troubleshooting.md)
- [Гайд разработчика](dev/index.md)
- [Добавление адаптера](dev/adapters.md)
- [Поддержать RimLoc](community/support.md)

!!! tip "Помочь перевести RimLoc"
    См. [Localization guide](community/localization.md). Self-localization RimLoc использует те же базовые project-концепции, что и другие адаптеры.
