---
title: GUI (Tauri)
---

# RimLoc GUI (Tauri)

RimLoc ships an optional desktop shell built with Tauri that wraps common CLI workflows.

## Features (MVP)
- Start: scan and export PO (with multiple TM roots).
- Validate: XML checks and XML health.
- Diff: source vs translation + changed source (baseline via CLI).
- Import / Build: dry-run previews for import and mod build.
- Lang Update: dry-run plan for updating official localization from GitHub.

## Run locally

Requirements:
- Rust toolchain
- Tauri CLI: `cargo install tauri-cli`

Run:

```bash
cd gui/tauri-app
cargo tauri dev
```

The app uses `rimloc-services` directly; no external binary is required.

## Notes
- Write operations are exposed via the CLI; the GUI focuses on previewing (dry-runs) for safety in early versions.
- Set paths in the form fields and press the action buttons; results appear in the panel below.

