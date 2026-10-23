---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/ru/)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-GitHub-%23ea4aaa?logo=github-sponsors)](https://github.com/sponsors/0-danielviktorovich-0)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/danielviktorovich)

RimLoc — **графическая студия перевода модов RimWorld**: работа со строками, проверка и экспорт прямо в приложении, без терминала. Rust CLI остаётся необязательным дополнением.

[:material-monitor: Графический интерфейс — начать здесь](guide/gui.md){ .md-button .md-button--primary }
[:material-play-circle: Начало работы](getting-started.md){ .md-button }
[:material-console: Необязательный CLI](cli/index.md){ .md-button }

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

## ИИ-перевод

Сейчас в GUI работает **Chat Batch без API-ключа**: структурированный
промпт, копирование в AI-чат, импорт ответа, проверка и применение.
Настроить и проверить провайдера уже можно, но **прямые автоматические
переводы внутри GUI ещё не подключены**. Планируются официальный вход
через ChatGPT для подходящих пользователей open-source версии, свои
API-ключи, локальные модели и существующий вариант через чат без API.

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
