---
title: Всё про PO‑файлы
---

# Всё про PO‑файлы

!!! info "Термины"
    Впервые видите `.po`? Начните со Словаря: ../glossary.md#po-portable-object

Коротко о том, что такое `.po`, зачем RimLoc его использует и как удобно редактировать.

## Что такое PO?

PO (Portable Object) — текстовый формат из экосистемы GNU gettext. Каждая запись включает:

```
#: <ссылка на исходный файл>
msgctxt "<опциональный контекст>"
msgid "<исходный текст>"
msgstr "<перевод>"
```

RimLoc добавляет комментарий `#: path:line` и уникальный `msgctxt`, который сочетает ключ и относительный путь — так записи стабильнее при изменениях.

## Почему PO в RimLoc?

- Удобно для переводчиков — много редакторов поддерживают PO.
- Хранит контекст, ссылки и ключи в одном месте.
- Легко смотреть diff и делать ревью в PR.

## Пример записи RimLoc

```
#: Mods/MyMod/Languages/English/Keyed/Gameplay.xml:42
msgctxt "Greeting|Keyed/Gameplay.xml:42"
msgid "Hello, {PAWN_label}!"
msgstr ""
```

Переводим, заполняя `msgstr`, но плейсхолдеры (например, `{PAWN_label}`) не меняем.

## Рабочий процесс с RimLoc

- Экспорт:

```bash
rimloc-cli --quiet export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --lang ru
```

- Редактирование в удобном редакторе (см. ниже).

- Проверка плейсхолдеров (строгий режим для CI):

```bash
rimloc-cli --quiet validate-po --po ./out/MyMod.po --strict --format json | jq .
```

- Импорт обратно в XML (в один файл или в структуру мода):

```bash
# Один XML для ревью
rimloc-cli --quiet import-po --po ./out/MyMod.po --out-xml ./out/MyMod.ru.xml

# Обновить структуру мода (с бэкапами)
rimloc-cli --quiet import-po --po ./out/MyMod.po --mod-root ./Mods/MyMod --backup
```

## Рекомендуемые редакторы

- Poedit — популярный кроссплатформенный редактор для PO.
- Gtranslator (GNOME), Lokalize (KDE) — нативные приложения под Linux.
- VS Code — есть расширения для “gettext/PO” (подсветка и базовое редактирование).
- CLI‑утилиты: `msgfmt`, `msgcat`, `msgconv` из gettext (для продвинутых).

Совет: PO должны быть в UTF‑8. Если инструмент сохранил другую кодировку, конвертируйте:

```bash
msgconv --to-code=utf-8 ./in.po > ./out.po
```

## Плейсхолдеры

Не меняйте плейсхолдеры (например, `{count}`, `%s`). Подробности — в разделах «Гайды → Плейсхолдеры».

См. также: ../cli/export_import.md · ../cli/validate_po.md
