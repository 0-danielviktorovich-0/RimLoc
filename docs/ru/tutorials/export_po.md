---
title: Экспорт в .po
---

# 📤 Как экспортировать строки в .po

Цель: получить один удобный `.po` для Poedit/CAT‑систем. Подходит как для первого перевода, так и для обновлений.

## Шаг 1. Подготовить мод и проверить

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan.json
rimloc-cli validate --root ./Mods/MyMod --format text
```

💡 Совет: если увидели ошибки — исправьте их до экспорта, так вы не унесёте мусор в `.po`.

## Шаг 2. Экспорт `.po`

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

⚠️ Важно: не меняйте [плейсхолдеры](../glossary.md) в переводе.

## Шаг 3. Проверить `.po` (рекомендуется)

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict --format text
```

## Шаг 4. Импорт обратно (по необходимости)

```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
```

Если всё верно — уберите `--dry-run`.

## См. также

- CLI: export/import — ../cli/export_import.md
- Проверка плейсхолдеров — ../cli/validate_po.md
- Словарь терминов — ../glossary.md

