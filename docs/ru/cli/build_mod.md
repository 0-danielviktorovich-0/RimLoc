---
title: Build Mod
---

# Сборка translation-only мода

<code>build-mod</code> упаковывает переведённые RimWorld language data в отдельный мод-перевод.

**PO не обязателен.** Есть два source mode:

1. внешний PO handoff;
2. уже готовое дерево <code>Languages/&lt;язык&gt;</code>.

## Сборка из готового Languages-дерева

Используйте этот вариант, если translated XML уже существует — в том числе как результат RimLoc project workflow:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

После проверки плана повторите команду без <code>--dry-run</code>.

Полезные опции:

- <code>--from-root &lt;DIR&gt;</code> — дерево с translated Languages;
- <code>--from-game-version &lt;CSV&gt;</code> — version subfolders;
- <code>--out-mod &lt;DIR&gt;</code> — отдельный destination;
- <code>--lang</code> / <code>--lang-dir</code>;
- <code>--name</code>, <code>--package-id</code>, <code>--rw-version</code>;
- <code>--dedupe</code>;
- <code>--dry-run</code>.

## Сборка из PO

Если команда/переводчик работает через Poedit/CAT:

~~~bash
rimloc-cli build-mod \
  --po ./work/MyMod.ru.po \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

Это удобный interoperability workflow, но не обязательный project path RimLoc.

## Безопасность output

Реальная сборка не должна молча смешиваться с непустым старым output без явно выбранной merge-семантики текущего CLI.

Всегда:

- сначала <code>--dry-run</code>;
- пишите в output, а не в game/Workshop source;
- валидируйте результат;
- тестируйте его в RimWorld.

## Почему в CLI всё ещё есть PO

CLI появился раньше полной canonical project model и сохраняет стабильные форматные команды ради внешних CAT-процессов. Это совместимость и удобство, а не требование хранить RimLoc-проект в PO.

Точные опции вашей сборки: <code>rimloc-cli build-mod --help</code>.
