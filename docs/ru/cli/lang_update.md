---
title: Lang Update
---

# Команда Lang Update

Обновляет официальную локализацию из репозитория GitHub в `Data/Core/Languages` в установленной игре RimWorld.

## Синопсис

```bash
rimloc-cli lang-update --game-root <RIMWORLD> [--repo <OWNER/NAME>] \
  [--branch <BRANCH>] [--zip <FILE>] [--source-lang-dir <DIR>] \
  [--target-lang-dir <DIR>] [--dry-run] [--backup]
```

## Опции
- `--game-root <RIMWORLD>`: Корень установленной игры (должна быть папка `Data/`).
- `--repo <OWNER/NAME>`: Репозиторий GitHub (по умолчанию: `Ludeon/RimWorld-ru`).
- `--branch <BRANCH>`: Имя ветки для загрузки (если не указано — ветка по умолчанию).
- `--zip <FILE>`: Локальный zip вместо загрузки из сети (для офлайн‑сценариев).
- `--source-lang-dir <DIR>`: Папка исходного языка в репо под `Core/Languages/` (по умолчанию: `Russian`).
- `--target-lang-dir <DIR>`: Имя целевой папки под `Data/Core/Languages/` (по умолчанию: `Russian (GitHub)`).
- `--dry-run`: Показать что будет записано, без изменений на диске.
- `--backup`: Если папка существует — переименовать в `.bak` перед записью.

## Примеры

Показать план обновления для репо по умолчанию в `Russian (GitHub)`:

```bash
rimloc-cli --quiet lang-update --game-root "/games/RimWorld" --dry-run
```

Использовать локальный zip (без сети):

```bash
rimloc-cli lang-update --game-root "/games/RimWorld" --zip ./RimWorld-ru.zip --backup
```

