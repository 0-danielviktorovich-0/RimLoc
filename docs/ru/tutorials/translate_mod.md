---
title: Как перевести мод
---

# Как перевести RimWorld-мод с нуля

Для большинства пользователей основной путь — desktop project workflow. PO опционален.

## Desktop workflow

1. Откройте RimLoc → **Новый перевод**.
2. Выберите RimWorld mod/source.
3. Выберите target locale.
4. Переводите прямо в workspace.
5. Проверяйте source/context, glossary/TM suggestions.
6. Запустите validation.
7. Build/export в отдельный output-каталог.
8. Проверьте перевод в RimWorld.

Оригинальный source должен оставаться read-only.

## CLI

Для автоматизации:

~~~bash
rimloc-cli scan --root ./Mods/MyMod --format json
rimloc-cli validate --root ./Mods/MyMod
~~~

### Опциональный PO handoff

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

Внешний перевод импортируйте в **working copy**, не поверх Workshop source.

### Build без PO

Если translated <code>Languages</code> XML уже готов:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

## Перед публикацией

- проверьте placeholders/tags;
- review source-changed entries;
- протестируйте в игре;
- не меняйте original source;
- сохраните diagnostics, если что-то пошло не так.

См. [Начало работы](../getting-started.md) и [Гайд переводчика](../guide/translators.md).
