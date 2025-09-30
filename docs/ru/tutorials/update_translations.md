---
title: Как обновлять переводы
---

# ♻️ Как обновлять переводы (когда мод обновился)

Сценарий: у нас есть старый перевод, автор мода выпустил обновление. Задача — быстро найти изменения и безопасно обновить строки.

📌 Нужны термины? Откройте [Словарь RimLoc](../glossary.md).

## Шаг 1. Обновить исходники и проверить

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan_after.json
rimloc-cli validate --root ./Mods/MyMod --format text
```

Зачем? Убедиться, что в моде нет проблем до начала обновления перевода.

## Шаг 2. Посмотреть, что изменилось

```bash
rimloc-cli diff-xml --root ./Mods/MyMod --format text
```

Это покажет, какие ключи/значения появились/исчезли/изменились. Можно сохранить в JSON для точной сверки.

📌 Подробнее: ../cli/diff_xml.md

## Шаг 3. Обновить/дополнить `.po`

Экспортируем новый `.po` — он будет включать свежие строки для перевода:
```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

Откройте `.po`, переведите добавившиеся строки.

## Шаг 4. Проверка качества `.po`

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict
```

Особое внимание — [плейсхолдерам](../glossary.md#плейсхолдер), иногда они меняются в исходнике.

## Шаг 5. Импорт и отчёт

```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
```

Посмотрите отчёт: какие файлы/ключи изменятся. Если всё ок, уберите `--dry-run`.

## Шаг 6. Финальная сборка RU‑мода (по желанию)

```bash
rimloc-cli build-mod --from-root ./Mods/MyMod --out-mod ./MyMod_RU --lang ru --dry-run
rimloc-cli build-mod --from-root ./Mods/MyMod --out-mod ./MyMod_RU --lang ru
```

## Полезные приёмы

- Храните предыдущие `scan.json`/`diff.json` — так проще объяснять изменения командой.
- Используйте `--report` у импорта — это удобный чеклист для ревью.
- Если автор поменял структуру `Defs/` — не беда: RimLoc сопоставляет по ключам, а не по строковым позициям.

## См. также

- Больше про импорт/экспорт: ../cli/export_import.md
- Проверка плейсхолдеров: ../cli/validate_po.md
- Если что-то ломается: ../troubleshooting.md

