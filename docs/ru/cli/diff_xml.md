---
title: Diff XML
---

# Команда Diff XML

Сравнивает наличие ключей между исходником и переводом; опционально выявляет изменившиеся исходные строки на основе базового PO.

## Синопсис

```bash
rimloc-cli diff-xml --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--lang <CODE>|--lang-dir <DIR>] [--baseline-po <PO>] [--format text|json] \
  [--out-dir <DIR>] [--game-version <VER>] [--strict]
```

## Опции
- `--root <MOD>`: корень мода (обязательно)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: папка исходника (по умолчанию English)
- `--lang <CODE>` / `--lang-dir <DIR>`: папка перевода (по умолчанию Russian)
- `--baseline-po <PO>`: предыдущий экспорт для выявления изменившихся исходных строк
- `--format`: text (по умолчанию) или json
- `--out-dir <DIR>`: записи txt-отчётов (ChangedData.txt, TranslationData.txt, ModData.txt)
- `--game-version <VER>`: подпапка версии
- `--strict`: ненулевой код выхода при отличиях

