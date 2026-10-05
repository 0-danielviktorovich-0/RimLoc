---
title: Правила оформления Issues
---

# Issues

GitHub Issues используются для воспроизводимых багов, product requests и проблем документации.

## Bug report

Используйте repository bug form. Полезный отчёт содержит:

- exact version / commit / artifact identity;
- область: React desktop, fallback, CLI, adapter, validation, build/export, TM/glossary, docs;
- ОС;
- UI steps или CLI command;
- expected/actual;
- маленький sanitized repro;
- screenshot;
- sanitized diagnostics/logs.

Для desktop pre-beta недостаточно написать «последняя версия» — нужен build identity.

## Security

Не публикуйте exploit details, secrets и чувствительные local data. Следуйте [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md).

## Feature request

Сначала опишите workflow/problem.

Для нового game/application adapter укажите:

- игру/приложение;
- source formats;
- existing translation format;
- version/dependency semantics;
- build/export target;
- маленький реальный пример.

Так проще понять, нужен ли simple adapter, сложный built-in adapter или будущий plugin protocol.

## Ошибка документации

Укажите exact page/link и что именно устарело/вводит в заблуждение.

Канонический публичный source — <code>docs/</code> → MkDocs. GitHub Wiki не поддерживается как второй технический источник истины.

## Перед созданием

Проверьте open/closed issues. Если проблема уже есть, добавьте новое evidence/repro туда.
