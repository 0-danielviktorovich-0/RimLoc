---
title: PO-файлы (опциональный обмен)
---

# PO-файлы в RimLoc

PO (Portable Object) — один из **interchange formats** RimLoc. Он удобен для Poedit/CAT, но **не является каноническим форматом проекта RimLoc** и не обязателен для обычного desktop-перевода.

## Когда PO полезен

Используйте PO, если нужно:

- отдать перевод во внешний CAT;
- ревьюить один текстовый файл в Git;
- импортировать существующий gettext-oriented workflow;
- сохранить совместимость с привычными translator tools.

Если переводите прямо в RimLoc desktop project, PO можно вообще не использовать.

## Структура

~~~text
#: path/to/source.xml:42
msgctxt "stable-context"
msgid "Hello, {PAWN_label}!"
msgstr "Привет, {PAWN_label}!"
~~~

Плейсхолдеры/tags не меняйте.

## Экспорт

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

## Проверка

~~~bash
rimloc-cli validate-po --po ./work/MyMod.ru.po --strict
~~~

## Импорт

Используйте working copy / отдельный output, а не оригинальный Workshop source:

~~~bash
rimloc-cli import-po \
  --po ./work/MyMod.ru.po \
  --mod-root ./work/MyMod-copy \
  --lang ru \
  --dry-run
~~~

## Сборка без PO

Если translated XML уже есть:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

Правильная модель:

~~~text
canonical RimLoc project
        ↕
   import/export adapters
   PO · CSV · XLIFF · XML · ...
~~~

а не “RimLoc project = PO”.

См. [PO export/import](../cli/export_import.md) и [плейсхолдеры](placeholders.md).
