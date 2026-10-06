---
title: Troubleshooting
---

# Troubleshooting

## I opened the app and see the old interface

During pre-beta, React and the frozen Svelte fallback can both exist in build outputs.

Check the artifact identity:

- frontend flavor should match the build you intended to test;
- owner/production candidate should not have automation enabled;
- do not assume the file name alone identifies the frontend.

If you were given an owner-test packet, use the app from that exact packet.

## Test/demo projects appear in Recent Projects

Automation should use an isolated data/profile directory. Synthetic projects in a normal owner profile are a bug — report the build identity and screenshot.

## The app cannot open/create a project

Include:

- build/commit identity;
- source type (mod/Core/DLC/existing/etc.);
- RimWorld version;
- diagnostics/support bundle after reviewing it for private data.

Do not move or edit Workshop originals as a workaround.

## Validation reports placeholder/tag problems

Review the target string against the source. Keep placeholders and markup required by the source/adapter.

For PO-specific handoffs:

~~~bash
rimloc-cli validate-po --po ./work/MyMod.po --strict
~~~

## CLI command not found

If installed with Cargo, make sure Cargo's bin directory is in PATH. If using a standalone CLI binary, run it from its folder with <code>./rimloc-cli</code> (macOS/Linux) or <code>.\rimloc-cli.exe</code> (Windows).

## A write command does nothing / writes somewhere unexpected

- prefer <code>--dry-run</code>;
- use absolute/isolated output paths where the command requires them;
- never point write operations at the original game/Workshop source;
- use current command help: <code>rimloc-cli &lt;command&gt; --help</code>.

## PO import does nothing

This only applies if you chose the PO workflow:

- check for non-empty <code>msgstr</code>;
- verify PO references/context belong to the same source;
- run import with <code>--report --dry-run</code>.

PO is optional; desktop translation does not require it.

## Need more help?

Use the [GitHub bug form](https://github.com/0-danielviktorovich-0/RimLoc/issues/new/choose) and attach a small reproducible fixture plus sanitized diagnostics.
