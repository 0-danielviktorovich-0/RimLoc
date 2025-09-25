# rimloc-cli

RimLoc CLI — scan, validate, export/import PO for RimWorld localization.

## Install

```bash
cargo install rimloc-cli --version 0.1.0-alpha.1
```

## Usage

Scan a mod folder and print JSON:

```bash
rimloc-cli scan --root ./Mods/MyMod --format json | jq '.[0]'
```

Validate XML and fail CI on issues:

```bash
rimloc-cli validate --root ./Mods/MyMod --format text
```

Export a single PO hand-off file for translators:

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --lang ru
```

Build a translation-only mod from a curated PO:

```bash
rimloc-cli build-mod --po ./out/MyMod.po --out-mod ./ReleaseMod --lang ru
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
