---
title: PO: экспорт и импорт
---

# PO export/import

Эти команды нужны для **обмена с PO/CAT workflow**. PO не является канонической моделью RimLoc-проекта и не нужен, если перевод выполняется прямо в desktop-приложении.

## Export PO

Используйте <code>export-po</code>, когда нужен единый handoff для Poedit, другого CAT или внешнего переводчика.

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

В актуальных сборках полезны:

- <code>--source-lang</code> / <code>--source-lang-dir</code>;
- повторяемый <code>--tm-root</code> для старого root-based reuse;
- <code>--game-version</code>;
- <code>--include-all-versions</code>.

Перед handoff:

~~~bash
rimloc-cli validate --root ./Mods/MyMod
~~~

## Validate PO

~~~bash
rimloc-cli validate-po --po ./work/MyMod.ru.po --strict
~~~

Это проверяет проблемы именно handoff-формата, например placeholders.

## Import PO

Импортируйте в отдельную рабочую копию/выходное дерево.

Сначала dry-run:

~~~bash
rimloc-cli import-po \
  --po ./work/MyMod.ru.po \
  --mod-root ./work/MyMod-copy \
  --lang ru \
  --dry-run \
  --report
~~~

Только после проверки плана запускайте запись без <code>--dry-run</code>.

Частые опции:

- <code>--backup</code>;
- <code>--incremental</code>;
- <code>--only-diff</code>;
- <code>--single-file</code>;
- <code>--format text|json</code>.

## Не импортируйте в оригинальный Workshop source

Новая product-модель считает game/Workshop/source read-only. Для CLI-экспериментов используйте working copy или отдельное output-дерево.

## Не используете PO?

Это нормально.

- Desktop project editing не требует PO.
- <code>build-mod --from-root</code> собирает готовое translated Languages-дерево.
- Другие interchange formats должны оставаться адаптерами вокруг той же canonical project model.

См. [Build Mod](build_mod.md) и [Начало работы](../getting-started.md).
