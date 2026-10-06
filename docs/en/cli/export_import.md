---
title: PO Interchange
---

# PO export and import

These commands are for **interoperability with PO/CAT workflows**. They are not the canonical RimLoc project model and are not required when you translate directly in the desktop application.

## Export PO

Use <code>export-po</code> when you want a single PO handoff for Poedit, another CAT tool, or an external translator.

~~~bash
rimloc-cli export-po \
  --root ./Mods/MyMod \
  --out-po ./work/MyMod.ru.po \
  --lang ru
~~~

Useful options in current builds include:

- <code>--source-lang</code> / <code>--source-lang-dir</code>;
- repeatable <code>--tm-root</code> for legacy/root-based translation reuse;
- <code>--game-version</code>;
- <code>--include-all-versions</code>.

Run source validation before handoff:

~~~bash
rimloc-cli validate --root ./Mods/MyMod
~~~

## Validate PO

~~~bash
rimloc-cli validate-po --po ./work/MyMod.ru.po --strict
~~~

This catches handoff-level issues such as placeholder mismatches.

## Import PO

Import can target a single XML file or a copied/mod working tree.

Preview first:

~~~bash
rimloc-cli import-po \
  --po ./work/MyMod.ru.po \
  --mod-root ./work/MyMod-copy \
  --lang ru \
  --dry-run \
  --report
~~~

Then rerun without <code>--dry-run</code> only after the plan is correct.

Common controls include:

- <code>--backup</code>;
- <code>--incremental</code>;
- <code>--only-diff</code>;
- <code>--single-file</code>;
- <code>--format text|json</code>.

## Do not import into Workshop originals

The current product direction treats game/Workshop/source directories as read-only. For manual CLI experiments, use a working copy or isolated output tree.

## Not using PO?

That is normal.

- Desktop project editing does not require PO.
- <code>build-mod --from-root</code> can package an existing translated Languages tree.
- Future/other interchange formats should remain adapters around the same canonical project model.

See [Build Mod](build_mod.md) and [Getting started](../getting-started.md).
