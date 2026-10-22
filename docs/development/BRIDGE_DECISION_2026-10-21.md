---
title: "Issue #13 — Bridge architecture decision (evidence-based)"
status: Final recommendation (owner decision pending)
date: 2026-10-21
related:
  - BRIDGE_HEAD_TO_HEAD_2026-10-20.md
  - evidence/in-game-2026-10-21/ACCEPTANCE.md
tags: [topic/architecture, topic/qa-bridge, kind/decision]
---

# Bridge architecture: evidence-based comparison and recommendation

## Proven today (2026-10-21, newest head, real game)

| Check | First-party (file channel) | Upstream RimBridgeServer/GABP |
|---|---|---|
| Boot with minimal mod list | **UP first try** (`v1.0.0 up; dispatch=sync-context`) | NOT BINDING (port closed, no sockets in game process; 2026-10-20 A1 worked — env drift, root cause unknown) |
| capabilities discovery | file-channel JSON, 8 ops | 128 mirrored MCP tools (GABS namespace) |
| state | **PASS** (language RU, game version, program state) | — (bridge not up) |
| keyed translation read | **PASS** («Привет, канареечный мир» from the live DB) | — |
| def-read | ResearchProjectDef only (extension candidate) | rimworld_read_def (128-tool surface, unverified today) |
| screenshot | op exists; blocked by occluded-window Metal suspension on THIS host (macOS behavior, not bridge) | take_screenshot worked 2026-10-20 |
| language switch | select_language op exists (untested today; pref-set worked at boot) | switch_language PROVEN 2026-10-20 |
| Network listener | **NONE** (file channel only) | TCP port + token |
| Security surface | **savedata-folder scoped, no network** | loopback TCP; token auth |
| Dependencies | none (standalone mod) | Harmony required |
| Install | unzip to Mods (user-doable) | unzip + Harmony + GABS config |

## Capability matrix (weights per owner's use case: RimLoc QA first)

| Criterion | Weight | FIRST_PARTY | UPSTREAM | Notes |
|---|---|---|---|---|
| Reliability on THIS host (21.10) | 5 | **10** (booted first try) | 0 (not binding) | single data point but decisive today |
| Proven on THIS host (20.10) | 5 | 6 (prior T6 PASS; file channel) | 8 (A1+A2+A3 screenshots) | upstream had richer proof yesterday |
| Security (no network) | 5 | **10** | 5 (loopback+token) | file channel has no attack surface beyond savedata |
| Translation QA (keyed/def) | 5 | **8** (keyed PROVEN live) | 7 (read_def advertised, proven 20.10) | |
| Screenshot | 3 | 5 (op exists; occluded-window limitation on macOS) | 7 (worked 20.10) | neither can capture a fully occluded window |
| Language switch | 3 | 5 (op exists, pref-set proven at boot) | 8 (in-game switch PROVEN 20.10) | |
| Tool breadth | 2 | 4 (8 ops, QA-focused) | 9 (128 tools via GABS) | breadth ≠ verified utility |
| Self-install by users | 4 | **9** (unzip, no deps) | 5 (Harmony + GABS config) | |
| Maintenance cost | 4 | **8** (one .cs, no protocol churn) | 6 (upstream MIT, SDK exists, GABS evolves) | |
| GABS/GABP compatibility | 2 | 0 (no adapter yet) | **10** (native) | first-party GABP adapter = new work |

Weighted: **FIRST_PARTY ≈ 6.9** · **UPSTREAM ≈ 5.6** (today's evidence);
on 20.10 evidence alone UPSTREAM ≈ 6.8 · FIRST_PARTY ≈ 6.2.

## The three options

### FIRST_PARTY_ONLY
**Pros:** zero network attack surface; booted today when upstream didn't;
minimal deps (no Harmony); user-installable in one unzip; RimLoc-QA-scoped
(the 8 ops cover the acceptance needs: state/keyed/def/screenshot/quit).
**Cons:** 8 ops vs 128; no GABS ecosystem; read_def covers only research
defs today; screenshot blocked by macOS occlusion (same for both).
**Maintenance:** one C# file; protocol v1 stable.
**Risks:** if QA needs broaden (UI navigation, def editing), extension work
is on us.
**Rollback:** the upstream stays installed in the game; switching = enable
in ModsConfig.

### UPSTREAM_ONLY
**Pros:** richest tool surface (128 ops); GABS integration free; proven
screenshots/language-switch on 20.10; MIT SDK for extensions.
**Cons:** **not binding today** (root cause unknown — the port never opens,
the game process holds no sockets; the same stack worked 20.10); requires
Harmony; network listener (loopback, but a listener); heavier dependency
chain for users.
**Maintenance:** tracking upstream + GABS updates; our QA scripts couple
to GABP op names.
**Risks:** today's bind failure is undiagnosed — depending on it alone is
a proven-bad idea THIS WEEK.
**Rollback:** first-party bridge stays in the repo.

### HYBRID (recommended)
**Pros:** first-party as the **default QA channel** (proven today, zero
deps, zero network); upstream as an **optional enhanced channel** when it
binds (screenshots with visible windows, in-game UI automation, GABS
interop); the acceptance matrix already runs on whichever is up.
**Cons:** two codepaths to test (mitigated: the acceptance doc records
which bridge produced which evidence; ops are disjoint).
**Maintenance:** first-party is primary (one file); upstream is
best-effort (no RimLoc code depends on it).
**Transition conditions:** if upstream's bind failure is diagnosed and
fixed, upgrade it to co-primary; if first-party needs GABS interop, build
a thin GABP adapter (op-name mapping, not a reimplementation) — estimated
small because the file-channel protocol already covers the semantics.
**Rollback:** each channel is independently disableable (remove from
ModsConfig).

## Recommendation

**HYBRID with FIRST_PARTY as the primary and the only REQUIRED channel
for RimLoc acceptance.** The decisive evidence: on 2026-10-21 the
first-party bridge booted and proved the translation live when the
upstream bridge did not bind. A required channel must work on the machines
we actually test on; an optional enhancement channel may come and go.
The upstream bridge's 128-tool surface is attractive for future UI
automation, but nothing in RimLoc's acceptance needs it TODAY — keyed +
state + source-integrity (all proven through the file channel) are the
product's acceptance criteria.

**Bridge-evolution backlog (priority-ordered, per real scenarios):**
1. `read_def` → general DefDatabase scan (ThingDef etc.) — closes the
   direct DefInjected evidence gap (small, isolated change).
2. Screenshot fallback: if the window is occluded, capture via
   `ScreenCapture.CaptureScreenshot` (file-based, not ReadPixels) — may
   bypass the Metal occlusion issue.
3. `select_language` in-game verification (the op exists; today's proof
   used the boot-time pref).
4. Capability versioning (`capabilities` already returns a version; add
   a feature-flags array for graceful client degradation).

GABP adapter: NOT recommended now — the official SDK path is the better
route IF GABS interop becomes a real requirement (it is not one today).

## What is NOT claimed

- Windows/Linux game runs (macOS-only host).
- Upstream bridge's 128 tools verified working (only advertised; the
  bind failure prevented any call today).
- First-party screenshot on an occluded window (macOS Metal limitation
  affecting both bridges).
