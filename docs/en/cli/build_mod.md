---
title: Build Mod
---

# Build a translation-only RimWorld mod

<code>build-mod</code> packages translated RimWorld language data into a standalone translation mod.

**PO is not required.** The command supports two source modes:

1. an external PO handoff;
2. an existing translated <code>Languages/&lt;locale&gt;</code> tree.

## Build from an existing Languages tree

Use this when translation XML already exists — including output produced by a RimLoc project workflow:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

After reviewing the plan, rerun without <code>--dry-run</code>.

Relevant options include:

- <code>--from-root &lt;DIR&gt;</code> — source tree containing translated Languages data;
- <code>--from-game-version &lt;CSV&gt;</code> — select version subfolders;
- <code>--out-mod &lt;DIR&gt;</code> — isolated destination;
- <code>--lang &lt;CODE&gt;</code> / <code>--lang-dir &lt;DIR&gt;</code>;
- <code>--name</code>, <code>--package-id</code>, <code>--rw-version</code>;
- <code>--dedupe</code>;
- <code>--dry-run</code>.

## Build from PO

Use this when PO is the chosen interchange format:

~~~bash
rimloc-cli build-mod \
  --po ./work/MyMod.ru.po \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

This is useful for Poedit/CAT workflows, but it is not the canonical RimLoc project path.

## Output safety

A real build refuses to silently merge into a non-empty output directory unless the explicit merge behavior is requested by the current CLI.

Always:

- use <code>--dry-run</code> first;
- write to an output directory, not the source mod/game tree;
- validate the generated result before publishing;
- test it in RimWorld.

## Why the command still mentions PO

The CLI predates the full canonical project workflow and keeps stable, useful format-specific commands for interoperability. That is intentional compatibility, not an architectural requirement that RimLoc projects be PO files.

Run <code>rimloc-cli build-mod --help</code> for the exact options supported by your build.
