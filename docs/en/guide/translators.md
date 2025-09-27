---
title: For Translators — Step‑by‑Step
---

# Translate a RimWorld mod with RimLoc (no stress) 🎯

This page teaches you how to translate a mod even if you’ve never used a terminal before. Short steps, copy‑paste commands, friendly explanations.

What you’ll do:
- Scan a mod to find text to translate
- Export a single .po file for Poedit (or any PO editor)
- Translate safely with placeholder checks
- Import your translations back into XML
- Build a translation‑only mod to test in game

Before you begin (one‑time)
1) Install Rust toolchain (gives you `cargo` and lets you install rimloc):
   - Windows: https://www.rust-lang.org/tools/install → “Install Rust” (default next/next)
   - macOS: same link; on Apple Silicon it’s ok
   - Linux: same link; follow the shell command there
2) Install RimLoc CLI:
   - Open Terminal (Windows: PowerShell)
   - Run: `cargo install rimloc-cli`

Why this matters?
- Rust is installed once to get `cargo install` — that’s how RimLoc CLI is installed.
- After that you’ll run everything below with a single tool: `rimloc-cli`.

Make RimLoc easy (optional but recommended)
Why helpful?
- A `rimloc.toml` config saves you from typing flags again and again. Create the file next to your mod. Minimal example:

```
source_lang = "English"
target_lang = "Russian"
game_version = "1.5"
list_limit = 100
```

Step 1 — Pick a mod folder 📁
- Find the mod you want to translate. Example path in this guide:
  - `C:/RimMods/MyCoolMod` (Windows)
  - `/Users/me/RimMods/MyCoolMod` (macOS)
  - `~/RimMods/MyCoolMod` (Linux)
Why?
- Commands need the mod’s root to find `Languages/*` and XML files under `Keyed`/`DefInjected`.

Step 2 — Scan and validate 🔎✅
- Scan (lists all strings in Languages/*/{Keyed,DefInjected}):
```
rimloc-cli scan --root "C:/RimMods/MyCoolMod" --format json > scan.json
```
- Validate (finds empty strings, duplicates, placeholder issues):
```
rimloc-cli validate --root "C:/RimMods/MyCoolMod"
```
If it shows problems, they are safe to fix later in PO or after import.
Why two steps?
- `scan` just lists what needs translation (saving to `scan.json` is handy to review or share).
- `validate` warns early about typical issues: empty strings, duplicate keys, suspicious placeholders. It saves time and prevents breakage.

Step 3 — Export .po for translation 📤📝
- One file for translators/CAT tools:
```
rimloc-cli export-po --root "C:/RimMods/MyCoolMod" --out-po "C:/RimMods/MyCoolMod.ru.po" --lang ru
```
Open the .po in Poedit (or any PO editor). Translate at your own pace. Keep placeholders intact (things like `%d` or `{NAME}`).
Why?
- You get a single, friendly file for translators/CAT tools. It contains both source and translation field — no need to hunt many XMLs.

Step 4 — Check placeholders in PO (optional but recommended) 🧪
```
rimloc-cli validate-po --po "C:/RimMods/MyCoolMod.ru.po"
```
If there’s a mismatch, Poedit entry is shown; adjust and rerun.
Why?
- Placeholders (`%d`, `%s`, `{NAME}`) are “slots” the game fills with numbers/names. If they’re removed or altered, the line may break. This check catches such issues before import.

Step 5 — Import translations back ⬅️📄
Two ways:
1) Single file (simple review, all keys go into `_Imported.xml`):
```
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --out-xml "C:/RimMods/_Imported.xml" --dry-run
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --out-xml "C:/RimMods/_Imported.xml"
```
2) Into the mod structure (recommended for release):
```
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --mod-root "C:/RimMods/MyCoolMod" --lang ru --report --dry-run
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --mod-root "C:/RimMods/MyCoolMod" --lang ru --report
```
Dry‑run prints the plan (what will be written). If it looks good, run without `--dry-run`.
Which to choose and why?
- Single file `_Imported.xml` — simplest for review and quick checks.
- Structured import — best for release: RimLoc writes entries to the same files/folders as the original. Easier to maintain and update.

Step 6 — Build a translation‑only mod (package to share) 📦
Option A: from the .po directly
```
rimloc-cli build-mod --po "C:/RimMods/MyCoolMod.ru.po" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru --dry-run
rimloc-cli build-mod --po "C:/RimMods/MyCoolMod.ru.po" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru
```
Option B: from an existing `Languages/Russian` tree
```
rimloc-cli build-mod --from-root "C:/RimMods/MyCoolMod" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru --dry-run
rimloc-cli build-mod --from-root "C:/RimMods/MyCoolMod" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru
```

Test in game
- Copy `MyCoolMod_RU` into your RimWorld `Mods` folder, enable it in the mod list, switch language to Russian.

Why is this important?
- Only in game you see real context: line breaks, lengths, where strings appear. If something’s off — fix the .po and re‑import.

Tips if you fear the terminal
- Copy‑paste commands exactly; paths in quotes `"..."` are your folders.
- On Windows use PowerShell, on macOS “Terminal”, on Linux any terminal.
- If unsure, put `--dry-run` first. RimLoc prints the plan without changing files.

Troubleshooting
- “command not found: rimloc-cli” — open a new terminal after `cargo install`, or ensure `~/.cargo/bin` is on PATH.
- “mismatched placeholders” — fix the entry in Poedit; placeholders like `%d` or `{NAME}` must stay.
- “nothing to import” — maybe all PO entries are empty; use `--keep-empty` if you want to import placeholders as blanks.

FAQ
- Can I start with zero knowledge? Yes. You need only copy‑paste commands and Poedit.
- Can I translate without touching the original mod? Yes. Build a translation‑only mod with `build-mod`.
- Where do I get help? Join our Discord from the home page.
Why?
- This “packs” your translation as a standalone mod you can enable without touching the original. Great for sharing and team workflows.
