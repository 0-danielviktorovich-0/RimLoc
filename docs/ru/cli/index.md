---
title: CLI команды
---

# Обзор CLI-команд

RimLoc CLI объединяет инструменты для сбора, проверки и обмена переводами RimWorld. Команды выводят единообразные сообщения и коды возврата, поэтому их легко скриптовать и запускать в CI.

## Перед началом

- Установите CLI: `cargo install rimloc-cli`.
- Работайте с чистой копией мода — команды читают и пишут внутри `Languages/`.
- Потренируйтесь на фикстуре `test/TestMod`, прежде чем запускать команды на реальных данных.

## Типовой рабочий цикл

1. **Scan** — извлекает строки из мода.
2. **Validate** — ловит дубликаты, пустоты и несоответствия плейсхолдеров.
3. **Export PO** — готовит пакет для переводчиков или CAT-инструментов.
4. **Validate PO** — сверяет плейсхолдеры в переведённых PO-файлах.
5. **Import PO** — возвращает переводы в XML и позволяет снова прогнать `validate` перед релизом.
6. *(Опционально)* **Build Mod** — собирает автономный мод-перевод из итогового `.po` файла.

## Сводная таблица

| Команда | Назначение | Частые опции |
|---------|------------|--------------|
| [`scan`](scan.md) | Собирает единицы перевода из XML. | `--lang`, `--format`, `--out-csv`, `--out-json`, `--game-version`, `--include-all-versions` |
| [`validate`](validate.md) | Проверяет XML на дубликаты, пустоты и плейсхолдеры. | `--format`, `--source-lang`, `--source-lang-dir`, `--game-version`, `--include-all-versions` |
| [`validate-po`](validate_po.md) | Сравнивает плейсхолдеры в PO-файлах. | `--po`, `--strict`, `--format` |
| [`export-po`](export_import.md#export-po) | Формирует единый PO-файл для переводчиков. | `--root`, `--out-po`, `--lang`, `--game-version`, `--include-all-versions` |
| [`import-po`](export_import.md#import-po) | Применяет изменения из PO к XML. | `--mod-root`, `--out-xml`, `--dry-run`, `--single-file`, `--game-version` |
| [`build-mod`](build_mod.md) | Собирает самостоятельный мод-перевод. | `--out-mod`, `--package-id`, `--dry-run` |

## Глобальные опции

- `--ui-lang <LANG>` — язык сообщений (например, `en`, `ru`).
- `--no-color` — отключить ANSI‑цвета в терминале.
- `--quiet` — скрыть стартовый баннер и несущ. сообщения в stdout (алиас: `--no-banner`). Рекомендуется для JSON‑конвейеров.

## Полезные паттерны

```bash
# Запустить проверку в CI и упасть только при ошибках
rimloc-cli validate --root ./path/to/mod --format text

# Получить машинно-читаемую диагностику
rimloc-cli validate --root ./path/to/mod --format json | jq '.[] | select(.level=="error")'

# Экспортировать и тут же проверить плейсхолдеры в PO
rimloc-cli export-po --root ./path/to/mod --out-po ./out/mymod.po --lang ru
rimloc-cli validate-po --po ./out/mymod.po --strict

# Посмотреть, каким будет готовый мод-перевод
rimloc-cli build-mod --po ./out/mymod.po --out-mod ./ReleaseMod --lang ru --dry-run
```

## Решение проблем

- **В примерах появляются префиксы `/RimLoc/`** — очистите `SITE_URL` локально; задавайте его только в CI перед `mkdocs build`.
- **Сообщения `placeholder-check`** — сравните плейсхолдеры в исходных и переведённых строках; флаг `--format json` подсветит проблемный ключ.
- **Экспорт/импорт ничего не делает** — убедитесь, что каталог `Languages/<lang>/` существует и код языка совпадает с переданным флагом.

Нужны детали по конкретной команде? На отдельных страницах приведены таблицы опций, примеры и советы по устранению ошибок.
