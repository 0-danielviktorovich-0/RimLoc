---
title: Troubleshooting
---

# Если что-то не работает

## Запустился старый интерфейс

В pre-beta одновременно существуют React и замороженный Svelte fallback.

Проверьте artifact identity:

- frontend flavor должен соответствовать нужной сборке;
- owner/production candidate не должен включать automation;
- имя файла <code>.app</code> само по себе не доказывает frontend.

Если вам выдали owner-test packet, запускайте app именно из него.

## В Recent Projects появились test/demo проекты

Automation должна использовать отдельный data/profile. Synthetic projects в обычном owner profile — баг. Приложите build identity и screenshot.

## Не открывается/не создаётся проект

В багрепорт добавьте:

- build/commit identity;
- source type (mod/Core/DLC/existing и т.д.);
- RimWorld version;
- diagnostics/support bundle после проверки на private data.

Не правьте Workshop originals как workaround.

## Validation ругается на placeholders/tags

Сравните target со source и сохраните обязательные placeholders/markup.

Для PO-handoff:

~~~bash
rimloc-cli validate-po --po ./work/MyMod.po --strict
~~~

## CLI не находится

При Cargo install проверьте Cargo bin в PATH. Standalone binary запускайте из папки как <code>./rimloc-cli</code> (macOS/Linux) или <code>.\rimloc-cli.exe</code> (Windows).

## Write-команда ничего не делает / пишет не туда

- используйте <code>--dry-run</code>;
- выбирайте отдельные/absolute output paths, где это требуется;
- не направляйте запись в original game/Workshop source;
- смотрите актуальный help: <code>rimloc-cli &lt;command&gt; --help</code>.

## PO import ничего не меняет

Это относится только к выбранному PO-workflow:

- проверьте непустые <code>msgstr</code>;
- source/context должны относиться к тому же проекту;
- используйте <code>--report --dry-run</code>.

PO опционален; desktop translation его не требует.

## Нужна помощь

Создайте issue через [GitHub form](https://github.com/0-danielviktorovich-0/RimLoc/issues/new/choose), приложите маленькую fixture и sanitized diagnostics.
