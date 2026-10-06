---
title: Getting Started
---

# Getting started with RimLoc

RimLoc has a desktop project workflow and a CLI. For most translators, the desktop workflow is the intended path. The CLI is useful for automation, debugging, CI, and external CAT-tool handoffs.

!!! warning "Pre-beta"
    The React desktop UI is still being hardened. If you are testing a development build, make sure the artifact explicitly says React/REACT_PROD rather than the legacy fallback.

## Path A — Desktop workflow

### 1. Create or open a project

From Home:

- choose **New translation** for a new source;
- choose **Open/update existing** when you already have a translation to preserve and refresh.

For RimWorld, the adapter can represent different source kinds (for example a mod, Core/DLC source, language pack, or an existing translation) when that capability is available in the current build.

### 2. Choose the target locale

A RimLoc project can carry multiple target locales. The source inventory stays shared; translations are isolated per target locale.

### 3. Translate directly in RimLoc

Edit the target text in the workspace. PO is not required.

The project workflow keeps translation state/revisions in the RimLoc project model.

### 4. Validate and review

Use Checks/validation before building. Fix placeholder, structural, source-drift or other findings surfaced by the current adapter.

### 5. Build/export

Choose an isolated output directory. The source mod/game tree is treated as read-only.

The exact output/build controls depend on the adapter capabilities.

## Path B — CLI workflow

If you prefer terminal automation, start with a bundled fixture:

~~~bash
cargo build -p rimloc-cli
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
~~~

### PO is optional

PO is useful when you want Poedit or another CAT tool:

~~~bash
cargo run -p rimloc-cli -- export-po \
  --root ./test/TestMod \
  --out-po ./logs/TestMod.po \
  --lang ru
~~~

Importing it back is a separate interoperability workflow:

~~~bash
cargo run -p rimloc-cli -- import-po \
  --po ./logs/TestMod.po \
  --mod-root ./test/TestMod \
  --lang ru \
  --dry-run
~~~

### Build without PO

If you already have a translated RimWorld <code>Languages/&lt;locale&gt;</code> tree:

~~~bash
cargo run -p rimloc-cli -- build-mod \
  --from-root ./Mods/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

This is why PO should be thought of as an interchange adapter, not the canonical RimLoc storage format.

## Next steps

- [Desktop GUI](guide/gui.md)
- [Translator workflow](guide/translators.md)
- [CLI overview](cli/index.md)
- [Update an existing translation](tutorials/update_translations.md)
- [Troubleshooting](troubleshooting.md)
