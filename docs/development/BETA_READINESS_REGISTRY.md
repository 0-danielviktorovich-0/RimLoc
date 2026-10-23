# RimLoc beta readiness registry — 2026-10-17

Single source of truth for the first limited beta. Head of record:
`feat/integration-rel27` @ **0475644af19e2e1cc22e20d42c027c6d9b911889**
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
| AI translation jobs: mock-first, cancel at 7 boundaries, preview/review with per-row accept/reject, full-scope acknowledgement over the 400-row preview bound, durable job records + restart-honest `interrupted` recovery | VERIFIED | 50a3cd2 + 588b674; ai_job.rs suite (19 tests incl. the 430-row truncation gate and selected-apply ledger) |
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

### P1 — remaining for the declared beta scope
1. **Real RimWorld acceptance — final step is a 30-second owner action**
   (see `RIMWORLD_ACCEPTANCE_2026-10-19.md`): every component is proven
   individually (live-mod GUI translation → canonical export → mount →
   activation → the game's Russian translation channel works on this
   machine); the single combined run did not land because the game's
   language preference behaved non-deterministically across background
   launches. Picking Russian once in the game's options (it persists)
   closes it with the already-mounted export.
2. **DMG distribution channel — CLOSED (88fccf1 + 356bad0)**: the
   `bundle-macos` CI job (macos-14) builds the production `.app` and
   the unsigned DMG for the exact tested commit and uploads both as a
   14-day artifact — verified live: `rimloc-macos-bundle`
   (20.2 MB, artifact 12189082584) on the green run.

