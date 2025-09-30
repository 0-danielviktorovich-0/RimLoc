---
title: Diff XML
---

# Команда Diff XML

Сравнивает наличие ключей между исходником и переводом; опционально выявляет изменившиеся исходные строки на основе базового PO. Для английского исходника RimLoc учитывает `Languages/English` и `Defs` (имплицитные поля).

## Синопсис

```bash
rimloc-cli diff-xml --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--defs-dir <PATH>] [--defs-field <NAME>] [--defs-dict <PATH>] [--lang <CODE>|--lang-dir <DIR>] [--baseline-po <PO>] [--format text|json] \
  [--out-dir <DIR>] [--game-version <VER>] [--strict] [--apply-flags] [--backup]
```

## Опции
- `--root <MOD>`: корень мода (обязательно)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: папка исходника (по умолчанию English)
- `--defs-dir <PATH>`: ограничить корень `Defs` указанным путём (относительно root или абсолютным)
- `--defs-field <NAME>`: дополнительные поля `Defs` для извлечения (повторять или через запятую)
- `--defs-dict <PATH>`: дополнительные словари `Defs` (JSON: DefType → [field paths])
- `--lang <CODE>` / `--lang-dir <DIR>`: папка перевода (по умолчанию Russian)
- `--baseline-po <PO>`: предыдущий экспорт для выявления изменившихся исходных строк
- `--format`: text (по умолчанию) или json
- `--out-dir <DIR>`: записи txt-отчётов (ChangedData.txt, TranslationData.txt, ModData.txt)
- `--game-version <VER>`: подпапка версии
- `--strict`: ненулевой код выхода при отличиях
- `--apply-flags`: править переводные XML на месте: добавить `<!-- FUZZY -->` у ключей с изменившимся исходником (по baseline PO) и `<!-- UNUSED -->` у ключей, которые есть только в переводе
- `--backup`: сохранять `.bak` перед изменением XML (по умолчанию: включено)
