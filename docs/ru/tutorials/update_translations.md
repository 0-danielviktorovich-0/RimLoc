---
title: Обновление существующего перевода
---

# Как обновить перевод после обновления мода

Главное правило: **сохранить ручную работу, но заново оценить изменившийся source**.

## Desktop workflow

Используйте **Открыть/обновить существующий**, а не начинайте с нуля.

Update workflow должен различать, где это поддерживает текущий adapter:

- unchanged;
- source-changed;
- new;
- obsolete/orphan;
- ambiguous/moved.

Review changed/new, сохраняйте trusted translations, запускайте validation и собирайте новый translation mod.

## CLI-диагностика

Для низкоуровневой проверки:

~~~bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan-after.json
rimloc-cli validate --root ./Mods/MyMod
rimloc-cli diff-xml --root ./Mods/MyMod --format text
~~~

PO export/import остаётся доступным, если команда работает через CAT, но это не каноническая update model.

## Rebuild из translated XML

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

## Checklist

- ручные правки сохранены;
- source-changed review-нуты;
- obsolete entries не шипятся молча;
- target locale изолированы;
- placeholders/tags валидны;
- output проверен в игре.
