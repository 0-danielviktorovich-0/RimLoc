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

Get RimLoc CLI (choose one)
- Option A — Download a ready build (recommended): use the Install page, then run it from a terminal.
  - Guide: Install → Run Downloaded Build.
  - Windows: from the folder run `.\\rimloc-cli`; macOS/Linux: `./rimloc-cli`.
- Option B — Install via Cargo (requires Rust):
  - Install Rust: https://www.rust-lang.org/tools/install
  - Then: `cargo install rimloc-cli`

Why this matters?
- If you download a release/dev build, you do NOT need Rust or Cargo at all.
- Either way, the tool you will use is the same command: `rimloc-cli`.

Plain terms (no jargon)
- Terminal: a program where you type commands. Windows → PowerShell; macOS → Terminal; Linux → any terminal.
- Mod root: the top folder of the mod (what you would copy into `Mods/`).
- PO file: a single translation file with pairs “source → translation”; open it in Poedit.
- Placeholder: pieces like `%d`, `%s`, `{NAME}` — don’t change them in translations; RimLoc can check them.
- Dry‑run: do a safe preview without changing files.

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
- Windows: PowerShell; macOS: Terminal; Linux: any terminal.
- If unsure, add `--dry-run` to preview changes.

Troubleshooting
- “command not found: rimloc-cli”
  - If you installed via Cargo: open a new terminal or ensure `~/.cargo/bin` (Windows: `%USERPROFILE%\\.cargo\\bin`) is on PATH.
  - If you downloaded a release: run from the folder where you unpacked it — `.\\rimloc-cli` (Windows) or `./rimloc-cli` (macOS/Linux) — or add that folder to PATH.
- “mismatched placeholders” — fix the entry in Poedit; placeholders like `%d` or `{NAME}` must stay.
- “nothing to import” — maybe all PO entries are empty; use `--keep-empty` if you want to import placeholders as blanks.

FAQ
- Can I start with zero knowledge? Yes. You need only copy‑paste commands and Poedit.
- Can I translate without touching the original mod? Yes. Build a translation‑only mod with `build-mod`.
- Where do I get help? Join our Discord from the home page.
Why?
- This “packs” your translation as a standalone mod you can enable without touching the original. Great for sharing and team workflows.
