# WP0 head-to-head bridge audit — Issue #13 (2026-10-20)

Same licensed game (1.6.4871, /Applications/RimWorld.app), same isolated
`-savedatafolder` profile, same macOS session; bridges swapped ONLY in the
isolated ModsConfig (never run together except where noted). No code was
deleted or replaced; the first-party bridge was rebuilt from
`testlab/runtime_bridge` against the REAL game Managed references.

## Provenance

| | RimLoc first-party (testlab/runtime_bridge) | pardeike upstream (RimBridgeServer) |
|---|---|---|
| Version / commit | v1.0.0 (in-tree, integration branch) | v2.3.0 release zip (2026-10-07) |
| License | GPL-3.0 (RimLoc repo) | MIT, own repo, Steam Workshop 3727949765 |
| Transport | file channel (savedata/QA commands→results), no listener | GABP over loopback + per-launch token via GABS |
| Runtime today | rebuilt + loaded live (log: `v1.0.0 up; run_id=…; runInBackground=True`) | loaded live (128 tools via GABS) |
| Distribution today | clone-only, machine-specific build paths (`../run/t6`) | downloadable release ZIP + Workshop + GABS + SDK |

## Live same-conditions results

| Test | First-party | Upstream |
|---|---|---|
| Discovery | `capabilities` op — 8 caps listed | `games_tool_names` — 128 tools |
| State | `state` ok (program_state, game_version, active_language Russian) | `get_bridge_status` ok |
| Localization Keyed | `keyed RimLocCanaryGreeting` → "Hello canary" | (same profile, A3 proven via labels) |
| Localization DefInjected | `read_def` op → NOT in v1 op set (NOT_TESTED/PARTIAL) | proven (A2/A3 canary labels RU) |
| Locale switch | `select_language` documented in source; not exercised this run | PROVEN live (EN→RU cycle, A1) |
| Screenshot | `capture.game_frame` + freshness documented; not exercised this run | PROVEN live (menu + inspect PNGs) |
| 50-call reliability | 50/50 verified, p50 265 ms / p95 271 ms (file-channel poll) | 50/50 verified, p50 8 ms / p95 9 ms (in-proc MCP) |
| Security: stale run id | **REJECTED live** (`RUN_ID_MISMATCH` — demonstrated) | per-launch token (GABS lease; not adversarially probed this run) |
| Background-safe | `runInBackground=True` from boot log; game not frontmost during all probes | PROVEN (focus stayed on other apps through screenshots) |
| Game lifecycle | launched/stopped via GABS DirectPath like upstream | same launcher |

## Reading of the numbers

- Upstream latency reflects an in-process MCP call path; first-party
  polls a file channel (25 ms poll loop ≈ expected, not a defect class).
  Both are 50/50 reliable under the same session.
- Capability surface differs by an order of magnitude (8 vs 128 ops) —
  but the Issue forbids counting documented tools as quality proof. The
  decisive gaps are FUNCTIONAL: DefInjected read, locale switch, UI
  semantics and screenshots are all PROVEN on upstream in this campaign
  (A1–A3), while first-party v1 has them documented but unproven this
  run (`read_def` absent from op set; select_language/screenshot not
  exercised).
- First-party unique strengths live: run-id rejection demonstrated
  adversarially; no listener by construction (offline guarantee);
  test-only isolation by design.

## Interim engineering conclusion (owner gate intact)

For the RimLoc QA lab (Issue #9 A1–A3) the upstream bridge currently
delivers the proven end-to-end surface; the first-party bridge is a
smaller, offline-by-construction channel whose core ops work but whose
coverage would need v2 work (DefInjected read, live locale switch,
fresh screenshot proof) to carry the same acceptance alone.

Options for the owner (no code removed, no migration performed):

1. **Hybrid (recommended for the beta)** — keep the proven
   upstream+GABS path as the lab driver; keep first-party as the
   security/offline reference channel; decide on consolidation after
   beta.
2. **First-party-only** — requires funding first-party v2 to parity
   (estimated: DefInjected read op, live select_language proof,
   screenshot freshness E2E, distribution decoupling from ../run/t6).
3. **Upstream-only** — simplest operationally; gives up the
   no-listener/offline guarantee and the in-repo control of the
   protocol.

Veto constraints observed: none on either side during this run.
