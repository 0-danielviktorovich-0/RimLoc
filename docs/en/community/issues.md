---
title: Issue Guidelines
---

# Issue Guidelines

This page explains how to file actionable issues and which types we use. The repository includes GitHub forms that mirror these sections.

## Types

- Bug report — Something is broken or behaves unexpectedly.
- Feature request — A new capability or flag is desired.
- Documentation — Docs corrections or additions.
- Question — Clarifications about usage or behavior (consider Discussions).

## Bug report checklist

Please include:

- Full command: the exact invocation you ran.
  - Prefer `--quiet` for JSON output and cleaner logs.
  - Example: `rimloc-cli --quiet validate --root ./Mods/MyMod --format json --ui-lang en`
- Version and environment:
  - `rimloc-cli --version`, OS/shell
  - Env vars: `RUST_LOG`, `RIMLOC_LOG_DIR`, `NO_COLOR`, `NO_ICONS`, `RIMLOC_LOG_FORMAT`
- Expected vs actual behavior (1–2 sentences each)
- Attachments:
  - Console output (stdout/stderr). For JSON, paste the JSON. For text, set `NO_COLOR=1`.
  - File logs (`RIMLOC_LOG_DIR`), ideally with `RUST_LOG=debug`.
  - Minimal reproducible example: a tiny mod snippet (2–3 XML files) or a short `.po`.

## Feature request checklist

- Problem statement — What problem does this solve?
- Proposal — The desired behavior (flags, options, examples)
- Alternatives — Other approaches considered
- Acceptance criteria — How we’ll validate the feature (commands, output)
- Documentation impact — Which pages need changes

## Documentation changes

- Page(s) that need updates, links to sections
- Proposed text or examples (optional but helpful)
- Screenshots of issues in the rendered site (if any)

## Tips for good issues

- For JSON pipelines, always use `--quiet` to keep stdout machine‑readable.
- Use `RUST_LOG=debug` and attach `logs/rimloc.log` to capture rich traces.
- For placeholders, include `validate --format json` or `validate-po --format json` output.

## Avoiding duplicates

Before filing a new issue:

- Search open and closed issues for similar reports (use keywords from errors or command names).
- If you find a match:
  - Add a thumbs‑up reaction to the original to show interest (avoid “+1” comments).
  - Add a comment only if you bring new details (exact command, logs, versions, minimal repro).
  - If the issue is closed but the problem has resurfaced, explain what changed (version, OS, steps) and ask to reopen.
- If you’re unsure whether it’s a duplicate, open a new issue but link related ones under “Related issues” and explain why yours is different.
