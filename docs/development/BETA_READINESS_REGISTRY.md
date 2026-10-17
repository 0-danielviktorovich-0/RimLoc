# RimLoc beta readiness registry — 2026-10-17

Single source of truth for the first limited beta. Head of record:
`feat/integration-rel27` @ **c51d1784782548ffe73974e35b373b7ed9cf1cd0**
(Draft PR #101; merge/release stay owner-gated).

Legend: P0 blocks a safe beta · P1 required for the declared beta scope ·
P2 fine after beta · OWNER_GATE genuinely needs the owner. No technical
work is hidden behind OWNER_GATE.

## Verified capability baseline (what the beta candidate DOES have)

| Capability | State | Evidence |
|---|---|---|
| Project create/open/switch over a real mod (wizard, 1.5/1.6 views) | VERIFIED | native lanes, wizard test |
| Canonical inventory (Defs + Keyed + DefInjected + TKey + patches) | VERIFIED | services suite 327 + corpus harness |
| Editor with durable drafts, recovery, collision-safe keys | VERIFIED | drafts lanes (drafts-*.conf.ts), unit race tests |
| AI translation jobs: mock-first, cancel at 7 boundaries, per-unit preview/review, accept through canonical intents, checkpoints for resume | VERIFIED | PASS_CANCELLATION (4 independent review rounds); ai_job.rs suite |
| prov-* provider factory (BYO keys via OS keychain, masked) | VERIFIED | provider lanes + typed refusals |
| Chat batch (export→external AI→import→guarded apply) | VERIFIED | chatbatch lanes |
| Existing-translation import (folder pack): analyze/apply, conflicts never overwritten | VERIFIED | session tests + Existing lane |
| PO + XLIFF 1.2 project exchange (export/analyze/apply, round trip) | VERIFIED | 328 unit + native lane 24/24 |
| Output preview (canonical serializer in-memory) + guarded export | VERIFIED | UI-05 lanes + export guards |
| Source Discovery Stage A+B (search, honest classification, counts, filter) | VERIFIED | native lane 25/25 |
| Visual quality: 16 routes × light/dark, WCAG AA (4.5/3.0) 0 violations, 982 text nodes, 32 screenshots | VERIFIED | this push (c72cd4f), evidence zip |
| Density + zoom controls (80–150%, comfortable/compact), 10px floor | VERIFIED | density lane |
| Data safety: read-only source tree, managed-root guards, typed refusals everywhere | VERIFIED | guard tests; rel26 accepted review |
| Packaging: signed-config .app + DMG via CI (Tauri bundle) | VERIFIED per-build | CI Tauri GUI build |

## Open items

### P0 — blocks a safe beta
*(none open at 2deb981 — the registry's purpose is to keep it that way;
any new P0 found by CI/CodeQL/lanes lands here)*

### P1 — required for the declared beta scope
1. **Installed-app smoke on the CI-produced DMG/.app** (not the local
   automation build): install → wizard → translate one row → export.
   The automation lane proves the same binary family, but the installer
   artifact itself has not been walked. Plan: reuse integration-ui lane
   against the CI bundle path; ~half a day.
2. **Large-project acceptance** (UI-12 residual + owner ask): a real
   10k+ string mod through create→translate→export. The 10k virtualizer
   stress lane exists (react-stress.conf.ts); the end-to-end corpus pass
   over a real large mod remains.
3. **UI-11 Stage C — guarded recovery** (Issue #6): per-candidate human
   approval for including out-of-inventory text into MANAGED project
   state only (epoch/revision/source-hash pinned, atomic stale refusal,
   no source writes). Stage B evidence is in; C is the write path.
4. **Beta README / honest supported-features list** for the beta bundle
   (this registry is the draft; the shipped artifact needs a user-facing
   cut).

### P2 — fine after beta
- validate-po / morph / schema as CLI-only by design (audit scoping) —
  expose via Advanced Tools only if users ask.
- UI-12 combinatorial sweep: zoom×density×RU/EN×min/max window.
- Hierarchical catalog beyond two levels (UI-13 residual).
- BMC copy migration + creator-hub publication staging (code exists,
  publication owner-gated).

### OWNER_GATE — genuinely the owner's call
- Merge PR #101 to main; tag; publish the release; Discord/community.
- Real-provider UAT (needs owner API keys; mock-path is fully verified).
- Sponsors/BMC/Ko-fi live account data for the About screen.
- Any change to GitHub branch protection / repo policy.

## Release candidate manifest (first limited beta)

- **Exact SHA:** `c51d1784782548ffe73974e35b373b7ed9cf1cd0` (branch
  `feat/integration-rel27`, Draft PR #101).
- **Reproducible builds:** CI `Tauri GUI build` (aarch64 .app + DMG from
  the same SHA) and the local automation bundle (`cargo tauri build
  --features automation-bridge --bundles app`, target dir on
  Portable-SSD). Unit: `cargo test --workspace`; native: integration-ui
  lane 25/25 + visual-audit lane 36/36 at this head.
- **Security posture:** CodeQL all languages 0 alerts at this head
  (verified on c72cd4f run; 2deb981 is docs-only), cargo-deny clean
  after the zip 9 bump (RUSTSEC-2026-0367/0368/0369 patched, 588679f).
- **Supported features:** the baseline table above, verbatim — nothing
  beyond it is claimed.
- **Known limitations (honest list for the beta notes):** jobs resume
  from checkpoints but the registry is in-memory per session (a quit
  mid-job asks to re-run, drafts never lost); AI quality depends on the
  user's own provider; PO/XLIFF import feeds the same review flow as
  folder packs (no direct write); validate-po stays CLI-only.
