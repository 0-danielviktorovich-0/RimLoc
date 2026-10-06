---
title: Install
---

# Install RimLoc

RimLoc is currently **pre-beta**. The CLI has historical published artifacts, but the newest desktop/React work lives in active development and should not be mistaken for a stable release.

## What should I use?

| Goal | Recommended path |
| --- | --- |
| Try the newest desktop UI | build the React candidate from current source or use an explicitly provided owner/test artifact |
| Use CLI automation | build current source; crates.io can be used if an older alpha is sufficient |
| Install a stable public desktop release | not available yet |

## Build current CLI from source

Install Rust from <https://rustup.rs>, then:

~~~bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
cargo build -p rimloc-cli --release
./target/release/rimloc-cli --version
~~~

On Windows, run:

~~~powershell
.\target\release\rimloc-cli.exe --version
~~~

## crates.io CLI

If you specifically want the published CLI package:

~~~bash
cargo install rimloc-cli
~~~

!!! note
    crates.io may lag far behind the active development branch. Check the version before relying on new pre-beta functionality.

## Build the React desktop candidate

Requirements:

- Rust toolchain;
- Node.js 20+;
- platform prerequisites for Tauri 2.

~~~bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc/gui/tauri-app/frontend-react
npm ci
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
~~~

The default Tauri config still references the frozen Svelte fallback during migration. Use <code>tauri.react.conf.json</code> when your goal is to test React R1.

## GitHub Releases

The repository contains historical alpha/dev pre-releases. They are useful as history, not as proof that you have the newest React/Rust product.

Until a new beta release is published:

- prefer current source for development/testing;
- verify the commit/build identity of any artifact someone sends you;
- do not assume a file named RimLoc GUI.app is the newest React frontend.

## macOS Gatekeeper

Development builds may be unsigned/not notarized during pre-beta. Follow normal macOS security prompts only for artifacts you built yourself or received from the project owner and verified by hash.

Signed/notarized distribution belongs to the release-candidate phase.

## Verify a supplied test artifact

When an owner/test packet includes a SHA-256:

~~~bash
shasum -a 256 "RimLoc GUI.app/Contents/MacOS/RimLoc GUI"
~~~

Compare against the identity file supplied with that exact artifact.

## Next

- [Getting started](getting-started.md)
- [Desktop GUI](guide/gui.md)
- [CLI](cli/index.md)
- [Security](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md)
