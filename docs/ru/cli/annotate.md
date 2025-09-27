---
title: Annotate
---

# Команда Annotate

Добавляет или удаляет комментарии с исходным текстом в переводных XML (Keyed). Удобно для ручной проверки в редакторах.

## Синопсис

```bash
rimloc-cli annotate --root <MOD> [--source-lang <CODE>|--source-lang-dir <DIR>] \
  [--lang <CODE>|--lang-dir <DIR>] [--dry-run] [--backup] [--strip] [--game-version <VER>]
```

## Опции
- `--root <MOD>`: корень мода (обязательно)
- `--source-lang <CODE>` / `--source-lang-dir <DIR>`: откуда брать оригинал (по умолчанию English)
- `--lang <CODE>` / `--lang-dir <DIR>`: целевая папка перевода (по умолчанию Russian)
- `--dry-run`: показывать план без записи
- `--backup`: создавать .bak перед перезаписью
- `--strip`: удалять существующие комментарии вместо добавления
- `--game-version <VER>`: ограничить подпапкой версии

