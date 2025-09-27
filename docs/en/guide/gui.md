---
title: GUI (Tauri)
---

# RimLoc GUI (Tauri)

RimLoc ships an optional desktop shell built with Tauri that wraps common CLI workflows.

## Features (MVP)
- Start: scan and export PO (with multiple TM roots).
- Validate: XML checks and XML health.
- Diff: source vs translation + changed source (baseline via CLI).
- Import / Build: dry-run previews and Apply actions with backups.
- Lang Update: dry-run plan and Apply action (backup existing folder).
- Annotate: dry-run plan and Apply (add/strip comments with source text).
- Morph: run morphology providers with filters/limits.
- Tools: dump JSON schemas; open last path.
- Logs: view tail of rimloc logs; auto-refresh.

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
- Write operations have confirmation prompts and backups where applicable.
- Use the context-menu (right-click) on “Plan Update (DRY)” to download with a progress bar.
- Hotkeys: Alt+1..9 to switch tabs.
- Set paths in the form fields and press the action buttons; results appear in the panel below.
