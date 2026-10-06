---
title: Run a Downloaded CLI Build
---

# Run a downloaded CLI build

This page is only for standalone **CLI** binaries. The desktop application is a separate Tauri app.

!!! warning "Pre-beta artifacts"
    Historical GitHub alpha/dev artifacts may be older than the current React/Rust development line. Verify the version/commit before assuming a downloaded CLI contains new functionality.

## Windows

Open PowerShell in the folder containing <code>rimloc-cli.exe</code>:

~~~powershell
.\rimloc-cli.exe --version
.\rimloc-cli.exe --help
~~~

Basic safe checks:

~~~powershell
.\rimloc-cli.exe scan --root .\MyMod --format text
.\rimloc-cli.exe validate --root .\MyMod
~~~

## macOS / Linux

~~~bash
chmod +x ./rimloc-cli
./rimloc-cli --version
./rimloc-cli --help
./rimloc-cli scan --root ./MyMod --format text
./rimloc-cli validate --root ./MyMod
~~~

If macOS blocks an unsigned development binary, prefer a binary you built yourself or a project-provided artifact whose hash/build identity you can verify. Do not disable system security globally.

## Optional PO workflow

Only when you actually want a CAT handoff:

~~~bash
./rimloc-cli export-po --root ./MyMod --out-po ./work/MyMod.po --lang ru
~~~

PO is not required for the normal desktop project model.

## Build without PO

If translated Languages XML already exists:

~~~bash
./rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

## Common issues

- command not found: run with <code>./</code> (macOS/Linux) or <code>.\</code> (Windows) from the current folder;
- permission denied: <code>chmod +x</code> on macOS/Linux;
- wrong architecture: use a build for your CPU/OS;
- unexpected behavior: compare <code>--version</code> / commit with the documentation version you are reading.

For the newest development state, [build from source](install.md).
