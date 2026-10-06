---
title: FAQ
---

# Frequently asked questions

## Do I have to use PO files?

No.

RimLoc's desktop project workflow is based on the canonical project model, not on PO. You can translate inside RimLoc, validate/review there, and build/export the result.

PO is available when you want Poedit or another CAT/interchange workflow.

## Why does the CLI still have export-po/import-po?

Because those commands are useful interoperability tools and already have stable workflows. Keeping them does not make PO the project format.

## Can I build a translation mod without PO?

Yes, if translated RimWorld XML already exists:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyMod-RU \
  --lang ru \
  --dry-run
~~~

## Which desktop UI is current?

React R1 is the intended production UI. The previous Svelte frontend remains a frozen fallback during convergence.

When testing a packaged app, check the artifact/build identity rather than assuming every <code>.app</code> is React.

## Is RimLoc only for RimWorld forever?

The first production target and first beta are RimWorld-first.

The core is adapter-oriented so another game/application can be added later without rewriting the generic project/editor/TM/glossary layers. Other games are not claimed as supported yet.

## Is there a stable public desktop release?

Not yet. RimLoc is pre-beta. Historical alpha/dev releases exist, but the active React/Rust line is newer.

## scan vs validate?

- <code>scan</code> inventories translatable units.
- <code>validate</code> runs QA checks and reports errors/findings.

## Can I preview writes?

Use <code>--dry-run</code> where supported and always prefer isolated output/working copies for write operations.

## How do I see source changes between mod versions?

Use the desktop existing/update workflow, or lower-level CLI diagnostics such as:

~~~bash
rimloc-cli diff-xml --root ./Mods/MyMod --format text
~~~

## Where do I report a security issue?

Do not publish exploit details in a normal issue. Follow [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md).
