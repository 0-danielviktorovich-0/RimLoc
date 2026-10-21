# RimLoc v0.1.0-beta.1 — Release Candidate Notes

**Status: PREPARED — awaiting owner approval for publish**

## What RimLoc Is

RimLoc is a local-first desktop tool for translating RimWorld mods: open a
mod, see every translatable string, translate with AI or by hand, validate,
and build a ready-to-install translation mod — the source mod is never
modified.

## What's In This Beta

### Core workflow (proven end-to-end)
- **Project wizard**: open any RimWorld mod folder (1.0–1.6, LoadFolders,
  conditional branches) — RimLoc builds a managed translation project
- **Editor**: three-pane workspace (file tree · entry list · editor),
  keyboard-first, virtualized for 10k+ strings, compact/comfortable density
- **AI translation**: local providers (Ollama) free and private; cloud
  providers (OpenAI/Anthropic/Z.AI) with keys in the OS keychain only;
  per-row accept/reject, never a batch blind-apply; paid providers through
  external MCP are disabled until a per-run consent mechanism exists
- **No-API mode**: export a structured prompt → paste the reply from any
  chat AI → strict import with provenance (machine work is never labelled
  as human)
- **Validation**: placeholders, duplicates, overflow, glossary adherence —
  findings with jump-to-entry
- **Export**: build a translation mod (Languages/DefInjected + Keyed),
  PO/XLIFF exchange, guarded output (the source mod stays byte-identical)
- **Drafts**: unsaved work survives language switches, app restarts, and
  external concurrent edits (proven live)

### External AI integration (MCP server)
- `rimloc-mcp` stdio server (MCP 2026-07-28): 9 read-only + 7 guarded
  write tools; external agents can open projects, propose translations,
  apply with revision/epoch guards — **proven live**: an agent translated
  a mod through the real server while a human worked in the GUI
  simultaneously (external edits appear in the UI without user action;
  drafts survive; stale edits are refused typed)
- Fail-closed profile isolation; paid-provider guard (non-local instances
  refuse before any secret is touched)
- GUI broker: one authoritative writer (the GUI process); the external
  server proxies through it when the GUI is running

### Proven in the real game (2026-10-21)
- A game-valid canary mod translated through the packaged RimLoc app and
  mounted in RimWorld 1.6: the game's own databases served the Russian
  translations — Keyed «Привет, канареечный мир» and DefInjected
  «Канареечный маяк» — language Russian active, source mod untouched,
  zero translation errors in the game log.

### UI/UX
- Warm OKLCH palette with wine accent, Literata display over Golos UI,
  hairline rhythm, eyebrow labels — light and dark themes (both tested,
  WCAG AA contrast verified computationally)
- Russian and English interface languages
- Spatial design system (4px grid, zone rules, adaptive density)
- Command palette (⌘K-style), zoom 100–150%, density control

## Known Limitations (honest)

- **Windows/Linux**: installers are CI-built but **not yet tested on real
  user machines** — first testers on those platforms are early adopters
- **RimWorld in-game screenshot**: proven programmatically (the game's
  databases serve the translations) but no visual screenshot captured
  (the game window was occluded during testing; macOS suspends rendering)
- **Windows MCP broker**: the GUI broker uses Unix sockets (macOS/Linux);
  Windows named-pipe support is post-beta
- **AI skills**: the RimLoc Agent Skill is tested live with Codex CLI
  (full scenario) and ZCode (discovery); other AI clients are untested
- **Large mod performance**: 10k+ strings are handled but first-open scan
  can take seconds; a progress indicator shows during scans

## Install

### macOS (tested)
1. Download `RimLoc_0.1.0-beta.1_aarch64.dmg`
2. Open and drag **RimLoc** to Applications
3. First launch: right-click → Open (unsigned beta)

### Windows (CI-built, not user-tested)
1. Download `RimLoc_0.1.0-beta.1_x64-setup.exe`
2. Run the installer (NSIS)

### Linux (CI-built, not user-tested)
1. Download `RimLoc_0.1.0-beta.1_amd64.deb` (Debian/Ubuntu)
   or `RimLoc_0.1.0-beta.1_amd64.AppImage` (universal)
2. DEB: `sudo dpkg -i RimLoc_*.deb` · AppImage: `chmod +x && ./RimLoc*.AppImage`

## First Steps

1. Open RimLoc → **New project** → pick a RimWorld mod folder
2. Wait for the scan (progress shown)
3. Click a row → type a translation → **Save & next**
4. Or use **AI translate** (local Ollama is free; cloud needs your key in
   Settings → Providers)
5. **Build & export** → pick an output folder → install the translation
   mod in RimWorld

## Feedback

- GitHub Issues: https://github.com/0-danielviktorovich-0/RimLoc/issues
- Discord: [link to be added on publish]

## SHA256

Manifest will be generated from the CI artifacts for the final SHA.
