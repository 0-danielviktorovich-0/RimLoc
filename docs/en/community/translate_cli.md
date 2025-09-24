---
title: Translate RimLoc (No Coding Required)
---

# Translate RimLoc (No Coding Required)

Help bring RimLoc to your language! This short, friendly guide shows how to translate the CLI messages using simple text files (Fluent/FTL). You can do it entirely in the browser — no Rust setup needed.

## What you’ll edit

RimLoc’s texts live in two files per language:

- `crates/rimloc-cli/i18n/<lang>/rimloc.ftl` — CLI help and messages
- `crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl` — small test strings

English is the source of truth: `crates/rimloc-cli/i18n/en/`.

## Quickest path (GitHub web editor)

1) Open the English folder: `crates/rimloc-cli/i18n/en/`
2) Create a new folder next to it named with your language code (e.g. `es`, `de`, `fr`).
3) Copy both files from `en/` into your new folder:
   - `rimloc.ftl`
   - `rimloc-tests.ftl`
4) Edit the copies and translate only the values — keep keys and placeholders as‑is.
5) Click “Commit changes” → “Create pull request”. In the PR, add a note with your locale code.

That’s it! CI will run checks and we’ll help polish anything that’s missing.

## Prefer local edits? (Optional)

```bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
# copy en → <lang>
cp -R crates/rimloc-cli/i18n/en crates/rimloc-cli/i18n/<lang>
# translate files in crates/rimloc-cli/i18n/<lang>

# (optional) verify locally
cargo test --package rimloc-cli -- tests_i18n
cargo test --workspace
```

Preview the localized help (requires Rust locally):

```bash
cargo run -q -p rimloc-cli -- --ui-lang <lang> --help
```

## Keep placeholders intact

Placeholders are tokens like `{count}` or `%s`. They must stay exactly the same between English and your translation. See the short guide: Guides → Placeholders.

Examples (do not change tokens):

```
EN: Found {count} files
ES: Se encontraron {count} archivos

EN: Invalid value: %s
DE: Ungültiger Wert: %s
```

## Friendly checklist

- [ ] Keys unchanged (only values translated)
- [ ] Placeholders intact (`{…}`, `%…`)
- [ ] Both files present: `rimloc.ftl`, `rimloc-tests.ftl`
- [ ] PR includes your language code and, if possible, a screenshot of `--help`

## Also translate the documentation (optional)

RimLoc’s website is built with MkDocs. You can translate the docs pages too:

- Copy pages from `docs/en/...` to `docs/<lang>/...` using the same folder and file names.
- Keep sections aligned across languages (same structure and headings).
- Preview locally:

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs serve
```

- For a new language, a maintainer needs to add it to `mkdocs.yml` under the `i18n` plugin (`languages:` list). Open an issue or mention it in your PR — we’ll help wire it up.

## Need help?

- Open an issue with questions (Community → Issue Guidelines)
- Start a draft PR — maintainers will guide you

Thank you for making RimLoc friendlier for everyone!
