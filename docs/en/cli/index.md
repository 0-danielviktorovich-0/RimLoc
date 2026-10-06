---
title: CLI Commands
---

# CLI commands

RimLoc CLI is the deterministic/headless surface of the same Rust toolchain used by the desktop app.

!!! important "PO is optional"
    PO is an interchange format. It is **not** the canonical RimLoc project model. The desktop workflow edits project state directly, and the CLI can also build from an existing RimWorld Languages tree.

## Typical uses

### Inspect and validate

~~~bash
rimloc-cli scan --root ./Mods/MyMod --format json
rimloc-cli validate --root ./Mods/MyMod
~~~

### External CAT / PO handoff

~~~bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./work/MyMod.po --lang ru
rimloc-cli validate-po --po ./work/MyMod.po --strict
rimloc-cli import-po --po ./work/MyMod.po --mod-root ./work/MyMod-copy --lang ru --dry-run
~~~

### Build without PO

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

This packages an existing translated Languages tree directly.

## Command families

The CLI currently contains more commands than the public nav historically documented. The most common ones are:

| Command | Purpose |
| --- | --- |
| [scan](scan.md) | discover translation units |
| [validate](validate.md) | validate RimWorld XML/translation state |
| [validate-po](validate_po.md) | validate a PO handoff |
| [export-po / import-po](export_import.md) | optional PO interoperability |
| [build-mod](build_mod.md) | build a translation-only mod from PO or Languages tree |
| [diff-xml](diff_xml.md) | compare source/translation and source changes |
| [annotate](annotate.md) | source-text comments |
| [xml-health](xml_health.md) | XML health checks |
| [morph](morph.md) | morphology providers |
| [init](init.md) | create translation skeleton |
| [lang-update](lang_update.md) | language-update workflow |

Additional developer/advanced commands exist in the binary (for example compare/doctor/schema/translate/wordinfo/version-diff/learning helpers). Their dedicated pages are being filled in during the documentation refresh; use <code>rimloc-cli --help</code> and <code>rimloc-cli &lt;command&gt; --help</code> as the current executable truth.

## Global options

Common options include:

- <code>--ui-lang &lt;LANG&gt;</code> — CLI message locale;
- <code>--no-color</code> — plain output;
- <code>--quiet</code> — reduce non-essential stdout for scripting.

## Safety

Not every historical CLI command was designed around the newer canonical project model. Before using a write command on valuable data:

- use a copy or isolated output directory;
- use <code>--dry-run</code> when supported;
- keep original Workshop/game directories read-only;
- review the command-specific help.

## See also

- [Getting started](../getting-started.md)
- [Translator workflow](../guide/translators.md)
- [Build Mod](build_mod.md)
- [Export/import](export_import.md)
