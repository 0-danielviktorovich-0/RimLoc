---
title: PO Files 101
---

# PO Files 101

This page explains what `.po` files are, why RimLoc uses them, and how to edit them comfortably.

## What is a PO file?

PO (Portable Object) is a simple text format from the GNU gettext ecosystem. Each entry contains:

```
#: <reference to source file>
msgctxt "<optional context>"
msgid "<source text>"
msgstr "<translated text>"
```

RimLoc adds helpful `#: path:line` comments and a unique `msgctxt` that combines the key and a relative path to keep entries stable across changes.

## Why PO with RimLoc?

- Friendly for translators — many desktop tools support PO.
- Keeps context, references, and keys in one place.
- Easy to diff and review in PRs.

## Typical RimLoc entry

```
#: Mods/MyMod/Languages/English/Keyed/Gameplay.xml:42
msgctxt "Greeting|Keyed/Gameplay.xml:42"
msgid "Hello, {PAWN_label}!"
msgstr ""
```

Translate by filling `msgstr` but keep placeholders (like `{PAWN_label}`) unchanged.

## Workflow with RimLoc

- Export:

```bash
rimloc-cli --quiet export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --lang ru
```

- Edit in your favorite PO editor (see below).

- Validate placeholders (strict for CI):

```bash
rimloc-cli --quiet validate-po --po ./out/MyMod.po --strict --format json | jq .
```

- Import back to XML (single file or full structure):

```bash
# Single XML for review
rimloc-cli --quiet import-po --po ./out/MyMod.po --out-xml ./out/MyMod.ru.xml

# Update a mod’s structure (with backups)
rimloc-cli --quiet import-po --po ./out/MyMod.po --mod-root ./Mods/MyMod --backup
```

## Recommended editors

- Poedit — popular, cross‑platform editor tailored for PO files.
- Gtranslator (GNOME), Lokalize (KDE) — native Linux apps.
- VS Code — “gettext/PO” extensions exist for syntax highlight and basic editing.
- CLI utilities: `msgfmt`, `msgcat`, `msgconv` from gettext (advanced users).

Tip: PO files must be UTF‑8. If a tool saved another encoding, convert it:

```bash
msgconv --to-code=utf-8 ./in.po > ./out.po
```

## Placeholders

Don’t change placeholders (e.g., `{count}`, `%s`). See Guides → Placeholders for details.

