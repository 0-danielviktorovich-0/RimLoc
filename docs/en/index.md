---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-GitHub-%23ea4aaa?logo=github-sponsors)](https://github.com/sponsors/0-danielviktorovich-0)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/danielviktorovich)

RimLoc is a **visual, desktop-first RimWorld mod translation studio**. Translate, review, validate and export in the GUI without a terminal. Shared Rust services also expose an optional CLI and support future adapters.

[:material-monitor: Desktop GUI — start here](guide/gui.md){ .md-button .md-button--primary }
[:material-play-circle: Getting started](getting-started.md){ .md-button }
[:material-console: Optional CLI / automation](cli/index.md){ .md-button }

!!! warning "Pre-beta"
    The current desktop UI is still being converged on React R1. Stable public installers are not available yet; use the current source/build instructions for development testing.

## Two ways to work

### Desktop project workflow

For normal translation work, PO is **not required**.

1. Create or open a RimLoc project.
2. Choose the RimWorld source and target locale(s).
3. Edit translations directly in the workspace.
4. Validate/review.
5. Build or export the translation output.

The project model, translations, glossary, TM, revisions and review state live in RimLoc's canonical project model — not in a PO file.

### CLI / interchange workflow

The CLI remains useful for automation and external CAT workflows:

- <code>scan</code>, <code>validate</code>, <code>diff-xml</code>;
- optional <code>export-po</code> / <code>import-po</code>;
- <code>build-mod</code> from PO **or from an existing Languages tree**;
- diagnostics and other maintenance commands.

Use PO when it helps you collaborate with Poedit/CAT tooling; do not treat it as mandatory.

## AI translation

Today the GUI offers **Chat Batch without an API key**: export a structured
prompt, use your favorite AI chat, import, review and safely apply translations.
Provider connections can be configured and tested, but direct automatic
AI translation jobs are **not yet shipped**. Planned options include
official eligible ChatGPT-plan authorization for OSS clients, BYO API keys,
local models and the existing no-API fallback — all in the graphical app.

## Current focus

The first public beta is deliberately RimWorld-first:

- modern RimWorld project discovery;
- Core/DLC/mod/language-pack sources;
- existing-translation update workflows;
- multi-target projects;
- glossary and Translation Memory;
- validation and safe build/export;
- security hardening;
- practical comparison with established RimWorld localization tools.

Future adapters are a design goal, not a current compatibility claim.

## Documentation paths

- [Getting started](getting-started.md)
- [Translator guide](guide/translators.md)
- [Desktop GUI](guide/gui.md)
- [CLI](cli/index.md)
- [Troubleshooting](troubleshooting.md)
- [Developer guide](dev/index.md)
- [Adapter authoring](dev/adapters.md)
- [Support RimLoc](community/support.md)

!!! tip "Help translate RimLoc"
    See the [localization guide](community/localization.md). RimLoc self-localization uses the same core project concepts as other adapters.
