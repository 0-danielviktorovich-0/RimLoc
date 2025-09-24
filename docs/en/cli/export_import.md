---
title: Export and Import
---

# Export & Import

Use these commands when you are ready to share translations with collaborators or fold translated strings back into XML.

## Export PO

`export-po` writes a single `.po` file from the source language of a mod. Combine it with `scan` and `validate` to hand off a clean bundle to translators.

**Usage**

```bash
rimloc-cli export-po --root <MOD> --out-po <FILE> [--lang <CODE>] [--source-lang <CODE>] [--source-lang-dir <DIR>] [--game-version <VER>] [--include-all-versions]
```

**Options**

| Option | Description | Required |
|--------|-------------|----------|
| `-r, --root <MOD>` | Path to the RimWorld mod or `Languages/<locale>` directory to export from. | Yes |
| `--out-po <FILE>` | Destination `.po` file. Existing files are overwritten. | Yes |
| `--lang <CODE>` | Target translation language (used in the PO header, e.g. `ru`, `ja`). | No |
| `--source-lang <CODE>` | ISO code of the source language to export (defaults to `en`). | No |
| `--source-lang-dir <DIR>` | Explicit source language folder name (e.g. `English`). Overrides `--source-lang`. | No |
| `--game-version <VER>` | Version folder to export from (e.g., `1.4`, `v1.4`). Auto-detected if omitted. | No |
| `--include-all-versions` | Export from all version subfolders under the root. | No |

**Examples**

Export the bundled fixture in Russian:

```bash
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
```

Export Japanese strings that live under `Languages/Japanese`:

```bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./build/MyMod.ja.po \
  --lang ja \
  --source-lang ja
```

Point to a custom source folder (useful if you renamed `English`):

```bash
rimloc-cli export-po --root ./Mods/MyMod --source-lang-dir Original --out-po ./out/mymod.po
```

**Tips**

- Without `--lang`, the exported PO header keeps the default target code (`ru`). Pass your actual locale to avoid post-editing.
- `--source-lang` translates ISO codes into RimWorld folder names (for example, `ru` → `Russian`). If your directory name is custom, use `--source-lang-dir` instead.
- Combine with `validate` before exporting to keep broken keys out of the PO hand-off.

---

## Import PO

`import-po` reads a PO file and writes the translated content back into XML. You can import into a single XML file, update an existing mod structure, or preview the changes first.

**Usage**

```bash
rimloc-cli import-po --po <FILE> [--out-xml <XML> | --mod-root <MOD>] [--game-version <VER>] [options]
```

**Options**

| Option | Description | Required |
|--------|-------------|----------|
| `--po <FILE>` | Path to the PO file to import. | Yes |
| `--out-xml <XML>` | Write into a single LanguageData XML file (mutually exclusive with `--mod-root`). | No |
| `--mod-root <MOD>` | Update files under the given mod root based on PO references. | No |
| `--lang <CODE>` | Target language code, used to resolve `Languages/<lang>` (defaults to `ru`). | No |
| `--lang-dir <DIR>` | Explicit language folder name (overrides `--lang`). | No |
| `--keep-empty` | Keep empty strings instead of dropping them during import. | No |
| `--dry-run` | Print the planned writes without touching the filesystem. | No |
| `--backup` | Create `.bak` copies before overwriting existing XML files. | No |
| `--single-file` | When used with `--mod-root`, write everything into `Keyed/_Imported.xml`. | No |
| `--game-version <VER>` | If used with `--mod-root`, target a specific version subfolder (e.g., `1.4`). | No |

**Examples**

Preview the files that would change inside an existing mod:

```bash
rimloc-cli import-po \
  --po ./build/MyMod.ja.po \
  --mod-root ./Mods/MyMod \
  --lang ja \
  --dry-run
```

Write a single XML file for manual review:

```bash
rimloc-cli import-po --po ./logs/TestMod.po --out-xml ./out/TestMod.ru.xml --keep-empty
```

Update a mod in-place while keeping backups:

```bash
rimloc-cli import-po \
  --po ./build/MyMod.ja.po \
  --mod-root ./Mods/MyMod \
  --lang ja \
  --backup
```

Route everything into `_Imported.xml` (for translators without the full mod):

```bash
rimloc-cli import-po --po ./logs/TestMod.po --mod-root ./Mods/MyMod --single-file
```

**Tips**

- `--dry-run` shows the exact paths and key counts RimLoc would touch—great for CI or PR reviews.
- Use `--backup` when importing into a working copy you cannot easily recreate.
- `--lang-dir` lets you match non-standard folder names such as `German (Formal)`.
- Empty translations are skipped by default; pass `--keep-empty` if you want placeholder entries written as-is.

---

## See also

- **[Scan XML](scan.md)** — Extract Keyed entries to work with.
- **[Validate](validate.md)** — Check duplicates, empty values, and placeholders in RimWorld mod XML files.
- **[Validate PO](validate_po.md)** — Compare placeholders inside PO files before importing.
- **[Build Mod](build_mod.md)** — Package the final PO file into a standalone translation mod.

## Troubleshooting

- **Missing keys on import** – run `scan` first and ensure the PO file was generated from the same mod structure.
- **Nothing written after import** – confirm the language folder exists and the PO references point to files under `Languages/<lang>`.
- **PO encoding issues** – PO files must be UTF-8; run `msgconv --output=utf-8` if a translator tool saved another encoding.
