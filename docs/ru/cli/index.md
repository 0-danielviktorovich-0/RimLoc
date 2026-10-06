---
title: CLI команды
---

# CLI-команды

RimLoc CLI — детерминированный/headless интерфейс того же Rust toolchain, который используется desktop-приложением.

!!! important "PO — опционально"
    PO — формат обмена. Это **не** каноническая модель проекта RimLoc. Desktop workflow редактирует project state напрямую, а CLI умеет собирать мод и из готового Languages-дерева.

## Типовые сценарии

### Scan и validate

~~~bash
rimloc-cli scan --root ./Mods/MyMod --format json
rimloc-cli validate --root ./Mods/MyMod
~~~

### Внешний CAT / PO handoff

~~~bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./work/MyMod.po --lang ru
rimloc-cli validate-po --po ./work/MyMod.po --strict
rimloc-cli import-po --po ./work/MyMod.po --mod-root ./work/MyMod-copy --lang ru --dry-run
~~~

### Build без PO

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

Это напрямую упаковывает уже существующее переведённое Languages-дерево.

## Основные команды

| Команда | Назначение |
| --- | --- |
| [scan](scan.md) | найти translation units |
| [validate](validate.md) | проверить XML/translation state |
| [validate-po](validate_po.md) | проверить PO handoff |
| [export-po / import-po](export_import.md) | опциональный PO interoperability |
| [build-mod](build_mod.md) | собрать translation-only мод из PO или Languages |
| [diff-xml](diff_xml.md) | source/translation diff и source changes |
| [annotate](annotate.md) | комментарии с source text |
| [xml-health](xml_health.md) | проверки XML |
| [morph](morph.md) | morphology providers |
| [init](init.md) | translation skeleton |
| [lang-update](lang_update.md) | update language workflow |

В binary есть и дополнительные advanced/developer команды (compare/doctor/schema/translate/wordinfo/version-diff/learning helpers). Отдельные страницы для них дополняются; текущая executable truth — <code>rimloc-cli --help</code> и <code>rimloc-cli &lt;command&gt; --help</code>.

## Общие опции

Часто используются:

- <code>--ui-lang &lt;LANG&gt;</code> — язык CLI;
- <code>--no-color</code> — plain output;
- <code>--quiet</code> — меньше служебного stdout.

## Безопасность

Часть старых CLI-команд появилась до новой canonical project model. Для write-команд:

- используйте копию или отдельный output;
- сначала запускайте <code>--dry-run</code>, если он поддерживается;
- держите game/Workshop source read-only;
- проверяйте command-specific help.

## См. также

- [Начало работы](../getting-started.md)
- [Гайд переводчика](../guide/translators.md)
- [Build Mod](build_mod.md)
- [Экспорт/импорт](export_import.md)
