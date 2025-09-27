---
title: Configuration (rimloc.toml)
---

# Configuration (rimloc.toml)

RimLoc reads defaults from a config file so you don’t have to repeat command flags every time.

Where RimLoc looks
- 1) `./rimloc.toml` — next to where you run the command (highest priority)
- 2) `$HOME/.config/rimloc/rimloc.toml`
CLI flags always override config values.

Minimal example
```
source_lang = "English"
target_lang = "Russian"
game_version = "1.5"
list_limit = 100
```

Full example (copy/paste and edit as needed)
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

How it maps to commands
- `source_lang`, `target_lang`, `game_version` apply to most commands if not provided via flags.
- `export` section affects `export-po` when flags aren’t set.
- `import` section affects `import-po` behavior (dry-run still prints a plan).
- `build` section fills defaults for `build-mod` (name/packageId/version/lang_dir).
- `diff` and `health` sections provide additional defaults for `diff-xml` and `xml-health`.
- `annotate` section controls adding/stripping comments and backups.
- `init` controls overwrite policy.
- `schema` sets where to dump JSON Schemas.

Tips
- Keep the file next to your mod repo to share defaults with the team.
- Put long paths in quotes.
- Start with `--dry-run` to see how config affects the plan.

