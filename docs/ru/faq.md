---
title: FAQ
---

# Часто задаваемые вопросы

## PO обязателен?

Нет.

Desktop project workflow RimLoc использует каноническую project model, а не PO. Можно переводить прямо в RimLoc, там же проходить validation/review и затем собирать/экспортировать результат.

PO нужен, когда удобен Poedit/CAT или внешний обмен.

## Почему в CLI всё ещё есть export-po/import-po?

Потому что это полезные и уже стабильные interoperability-команды. Их наличие не делает PO внутренним форматом проекта.

## Можно собрать translation mod без PO?

Да, если translated RimWorld XML уже существует:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

## Какой GUI сейчас основной?

React R1 — будущий production UI. Предыдущий Svelte временно остаётся frozen fallback.

При тестировании packaged app смотрите artifact/build identity, а не только имя <code>.app</code>.

## RimLoc навсегда только для RimWorld?

Первая production-цель и первая beta — RimWorld-first.

Core проектируется через adapters, чтобы позже можно было добавлять другие игры/приложения без переписывания editor/TM/glossary/project model. Другие игры пока не заявляются как поддерживаемые.

## Уже есть стабильный desktop release?

Пока нет. RimLoc — pre-beta. Исторические alpha/dev релизы существуют, но активная React/Rust линия новее.

## scan и validate — в чём разница?

- <code>scan</code> собирает translation units.
- <code>validate</code> запускает QA и показывает findings/errors.

## Можно сначала посмотреть, что будет записано?

Используйте <code>--dry-run</code>, где он поддерживается, и отдельные working/output каталоги.

## Как посмотреть изменения source между версиями мода?

Основной путь — existing/update workflow в desktop UI. Для низкоуровневой диагностики:

~~~bash
rimloc-cli diff-xml --root ./Mods/MyMod --format text
~~~

## Куда сообщать об уязвимости?

Не публикуйте exploit в обычном issue. Следуйте [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md).
