---
title: Init
---

# Команда Init

Создаёт заготовку перевода в `Languages/<язык>` с пустыми значениями на основе исходного языка.

## Синопсис

```bash
rimloc-cli init --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--lang <CODE>|--lang-dir <DIR>] [--overwrite] [--dry-run] [--game-version <VER>]
```

## Опции
- `--root <MOD>`: корень мода (обязательно)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: папка исходного языка (по умолчанию English)
- `--lang <CODE>` / `--lang-dir <DIR>`: целевая папка перевода (по умолчанию Russian)
- `--overwrite`: перезаписывать существующие файлы
- `--dry-run`: показывать план без записи
- `--game-version <VER>`: ограничить подпапкой версии

