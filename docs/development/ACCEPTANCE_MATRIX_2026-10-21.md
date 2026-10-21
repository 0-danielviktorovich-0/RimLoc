---
title: "Acceptance matrix — RimLoc campaign, 2026-10-21"
status: Current
date: 2026-10-21
related:
  - FINAL_ACCEPTANCE.md
  - MCP_SERVER_PHASE1.md
  - MCP_SERVER_PHASE2.md
  - SPATIAL_DESIGN_SYSTEM.md
  - BRIDGE_HEAD_TO_HEAD_2026-10-20.md
tags: [kind/acceptance, topic/campaign]
---

# Acceptance matrix — 2026-10-21

Supersedes FINAL_ACCEPTANCE.md (2026-09-23, iteration 1) as the CURRENT
state; the historical doc stays for the record.

## Lanes

| LANE | HEAD_SHA | WORKTREE/BRANCH | PR | CI | IMPLEMENTED | TESTED | NATIVE_E2E | SECURITY | BLOCKERS | NEXT |
|---|---|---|---|---|---|---|---|---|---|---|
| MCP Phase 1 (stdio server, read-only + guarded writes) | `b7232b16` | feat/mcp-server-phase1 | #104 | **12/12** | full v1 surface, fail-closed SharedProfile, paid-guard | 91 crate tests + integration suites | live stdio E2E (A1+A2min+A5, hash-tree source identity) | both audit P0s closed in code; CodeQL annotation triaged (owner call) | owner merge | — |
| Phase 2a broker (crate + GUI hosting) | `807bfab5` | feat/mcp-gui-broker | #105 | **13/13** | UDS 0600 broker, ≤4 clients, revision_changed fan-out, GUI-hosted single writer | 14+2 tests | unit; transport exercised live in A3 | paid-guard duplicated broker-side | owner merge | — |
| Phase 2b (broker client + React sync) | `7867bb5` | (same branch) | #105 | **13/13** | broker_client proxy mode, broker-changed listener, draft-safe adopt | 120 tests both crates | **A3 8/8 live** (below) | same guarantees through the proxy | owner merge | — |
| A3 concurrency acceptance | `fcaf0e9` journey | packaged build | evidence in #105 | — | — | — | **8/8 WDIO steps on the packaged app**: broker detected, wizard create, human translate, external apply → UI refetch w/o user action, draft survives, stale → typed `stale_revision`, screenshot | single-writer proven | — | — |
| UI spatial quality | `0f03e821` | feat/ui-spatial-quality | #107 | **13/13** | SDS scale --sp-1..8, waves 1+2, critique round-2 (hero, legible disabled, footer scrim, honest labels) | live probes (720px column), 36/36 visual-audit ×2 themes | BEFORE/AFTER captures incl. wide-window | contrast untouched | owner merge | remaining low-priority critique items listed in SPATIAL_CRITIQUE doc |
| Branch hygiene | wave 1 done | — | — | — | 32→24 branches, compare-and-delete journal `1a3c2df` | auditor v3, 28 unit tests | — | no unique commits lost | wave 2 (PR #90–94) frozen until owner approval | — |

## Beta journey (packaged app, isolated profile)

`integration-ui.spec` on the mcp-broker bundle (`fcaf0e9` + A3 spec): **26/26 PASS** —
wizard create → UI-11 source search (A/B/C) → UI-13 tree → UI-07/14 filters →
UI-03/08 panes/modes (drafts survive mode switches) → UI-05 canonical output →
UI-10 PO export/import guards → dark theme (with an explicit project REOPEN
through the wizard after reload) → UI-09 settings/projects → CLI↔GUI parity →
About → UI-01 zoom → UI-04 density persistence.

User-path coverage vs the beta checklist: установка (bundle artifact CI-built) ✅ ·
первый запуск/мастер ✅ · проект/обнаружение ✅ · перевод/редактор ✅ · сохранение
(durable envelope) ✅ · экспорт/PO-обмен ✅ · повторное открытие ✅ (dark-pass reopen).
Explicitly NOT verified this round: Windows/Linux packaged runs (no harness host),
skill-driven interactive TUI session (ZCode has no non-interactive exec mode).

## RimWorld in-game re-acceptance — BLOCKED (game environment, not RimLoc)

Status 2026-10-21: the export for the in-game run was produced by the NEWEST
head through the real product path (A2 wizard journey PASS on the packaged
build, guarded export `a2-export` @ 14:09, DefInjected
`VFES_Turret_AntiAir.label = ПВО турель (RimLoc)`) and mounted into the
isolated game profile (`RimLocA2Translation`, packageId `rimloc.1845154007`).

The game launch then hit the ISOLATED PROFILE'S MOD STACK, not RimLoc:
- first boot: `ReflectionTypeLoadException ... HarmonyLib.CodeInstruction` in
  VFESecurity — the isolated ModsConfig had Harmony LAST in the load order;
  fixed to Harmony-first (bridges logs: `[RimBridge] Applied essential
  Harmony patches`, 58 optional classes) — proves the fix works;
- second boot: VFESecurity's own defs fail (`VFES_Complex_*` cross-refs,
  `StatDef Beauty`) — Vanilla Expanded Framework was missing from the
  isolated list; added `VanillaExpanded.VFECore` before VFESecurity;
- third boot: the log freezes at 1287 lines with an unresolved-def error
  list (a mod-error dialog waits for interactive dismissal), and the GABP
  bridge never binds — so the language switch and screenshot step cannot
  run autonomously.

RimLoc-attributable log errors in every run: **0** (`RimLocA2Translation`
never appears in an error line; the frozen state exists with and without
the translation mounted). The mod stack of the isolated game profile needs
an interactive repair pass (dismiss the boot dialog, resolve the VFE/VEF
chain in the owner's mod list) before the in-game rendering proof can run.

Fresh artifacts kept: `a2-export` (newest-head export) remains mounted in
`RimLocA2Translation`; run logs in the Player.log of the isolated launch.

## Owner gates (unchanged)

Merge of #104/#105/#107 into the integration line (and later main), tag/release,
Discord/community posts, crates.io publish, wave-2 branch cleanup, CodeQL
annotation final call.
