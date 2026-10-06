---
title: Translate a Mod
---

# Translate a RimWorld mod from scratch

For most users, the intended path is the desktop project workflow. PO is optional.

## Desktop workflow

1. Open RimLoc and choose **New translation**.
2. Select the RimWorld mod/source.
3. Choose target locale(s).
4. Translate directly in the workspace.
5. Review source/context and glossary/TM suggestions where available.
6. Run validation.
7. Build/export to an isolated output directory.
8. Test the translation in RimWorld.

The original source should remain read-only.

## CLI workflow

If you prefer automation:

~~~bash
rimloc-cli scan --root ./Mods/MyMod --format json
rimloc-cli validate --root ./Mods/MyMod
~~~

### Optional PO handoff

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

Translate externally, then validate/import on a **working copy**.

### No-PO build

If you already have translated <code>Languages</code> XML:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

## Before publishing

- validate placeholders/tags;
- inspect changed-source entries;
- run a real game check;
- keep the source mod untouched;
- save diagnostics if something behaves unexpectedly.

See [Getting started](../getting-started.md) and [Translator guide](../guide/translators.md).
