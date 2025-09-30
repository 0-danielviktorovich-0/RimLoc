---
title: FAQ
---

# ❓ Часто задаваемые вопросы

## RimLoc не находится в терминале

- Если ставили через Cargo — откройте новый терминал или проверьте, что `~/.cargo/bin` (Windows: `%USERPROFILE%\.cargo\bin`) есть в PATH.
- Если скачали бинарник — запускайте из папки с ним: `./rimloc-cli` (macOS/Linux) или `.\rimloc-cli.exe` (Windows), либо добавьте папку в PATH.

## Чем `scan` отличается от `validate`?

- `scan` просто собирает переводимые строки.
- `validate` проверяет качество (пустые/дубликаты/плейсхолдеры) и возвращает код `1`, если нашёл ошибки.

## Что такое «плейсхолдер» и почему он ломается?

Это «дырки» в строке, куда игра подставляет числа/имена и т. п. Если вы их удалите или измените, строка станет нерабочей. Подробнее в [Словаре](glossary.md#плейсхолдер) и в руководстве по проверке: cli/validate_po.md

## Как собрать отдельный мод‑перевод?

```bash
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru
```

Или из структуры `Languages/Russian` оригинального мода: cli/build_mod.md

## Можно ли сначала посмотреть, что изменится?

Да. Почти везде есть `--dry-run`. Например:
```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
```

## Как понять, что именно поменялось между версиями мода?

Используйте diff:
```bash
rimloc-cli diff-xml --root ./Mods/MyMod --format text
```
Подробнее: cli/diff_xml.md

## Где живёт конфигурация проекта?

В `rimloc.toml` — так можно один раз задать `source_lang`, `target_lang`, пути и упрощать команды. См. guide/configuration.md