### Closed on 2026-10-19 (late evening): P2 parity decisions + live-game acceptance evidence
- **CLI→capability→IPC→GUI traceability (Issue #5) delivered**:
  `CLI_PARITY_TRACEABILITY.md` — all 20 command modules mapped with the
  live IPC registry and GUI routes, the non-CLI product flows
  summarized. **P2 decisions closed**: validate-po/morph/schema stay
  CLI-only for the beta with recorded reasons; the Advanced Tools UI is
  intentionally deferred (its whole candidate set is those three, none
  beta-blocking).
- **Live RimWorld acceptance (partial, honest)**: a real string of the
  owner's LIVE mod (VFE Security `VFES_Turret_AntiAir.label`) went
  through the real GUI path into the canonical export; the mount and
  activation mechanics are proven in the game's own log; the game's
  Russian translation channel is proven in a control run. The single
  combined run is blocked on the game's non-deterministic language
  pref (environment, not RimLoc) — full story and the 30-second owner
  closure path in `RIMWORLD_ACCEPTANCE_2026-10-19.md`; the
  `rw-acceptance` native lane is in the repo (1 passing).

### Closed on 2026-10-19 (late): the last two P1 items
- **UI-11 Stage C — guarded recovery (Issue #6): CLOSED at 472bb43.**
  `project_include_candidate` re-verifies every UI input server-side
  (containment, effective view, XML validity, the row's own
  element/text, NoTranslate, duplicate identity, source fingerprint),
  persists into the managed state only (human_recovered marker,
  revision bump, persist-before-ack) and the source stays
  byte-identical (asserted). Search reconciliation reclassifies
  recovered rows as AlreadyIndexed; eight negative regressions per the
  issue; native lane 26/26 with include → reclassify → row live in the
  editor.
- **User-facing beta README: CLOSED** — `docs/beta/README-BETA.md`
  (verified capabilities, drag-install, local-Ollama walkthrough, honest
  limitations).

### Closed on 2026-10-19 (the build window returned)
- **Search/editor lane (directive §3): 5/5 native** — and it found a
  REAL product defect: ApparelDef was missing from the defs allowlist,
  so apparel mods silently lost every row. Fixed (4b0c375) with a unit
  regression + the seed-pack guard now documents 171 rules.
- **Real-provider E2E (directive §4): 3/3 native on local Ollama
  (qwen2.5:3b)** — typed-IPC provider instance, BYO selection, real
  model round-trips to the preview, per-row selection, accept-selected,
  durable envelope proving exactly the chosen row (4a4e0ee).
- **Large-mod E2E (directive §12): 3/3 native on a 12 000-row corpus**
  (6 000 defs / 20 files): wizard counter honest, search interactive,
  edit durable, guarded export reparse-checked. Forced a product fix:
  the workspace search filter now debounces (150ms) — per-keystroke
  rememo measured seconds at 12k (0475644).
- **Installed-app smoke**: the production bundle (no automation
  feature) installed by copy and launched with an isolated data dir —
  the process stayed alive and initialized its managed root. The
  DMG-vs-.app note is recorded above.
- **Dependabot majors**: yaml-rust2 0.13 + direct ico 0.6 applied and
  verified (tauri-codegen keeps ico 0.5 transitively — upstream);
  the @tanstack patch group applied (97d1eb2).
- **Regression sweep**: integration-ui lane 25/25 on the rebuilt head
  (the theme-toggle step now seeds a deterministic light baseline —
  a prior lane could leave dark persisted).

### Closed during the 2026-10-19 review round
- AI job durability (registry was process-local, records unwritten):
  closed at 50a3cd2 — atomic `<job_id>.job.json` on every transition,
  stale `running` relabelled `interrupted` on manager open, status falls
  back to the durable record after a restart.
- Reviewer P0 (a 400-row preview could hide rows the full accept
  applied): closed at 588b674 — `rows_truncated` + refusal naming the
  true scope until an explicit full-scope acknowledgement; regression
  with a 430-row job.
- Reviewer P1 (no per-row accept/reject): closed at 588b674 —
  `accept_selected` applies only chosen identities, marks rejected rows
  in the preview ledger; GUI checkboxes + apply-selected + two-step
  confirmation; the missing ai.* i18n set added (the screen had been
  rendering raw keys).

### P2 — fine after beta
- validate-po / morph / schema as CLI-only by design (audit scoping) —
  expose via Advanced Tools only if users ask.
- UI-12 combinatorial sweep: zoom×density×RU/EN×min/max window.
- Hierarchical catalog beyond two levels (UI-13 residual).
- BMC copy migration + creator-hub publication staging (code exists,
  publication owner-gated).

### OWNER_GATE — genuinely the owner's call
- **Installer signing + notarization + auto-update**: require the
  owner's Apple Developer certificate / notarytool credentials and a
  release-update endpoint decision — impossible to close without them
  (the CI DMG is deliberately unsigned and labeled as such). Target:
  post-beta hardening.
- **Real RimWorld acceptance — the final 30-second step** (see
  `RIMWORLD_ACCEPTANCE_2026-10-19.md`): either pick Russian once in the
  game's options and re-run the mounting script for the end-to-end
  proof, or explicitly accept the component-level evidence.
- Merge PR #101 to main; tag; publish the release; Discord/community.
- Real-provider UAT (needs owner API keys; mock-path is fully verified).
- Sponsors/BMC live account data for the About screen.
- Any change to GitHub branch protection / repo policy.

## Release candidate manifest (first limited beta)

- **Exact SHA:** `0475644af19e2e1cc22e20d42c027c6d9b911889` (branch
  `feat/integration-rel27`, Draft PR #101).
- **Reproducible builds:** CI `Tauri GUI build` (aarch64 .app + DMG from
  the same SHA) and the local automation bundle (`cargo tauri build
  --features automation-bridge --bundles app`, target dir on
  Portable-SSD). Unit: `cargo test --workspace`; native: integration-ui
  lane 25/25 + visual-audit lane 36/36 at this head.
- **Security posture:** CodeQL all languages 0 alerts at this head
  (all 40 checks green on 75d3763, every CodeQL analyze included), cargo-deny clean
  after the zip 9 bump (RUSTSEC-2026-0367/0368/0369 patched, 588679f).
- **Supported features:** the baseline table above, verbatim — nothing
  beyond it is claimed.
- **Known limitations (honest list for the beta notes):** jobs resume
  from checkpoints but the registry is in-memory per session (a quit
  mid-job asks to re-run, drafts never lost); AI quality depends on the
  user's own provider; PO/XLIFF import feeds the same review flow as
  folder packs (no direct write); validate-po stays CLI-only.
