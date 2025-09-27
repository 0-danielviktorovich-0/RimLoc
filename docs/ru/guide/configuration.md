---
title: Конфигурация (rimloc.toml)
---

# Конфигурация (rimloc.toml)

RimLoc читает значения по умолчанию из файла настроек, чтобы не повторять флаги команд каждый раз.

Где ищется файл
- 1) `./rimloc.toml` — рядом с местом запуска команд (высший приоритет)
- 2) `$HOME/.config/rimloc/rimloc.toml`
Флаги команд всегда перекрывают значения из конфигурации.

Минимальный пример
```
source_lang = "English"
target_lang = "Russian"
game_version = "1.5"
list_limit = 100
```

Полный пример (скопируйте и отредактируйте под себя)
```
source_lang = "English"
target_lang = "Russian"
game_version = "1.5"
list_limit = 100

[export]
source_lang_dir = "English"
include_all_versions = false
# tm_root = "./Mods/MyMod/Languages/Russian"

[import]
keep_empty = false
backup = true
single_file = false
incremental = true
only_diff = true
report = true
lang_dir = "Russian"

[build]
name = "RimLoc Translation"
package_id = "yourname.rimloc.translation"
rw_version = "1.5"
lang_dir = "Russian"
dedupe = true
# from_root_versions = ["1.4", "1.5"]

[diff]
out_dir = "./logs/diff"
strict = false

[health]
lang_dir = "Russian"
strict = false
# only = ["encoding-detected"]
# except = ["unexpected-doctype"]

[annotate]
comment_prefix = "EN:"
strip = false
backup = true

[init]
overwrite = false

[schema]
out_dir = "docs/assets/schemas"
```

Как это влияет на команды
- `source_lang` / `target_lang` / `game_version` — общие дефолты (если флаги не заданы).
- `export` — для `export-po`.
- `import` — для `import-po` (с `--dry-run` всё равно печатает план).
- `build` — для `build-mod` (name/packageId/version/lang_dir).
- `diff` и `health` — для `diff-xml` и `xml-health`.
- `annotate` — для `annotate` (префикс, strip, backup).
- `init` — для `init` (overwrite).
- `schema` — куда сохранять JSON‑схемы.

Советы
- Храните файл рядом с репозиторием мода — команда будет работать одинаково у всей команды.
- Длинные пути берите в кавычки.
- Начинайте с `--dry-run`, чтобы увидеть, как настройки влияют на план действий.

