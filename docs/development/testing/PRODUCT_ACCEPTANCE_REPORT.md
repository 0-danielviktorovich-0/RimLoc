# RimLoc — product acceptance report (P0–P7), 2026-10-23

Every claim below is backed by a live GitHub/CLI
reading or a local run whose log lives in this repository.

MAIN_SHA | WORKFLOW_RUN_ID + SUBAGENT_LANES | EXACT_RC_BUNDLE_SHA | ROUTE_STATE_CONTROL_INVENTORY | CRITICAL_ACTION_POSTCONDITION_COVERAGE | P0_P1_DEFECTS | UI_BEFORE_AFTER | EXISTING_PACK_A_B_C | AI_SUCCESS_FAILURE_CANCEL | REAL_RIMWORLD_KEYED_DEF | WIN_LINUX_INSTALL_SMOKE | SECURITY_WAIVERS_PENDING | CI_PR_LINKS | OWNER_UAT_READY_OR_NOT | NEXT_BLOCKERS

---

**MAIN_SHA:** `1194f7e6` (bundles) / product code identical to merged main `25b61f71` (PR #129 state).

**WORKFLOW_RUN_ID + SUBAGENT_LANES:** single ZCode native workflow, serial lanes
(this session): P1 UX (PR #123) → P0 QA-oracle (PR #124/#125) → P2 Issue #2 B/C
(PR #126) → P3 AI mock/cancel (PR #127 + mock-marker follow-up PR #128) →
P5 matrix + P7 final state (PR #129). CI dispatch run on merged main:
39914744050 (success, all three platform bundles).

**EXACT_RC_BUNDLE_SHA** (rc-1194f7e6/, deployed to owner UAT):
- macOS dmg `b76f77a0…` · app binary `6837c870…`
- Windows NSIS `08596435…` · Linux deb `6029ae8f…` · AppImage `bfac62f2…`

**ROUTE_STATE_CONTROL_INVENTORY:** `control-ledger.json` — 51 stable
route×state×control rows (union denominator, no per-run shrinkage);
`ROUTE_CONTROL_MANIFEST.md` pass-3 records the honest classification.

**CRITICAL_ACTION_POSTCONDITION_COVERAGE:** oracle runner asserts a
POSTCONDITION after every action (UI flip / disk / clipboard / applied
setting). Measured **46/51 = 90.2% verified, 0 FAIL**; 5 honest skips
(native pickers, external action buttons, browser download, mock-server-gated
AI cancel/stale — the cancel half is now proven, see below). Click-without-
effect is FAIL, never PASS; the stage-3 ACTIVATED set is gone.

**P0_P1_DEFECTS:** none open in the basic advertised flows. Recorded and
NOT hidden: (1) **a11y defect** — keyboard selection of the AI provider
instance leaves Run disabled (PR #127 lane, forced-handler diagnostic
proves the guard sees a stale id); (2) GUI success lane NOT_TESTED until it
is fixed. The TM manual-add "defect" from pass 3 was a RUNNER bug (free-text
locale input vs select-setter) — resolved, runner now types it like a user.

**UI_BEFORE_AFTER:** full source path + one-click copy + Open-file action
(contained via project_source_reveal) shipped in PR #123; context tabs
renamed Source / Terms / Source XML / Output XML in RU+EN. 19-screenshot
capture lane archived (design-shots-rc/); first design pass: no P0/P1, one
suspicion dismissed programmatically.

**EXISTING_PACK_A_B_C:** A — original + existing import: shipped, covered by
chatbatch E2E + the lanes below. **B — standalone pack WITHOUT original:
PROVEN** (stage5-pack.spec 4/4: wizard with source = pack language, Keyed +
DefInjected render, edit persists across reopen, guarded export verified on
disk). **C — attach original later: PROVEN** (stage5c-attach.spec 3/3:
original as its own project + pack import through Existing; contract requires
the language FOLDER — typed violation otherwise; pack translations land in
the original project). Issue #2 can now be closed as implemented, owner call.

**AI_SUCCESS_FAILURE_CANCEL:**
- FAIL path: PROVEN (stage2: dead endpoint → 0 translated / failed rows /
  empty target, nothing applicable).
- CANCEL: PROVEN (stage6: one delayed batch lands in flight, job cancelled,
  mock request counter flat afterwards — no further queued API calls).
- SUCCESS: engine path PROVEN via CLI over the same mock (4/4 translated,
  0 errors); **GUI success lane NOT_TESTED** behind the a11y defect above
  (WebDriver cannot click the native option popup). ChatGPT plan stays
  disabled with its honest note (incompatible with OpenAI ToS).

**REAL_RIMWORLD_KEYED_DEF:** Keyed PROVEN on RC source (file channel:
active language Russian, game serves «Привет, канареечный мир» from the
RC export). **DefInjected direct in-game read remains NOT proven** — the
bridge's read_def supports ResearchProjectDef only; the DefInjected file is
verified on disk and rides the same atomic Languages/Russian load. This is a
bounded, named limitation (bridge evolution item), not an inference.

**WIN_LINUX_INSTALL_SMOKE:** BUILD_VERIFIED (CI bundles + SHA256).
RUNTIME_NOT_TESTED — no Windows/Linux host or VM in this environment; named
gap in CROSS_PLATFORM_MATRIX.md. Linux CI has a virtual-display startup
smoke (flaked once, green on rerun — infra flake, not acceptance).

**SECURITY_WAIVERS_PENDING:** zero waivers applied. Open, awaiting the OWNER:
(1) 19 CodeQL records — false-positive annotation vs accept-as-tooling-risk
(matrix with sources/guards in the private handoff); (2) 16 dev-only npm
highs — recommended time-box to 2026-11-22; (3) basic-toml + 7 RustSec
unmaintained ignores stay documented. No mass dismissal performed.

**CI_PR_LINKS:** #123 (UX), #124/#125 (oracle), #126 (Issue #2 B/C),
#127 (AI mock lane, cancel proof, a11y), #128 (mock marker fix),
#129 (final release state). All merged green; CodeQL high on the test
evidence path resolved in code, not suppressed.

**OWNER_UAT_READY_OR_NOT:** **UAT_READY** — with the honest caveat list:
Windows/Linux installed-runtime acceptance NOT_TESTED; AI GUI success lane
NOT_TESTED behind the a11y defect; in-game DefInjected direct read = named
bridge limitation. Everything else above is verified. Package:
`~/Developing/RimLoc-evidence/owner-uat/rc-1194f7e6/` + updated UAT_GUIDE +
design-shots-rc/ (19 screens).

**NEXT_BLOCKERS:** owner decisions only — (1) UAT execution; (2) 19 CodeQL
FP decision; (3) npm time-box approval; (4) 12 diverged branches; (5) Issue #2
close; (6) public pre-release/tag/Discord/Reddit remain separately gated.

---

## Addendum — status after this report (2026-10-23)

The items this report listed as open were closed by the following merges;
the report body above is kept as the point-in-time record.

- **a11y keyboard-selection defect: FIXED** (PR #130). The AI screen
  selects commit on keyup (`keyboardCommitable`); a blur handler that
  re-rendered mid-click and ate Run presses was removed. The AI GUI
  success lane moved from NOT_TESTED to VERIFIED: real click-through of
  keyboard selection → Run → preview rows with per-action postconditions.
- **AI preview persistence: SHIPPED** (PR #136). A running/finished
  preview survives route navigation; the "stale accept unreachable"
  observation from the report is resolved. The Gate B starter lane
  (PR #138) asserts the persistence journey; its runtime stability is
  part of the remaining AX-lane work below.
- **Stage-6 postconditions strengthened** (PR #135): cancel quiescence is
  asserted against the mock request counter, stale-job cleanup asserted.
- **Gate B documented honestly** (PR #138): status file + starter lane;
  native pickers and narrow-window layout remain separate AX-lane work
  outside WebDriver scope — recorded as such, not claimed.
- **Ko-fi hidden until payment setup is ready** (PR #134): removed from
  FUNDING.yml, READMEs, MkDocs pages, About screen, external-URL
  allowlist and the canonical support registry; UI tests aligned to two
  promoted destinations.
- **Candidate refreshed**: the owner-UAT bundles are rebuilt from merged
  main 019755ba (product code 3603e08c) — hashes and deployed paths live
  in `.rimloc-release-state.json`, which supersedes the rc-1194f7e6
  pointers in the body above.

Superseded PR #137/#132 were closed without merge (stale duplicates of
lanes that landed in reviewed form as #138/#133).
