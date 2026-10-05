---
title: Update Existing Translation
---

# Update an existing translation after a mod update

The important rule is: **preserve human work while re-evaluating the source**.

## Desktop workflow

Use **Open/update existing** rather than starting over.

The update workflow should distinguish, where the current adapter supports it:

- unchanged;
- source-changed;
- new;
- obsolete/orphan;
- ambiguous/moved entries.

Review the changed/new set, keep trusted translations, validate, then build/export a fresh translation mod.

## CLI diagnostics

For lower-level inspection:

~~~bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan-after.json
rimloc-cli validate --root ./Mods/MyMod
rimloc-cli diff-xml --root ./Mods/MyMod --format text
~~~

If your team uses PO, export/import remains available as an interoperability path, not the canonical update model.

## Rebuild from translated XML

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

## Review checklist

- human edits preserved;
- source-changed strings explicitly reviewed;
- deleted/obsolete entries not silently shipped;
- target locales remain isolated;
- placeholders/tags valid;
- output tested in game.
