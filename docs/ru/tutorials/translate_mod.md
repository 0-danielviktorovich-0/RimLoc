---
title: Как перевести мод
---

# 🧭 Как перевести мод с нуля

Этот гайд — для ситуации «есть мод без перевода, хочу быстро сделать хороший RU». Делаем по шагам, с примерами команд.

📌 Не знакомы с терминами? Откройте [Словарь RimLoc](../glossary.md).

## Шаг 1. Подготовка

- Установите RimLoc: `cargo install rimloc-cli`
- Найдите корень мода (папка, где есть `About/`, `Defs/`, иногда `Languages/`). Допустим: `./Mods/MyMod`.

## Шаг 2. Скан и проверка

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan.json
rimloc-cli validate --root ./Mods/MyMod --format text
```

Зачем? Сразу видим дубликаты, пустые строки и потенциальные проблемы [плейсхолдеров](../glossary.md#плейсхолдер). Это экономит часы на ревью.

📌 Подробнее: ../cli/scan.md · ../cli/validate.md

## Шаг 3. Экспорт PO

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

- Получится один удобный `.po` с оригиналами (`msgid`) и местом для перевода (`msgstr`).
- Открывайте в Poedit/веб‑редакторе и переводите.

📌 Подробнее: ../cli/export_import.md

## Шаг 4. Проверка PO

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict
```

Ловим несоответствия [плейсхолдеров](../glossary.md#плейсхолдер) заранее.

📌 Подробнее: ../cli/validate_po.md

## Шаг 5. Импорт перевода в мод

Быстрый один файл (для ревью):
```bash
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml
```

Или правильная разкладка по структуре:
```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report
```

## Шаг 6. Собрать отдельный мод‑перевод (опционально)

```bash
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru
```

Проверьте готовый `MyMod_RU` в игре (включите мод и выберите язык Russian).

## Типичные ошибки и как их избежать

- Испортили плейсхолдеры (`%d`, `{0}`, `{PAWN_name}`)?
  - Перепроверьте `validate-po` и верните точное соответствие исходнику.
- Нечего импортировать?
  - Убедитесь, что в `.po` есть непустые `msgstr`. Для заглушек используйте флаг `--keep-empty` у импорта.
- Сломали XML‑теги в тексте?
  - RimLoc подскажет валидацией. Старайтесь не убирать техтеги (`<br/>`, `</i>` и т.п.) из оригинала.

## Куда дальше

- Если переводили старую версию — посмотрите обновление: [Как обновлять переводы](update_translations.md)
- Хотите подсказок и приёмов? Загляните в [Советы и лайфхаки](../tips.md)

