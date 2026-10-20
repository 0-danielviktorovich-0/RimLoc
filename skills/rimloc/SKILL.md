---
name: rimloc
description: Translate, review and export RimWorld mods with the real RimLoc desktop app. Use when the user wants to localize a RimWorld mod (Defs/Keyed/DefInjected/patches), review AI-proposed translations row by row, validate a translation project or build an exportable translation mod while keeping source files untouched. Not for general XML questions, non-RimWorld games, or editing mods by hand in a text editor.
license: GPL-3.0
compatibility: Requires RimLoc (GUI or CLI) installed locally and the rimloc MCP adapter connected. Protocol revision discovered at runtime via rimloc.capabilities; do not assume a version.
metadata:
  version: "0.1.0"
  repository: https://github.com/0-danielviktorovich-0/RimLoc
  scope: first-party
---

# RimLoc — mod translation through the real product

This skill drives the INSTALLED RimLoc application (visual editor + Rust
services). It never edits a mod's source files: RimLoc copies nothing
into the source tree, translations live in a managed project, and every
export goes through an explicit guarded step. If RimLoc is not running
or the MCP adapter is absent, say so and stop — never simulate tools.

## Start here (every task)

1. Discover what is actually available:

```
rimloc.capabilities
```

   Record the protocol revision and the tool list. If a tool this skill
   names is NOT in the response, tell the user and use only what exists.
2. See the projects: `rimloc.projects.list`.
3. To work on a mod the user names, open or create a project
   (`rimloc.project.open` / wizard) and read
   `rimloc.project.snapshot` for: source locale, target locale,
   revision, session epoch. Quote them back before any write.

## Novice path — "translate this mod to Russian"

See references/novice-walkthrough.md for the full annotated version.

1. Open the mod as a project; confirm the source tree stays read-only.
2. Search entries (`rimloc.entries.search`) to find the strings to
   translate; show the user a bounded list (20–50), not the corpus.
3. Propose translations DRAFT-only (`rimloc.translation.propose`).
   Drafts are never committed by proposing.
4. The user reviews: apply exactly the selected identities
   (`rimloc.translation.apply` with the pinned revision/epoch from the
   snapshot). Nothing else is applied.
5. Validate (`rimloc.validation.run`), fix findings, then export
   (`rimloc.export.build`) to a directory the USER chose. The export
   writes a translation mod; the source mod is untouched.
6. Report the export path and what changed. Do not claim in-game
   verification unless the in-game lab was actually run.

## Professional path — existing translations / large mods

references/translate-review-export.md covers: importing an existing
translation folder or PO/XLIFF exchange, the conflict rules (existing
human work is never overwritten), per-row AI review
(accept-selected/reject; unselected rows stay reviewable), the
full-scope acknowledgement for previews over 400 rows, and safe
re-export after source drift (fingerprint refusal → rescan).

## Local vs paid AI

RimLoc's AI translation works with a local provider (e.g. Ollama) or
the user's own API keys stored in the OS keychain. Never start a paid
provider job without the user's explicit go-ahead in THIS conversation.
Never ask for or store keys yourself. See references/ai-modes.md.

## Safety rules (always)

- Source mod files are READ-ONLY. If any step would write there, stop.
- One writer: if the GUI has unsaved drafts, external applies must not
  clobber them — re-read the snapshot and warn on revision drift.
- Every apply needs the pinned revision + session epoch from a FRESH
  snapshot taken after the last user-visible change.
- Exports only to user-chosen directories, through the guarded build.
- Report failures with the typed error codes RimLoc returns; never
  paraphrase a refusal as success.

## When RimLoc is unavailable

State exactly what is missing (app not running / adapter not connected
/ tool absent from `rimloc.capabilities`) and what the user can do.
Do not invent fallbacks that bypass RimLoc. See references/troubleshooting.md
(host matrix in references/supported-hosts.md).
