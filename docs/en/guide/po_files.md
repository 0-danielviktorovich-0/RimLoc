---
title: PO Files (optional interchange)
---

# PO files in RimLoc

PO (Portable Object) is one **interchange format** supported by RimLoc. It is useful with Poedit and many CAT tools, but it is **not the canonical RimLoc project format** and it is not required for normal desktop translation.

## When PO is useful

Use PO when you want to:

- hand a translation to an external translator/CAT tool;
- review a single text-oriented file in Git;
- import work produced by an existing gettext-oriented workflow;
- keep compatibility with established translation tooling.

If you translate directly in the RimLoc desktop project, you can skip PO entirely.

## Basic structure

~~~text
#: path/to/source.xml:42
msgctxt "stable-context"
msgid "Hello, {PAWN_label}!"
msgstr "Привет, {PAWN_label}!"
~~~

Keep placeholders/tags intact.

## Export

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

## Validate

~~~bash
rimloc-cli validate-po --po ./work/MyMod.ru.po --strict
~~~

## Import

Always prefer a working copy / isolated output rather than the original Workshop source:

~~~bash
rimloc-cli import-po \
  --po ./work/MyMod.ru.po \
  --mod-root ./work/MyMod-copy \
  --lang ru \
  --dry-run
~~~

## Build without PO

If translated XML already exists:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

So the mental model is:

~~~text
canonical RimLoc project
        ↕
   import/export adapters
   PO · CSV · XLIFF · XML · ...
~~~

not “RimLoc project = PO”.

## Editors

If you choose the PO path:

- Poedit;
- Lokalize;
- Gtranslator;
- gettext CLI tools;
- editors with PO extensions.

See also [PO export/import](../cli/export_import.md) and [placeholders](placeholders.md).
