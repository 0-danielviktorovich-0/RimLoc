---
title: Documentation Style Guide
---

# 📖 RimLoc Documentation Style Guide

Goal: write for two audiences at once — translators/newcomers and developers — with clear value up front, real examples, and consistent cross‑links.

## Audiences

- Newcomer/translator: no coding background. Needs step‑by‑step flows, copy‑pasteable commands, screenshots, minimal jargon.
- Developer: builds from source, extends CLI/services. Needs commands, source links, testing, architecture notes.

Each page should start with “Why this matters” and a short scenario. Details and developer links follow below.

## Style and Tone

- Prefer simple, friendly language. Lead with actions, then explain.
- Open with the value: what problem it solves and why RimLoc helps.
- Use admonitions for tips, warnings, and examples:

```markdown
!!! tip "💡 Tip"
    Short, actionable advice.

!!! warning "⚠️ Important"
    What breaks if used incorrectly.

!!! example "✅ Example"
    Command + expected result.
```

- Include real CLI commands and a snippet of output. Verify examples run.
- Emojis are welcome for scanning: 💡⚠️✅🚀 (don’t overdo it).

## Terms and Glossary

- Explain terms inline in plain words and/or link to the Glossary.
- First mention links to `../glossary.md#placeholder`, `../glossary.md#po-portable-object`, etc.
- Link to external tools where relevant (Poedit, Git, Cargo).
- New terms must be added to `docs/en/glossary.md` (and mirrored in RU).

Link examples:

- Placeholder: `../glossary.md#placeholder`
- PO file: `../glossary.md#po-portable-object`

## Documentation Structure

Mirror across EN/RU. Suggested map:

- 🚀 Quick Start: `getting-started.md`
- Tutorials (step‑by‑step):
  - Translate a Mod: `tutorials/translate_mod.md`
  - Update Translations: `tutorials/update_translations.md`
  - Export to .po: `tutorials/export_po.md`
- RimLoc CLI commands: `cli/*.md` — start each with “Why use it” and examples.
- Formats & Concepts: `guide/*.md` — configuration, placeholders, PO, etc.
- Developers: `dev/*.md` — build, test, release, and this style guide.
- FAQ: `faq.md`
- Troubleshooting: `troubleshooting.md`
- Glossary: `glossary.md`

Navigation lives in `mkdocs.yml`; keep EN/RU sections aligned.

## Cross‑links and Examples

- Inside guides, link to the relevant CLI page (e.g., `../cli/validate.md`).
- Always link terms to the Glossary.
- In technical pages, link external tools (Poedit, Git, Cargo, VS Code).
- Store screenshots in `docs/assets/` with descriptive names (`cli-validate-ok.png`) and alt text. Optimize sizes.

Short example block:

```bash
rimloc-cli validate --root ./Mods/MyMod --format text
# Expect warnings list; exit code 1 on errors
```

## Formatting

- One H1 per page; then H2/H3.
- Use lists and tables when comparing options or modes.
- Use fenced code blocks with language hints (`bash`, `toml`, `json`).
- Use admonitions for tips, warnings, examples.

Comparison table (example):

| Command | When to use | Key flag |
|---|---|---|
| `validate` | Translation QA checks | `--format` |
| `diff-xml` | Compare source/translation | `--baseline-po` |
| `xml-health` | Validate XML structure | `--lang-dir` |

## Page Templates

Start of any tutorial:

```markdown
# Page title

Briefly: why it matters and what you’ll get.

## 🚀 Quick plan
1) … 2) … 3) …

!!! tip "💡 Tip"
    …

## Step 1. …
Command/screenshot/expected output.

## What’s next
See also: links to CLI/guides/Glossary.
```

CLI command pages: begin with “Why use it”, then “How to use”, then “Examples”.

## Maintenance and Updates

- Any new CLI command/flag → a page under `docs/*/cli/` + updated examples.
- Any new term → add to `glossary.md` (EN/RU).
- Any new feature → at least one step‑by‑step page or a “How to use” section.
- Keep README.md short: overview + quick start + link to the docs site.

PR checklist for docs changes:

- [ ] Includes “Why this matters” and clear steps.
- [ ] Real commands with verified output.
- [ ] Terms link to the Glossary.
- [ ] RU and EN versions updated.
- [ ] `mkdocs build` passes with no broken links.

## Balancing Newcomers and Developers

- First: what to do (simple steps). Then: how it works (details).
- Newcomers get copy‑paste commands and screenshots; developers get links to source, schemas, and tests.
- Avoid overloading pages with theory; use “See also” for deep dives.

## Where to edit and how to preview

- Files: `docs/en/**` and `docs/ru/**` (mirror structure).
- Locally: `python -m venv .venv && source .venv/bin/activate && pip install -r requirements-docs.txt && mkdocs serve`.
- Before release: `SITE_URL=… mkdocs build` to validate absolute links.

