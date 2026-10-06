---
title: Desktop GUI
---

# RimLoc desktop GUI

The desktop application is built with **Tauri 2** over the shared Rust service layer.

## Current frontend status

- **React 19 / frontend-react** — intended production UI and current R1 convergence target.
- **Svelte / frontend-v2** — frozen legacy fallback/reference during migration.
- **frontend/** — older historical shell; not the product direction.

Do not assume a random local <code>.app</code> is the React build. Development/automation builds have had stale-bundle collisions in the past, so owner/test artifacts should carry an explicit build identity.

## What the React workspace is for

The current product direction exposes the normal localization lifecycle:

- Home / recent projects;
- new translation;
- open/update existing translation;
- workspace/editor;
- target-locale switching;
- validation/checks;
- glossary;
- Translation Memory as it is enabled during pre-beta;
- build/export;
- diagnostics;
- settings and language management.

A control should only appear as usable when the backend/adapter capability is actually live.

## Normal translator workflow

1. Create/open a project.
2. Choose a source and target locale(s).
3. Translate directly in the editor.
4. Review context/source information.
5. Validate.
6. Build/export to an isolated output directory.

**PO is not required** for this workflow. PO is an optional interchange format for external CAT tools.

## Build the React candidate

Requirements:

- Rust toolchain;
- Node.js 20+;
- Tauri platform prerequisites.

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
~~~

For development:

~~~bash
cd gui/tauri-app/src-tauri
cargo tauri dev --config tauri.react.conf.json
~~~

## Safety

- Game/Workshop/source directories are treated as read-only inputs.
- Use a separate output directory for generated translation artifacts.
- Automated tests should use an isolated RimLoc data/profile directory so fixtures do not pollute Recent Projects.
- Production artifacts must not include automation/test bridges.

## Visual status

The React R1 interface uses the approved Lovable-derived design direction as the visual/interaction baseline. Screenshots in older docs may still show the frozen Svelte UI until the new owner-approved screenshot set replaces them.

## See also

- [Getting started](../getting-started.md)
- [Translator guide](translators.md)
- [Troubleshooting](../troubleshooting.md)
- [Frontend boundary](../../architecture/FRONTEND_UI_BOUNDARY.md)
