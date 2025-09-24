---
title: Build Mod
---

# Build Mod Command

`build-mod` turns a translated `.po` file into a standalone RimWorld translation mod. It is useful when you want to ship translations without the original source files.

## Usage

```bash
rimloc-cli build-mod --po <FILE> --out-mod <DIR> --lang <CODE> [options]
```

## Options

| Option | Description | Required |
|--------|-------------|----------|
| `--po <FILE>` | Source PO file to package. | Yes |
| `--out-mod <DIR>` | Destination folder for the generated mod (created if missing). | Yes |
| `--lang <CODE>` | Target language code (e.g. `ru`, `ja`). Determines the language folder. | Yes |
| `--name <NAME>` | Display name for the translation mod (defaults to `RimLoc Translation`). | No |
| `--package-id <ID>` | RimWorld `PackageId` for the generated mod (defaults to `yourname.rimloc.translation`). | No |
| `--rw-version <VERSION>` | Target RimWorld version placed in `About.xml` (defaults to `1.5`). | No |
| `--lang-dir <DIR>` | Explicit language folder name inside the mod (overrides the code-based default). | No |
| `--dry-run` | Print the planned layout without writing files. | No |
| `--dedupe` | Remove duplicate keys within a single XML file (last wins). | No |

## Examples

Preview the resulting mod without touching the filesystem:

```bash
rimloc-cli build-mod \
  --po ./logs/TestMod.po \
  --out-mod ./dist/TestMod-ru \
  --lang ru \
  --dry-run
```

Generate a release-ready Russian translation mod with custom metadata (and deduplicate keys):

```bash
rimloc-cli build-mod \
  --po ./logs/TestMod.po \
  --out-mod ./dist/TestMod-ru \
  --lang ru \
  --name "TestMod — Russian" \
  --package-id author.testmod.ru \
  --rw-version 1.5 \
  --dedupe
```

## Output

- `About/About.xml` is created (or overwritten) with the provided name, package id, and target RimWorld version.
- `Languages/<lang_dir>/Keyed/_Imported.xml` contains the strings extracted from the PO file.
- The command respects the `--lang-dir` override; otherwise it converts the ISO code using RimWorld’s standard naming (`ru` → `Russian`).

## Tips

- Run `--dry-run` inside CI to review the planned files before commiting them to source control.
- Follow up with `rimloc-cli validate --root <out-mod>` if you plan to edit the generated XML further.
- Pair the command with `rimloc-cli export-po` to automate “export → package → publish” workflows.
