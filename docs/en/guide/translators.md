---
title: For Translators
---

# Translate a RimWorld project with RimLoc

This guide focuses on the workflow a translator should actually use. You do **not** need to understand RimWorld XML, and you do **not** need PO unless you want an external CAT workflow.

## Recommended path: desktop project

### 1. Start from the source

Create a new project or open/update an existing translation.

Pick the relevant RimWorld source type supported by the current build (for example a mod or existing translation). RimLoc builds a canonical inventory from the source.

### 2. Choose target language(s)

One project can keep multiple target locales. Switching target locale should not overwrite work in another locale.

### 3. Translate in the editor

Work directly in the RimLoc workspace:

- source and target text;
- context/source information;
- status/review state;
- glossary;
- TM suggestions when enabled;
- validation findings.

Your normal translation does not need to leave RimLoc.

### 4. Validate before building

Run Checks/validation. Pay particular attention to placeholders, tags and source-changed entries.

### 5. Build/export safely

Choose an isolated output directory. RimLoc should not rewrite the original Workshop/game source tree.

Test the generated translation in RimWorld before publishing it.

## Optional path: PO / external CAT tool

Use this only when you want Poedit or another CAT tool.

Export:

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

Validate the returned PO:

~~~bash
rimloc-cli validate-po --po ./work/MyMod.ru.po --strict
~~~

Preview import:

~~~bash
rimloc-cli import-po \
  --po ./work/MyMod.ru.po \
  --mod-root ./work/MyMod-copy \
  --lang ru \
  --dry-run
~~~

PO is a handoff format here — not the canonical project database.

## Build from an existing Languages tree

If a translation already exists as RimWorld XML, PO is unnecessary:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyModTranslated \
  --out-mod ./dist/MyMod-Russian \
  --lang ru \
  --dry-run
~~~

Remove <code>--dry-run</code> only after the plan looks correct.

## Updating an existing translation

When the source mod changes, the important behavior is preserving human work while distinguishing:

- unchanged;
- source-changed;
- new;
- obsolete/orphan;
- ambiguous cases.

Use the dedicated existing/update project workflow rather than starting from zero.

## Practical advice

- Keep source mods read-only; work through RimLoc project/output locations.
- Do not “fix” placeholders by translating them.
- Validate again after bulk/import/AI-assisted changes.
- Test in the game: context, line breaks and wording cannot be judged from XML alone.
- If an AI/provider is used, treat its output as a draft until reviewed.

## See also

- [Getting started](../getting-started.md)
- [Desktop GUI](gui.md)
- [Update translations](../tutorials/update_translations.md)
- [Placeholders](placeholders.md)
- [Troubleshooting](../troubleshooting.md)
