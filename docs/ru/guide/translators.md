---
title: Для переводчиков
---

# Перевод RimWorld-проекта с RimLoc

Здесь описан рабочий путь переводчика. Знать RimWorld XML не обязательно, и **PO тоже не обязателен**, если вы не хотите работать через внешний CAT.

## Рекомендуемый путь: desktop project

### 1. Выберите источник

Создайте новый проект или откройте/обновите существующий перевод.

Выберите поддерживаемый текущей сборкой RimWorld source type (например мод или existing translation). RimLoc строит канонический inventory источника.

### 2. Выберите target language

Один проект может хранить несколько target locale. Переключение языка не должно перетирать перевод другого locale.

### 3. Переводите в редакторе

В workspace доступны по мере текущих capabilities:

- source/target;
- контекст и source information;
- status/review;
- glossary;
- TM suggestions;
- validation findings.

Для обычного сценария перевод не обязан покидать RimLoc.

### 4. Validation перед сборкой

Запустите Checks/validation. Особое внимание — placeholders, tags и source-changed entries.

### 5. Безопасный build/export

Выбирайте отдельный output-каталог. Оригинальный Workshop/game source не должен переписываться.

Перед публикацией обязательно протестируйте результат в RimWorld.

## Опционально: PO / внешний CAT

Этот путь нужен, если удобнее Poedit или другой CAT.

Экспорт:

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

Проверка:

~~~bash
rimloc-cli validate-po --po ./work/MyMod.ru.po --strict
~~~

Dry-run импорта:

~~~bash
rimloc-cli import-po \
  --po ./work/MyMod.ru.po \
  --mod-root ./work/MyMod-copy \
  --lang ru \
  --dry-run
~~~

Здесь PO — handoff/interchange format, а не база проекта RimLoc.

## Сборка из готового Languages-дерева

Если перевод уже существует в RimWorld XML, PO не нужен:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyModTranslated \
  --out-mod ./dist/MyMod-Russian \
  --lang ru \
  --dry-run
~~~

Убирайте <code>--dry-run</code> только после проверки плана.

## Обновление существующего перевода

При обновлении source mod важно сохранить ручную работу и различить:

- unchanged;
- source-changed;
- new;
- obsolete/orphan;
- ambiguous.

Используйте dedicated existing/update workflow, а не начинайте перевод заново.

## Практические советы

- Считайте исходные моды read-only.
- Не переводите placeholders.
- После bulk/import/AI изменений снова запускайте validation.
- Проверяйте результат в игре: контекст и переносы не видны из XML.
- AI/provider output считайте draft, пока человек его не review-нул.

## См. также

- [Начало работы](../getting-started.md)
- [Desktop GUI](gui.md)
- [Обновление перевода](../tutorials/update_translations.md)
- [Плейсхолдеры](placeholders.md)
- [Решение проблем](../troubleshooting.md)
