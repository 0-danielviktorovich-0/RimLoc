# Multi-lane product closure: preflight, live gates and reviewer handoff

**Authority:** owner mandate + live GitHub/CI + actual source/test artifacts.
**Snapshot:** 2026-10-14 (JST), for orientation only. Re-poll before action.
**Protected:** rel26 UAT `089b424`; no merge, release, public alpha, Discord,
paid inference or OAuth grant without the owner's explicit decision.

## Why this exists

The AI/source/UI effort is **not** a completed campaign despite multiple Draft
PRs and a UAT zip. A one-agent implementation skipped the repository's existing
`rimloc-release-orchestrator` / ZCode Workflow subagent policy and closed its
report while exact-head CI was red. Stale release-state documents also referred
to older candidates. These are governance/verification defects, not grounds to
delete successful source work.

Read the **real** `AGENTS.md` policy, the existing skill in
`tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator/SKILL.md`,
and `docs/development/ZCODE_WORKFLOW_RUNTIME.md` before coding. Do not create
a new incompatible orchestration framework; discover and reuse the native one.

## First-action preflight (required)

1. Run `git worktree list`, inspect all clean/dirty trees, `git status`,
   `git branch -vv`, `git log` and exact SHA of main/PR branches. Preserve
   existing work. No force-push, deletion or unsafe reset.
2. Read local ZCode tool catalog. If `CreateWorkflow` is available, create a
   persistent workflow with `phase`, named `agent(...).ask`, deterministic
   `world.run` checks and progress artifacts per the documented runtime.
   Parallel work must use separate worktrees and separate Cargo target dirs.
   If unavailable, record `WORKFLOW_UNAVAILABLE:<reason>` and use the best
   supported bounded alternative — **do not claim it ran**.
3. Publish a DAG with deliverables, owners, touched file hot spots, worktree
   names, explicit merge/integration order, P0 data/security hazards and test
   evidence needed for every lane. In particular, reserve
   `contract_adapter.rs`, `main.rs`, `App.tsx`, `client.ts`,
   `transport.ts` and `types.ts` for the **integration lane** rather than
   conflicting concurrent writers.
4. Compare live PR heads/CodeQL/Codecov comments/checks to on-disk work and
   stale `.rimloc-release-state.json` / `RELEASE_CONVERGENCE_STATE.md`.
   Write the reconciled state and owner gates into a durable campaign ledger
   **without claiming a private local artifact is on GitHub**.
5. Before claiming an external review, attempt `local.handoff` only if the
   exact agent runtime exposes it (`reason`, `prompt`); capture receipt and
   response. Otherwise deliver sanitized REVIEW_REQUEST/evidence to the
   **existing private** `0-danielviktorovich-0/RimLoc-review-handoff`
   mailbox, and set `WAITING_EXTERNAL_REVIEW`. GitHub comments do not
   automatically trigger a ChatGPT conversation.

## Live reality at this snapshot — DO NOT REUSE AS CURRENT WITHOUT POLLING

| Draft PR | Exact head prefix | CI observed | What is and is not done |
| --- | --- | --- | --- |
| #90 Data Safety | `81216ca` | 40 success, 2 skipped | Conditional-pass rel26; owner UAT/RimWorld still pending |
| #91 Source Discovery | `4e89f03` | 2 failures | Stage A only; Windows source_search 6 tests FAIL; CodeQL alerts on path expressions; manual adopt/scanner-audit report unimplemented |
| #92 Workspace UI | `60bbad8` | 1 failure | Zoom/XML source read-only preview implemented; Tauri coverage FAIL; full Lovable defect audit/panel modes/filters/responsiveness/CLI→GUI gaps deferred |
| #93 Desktop-first docs | `9c98dae` (before this guidance) | green | Docs and product contract, not code execution |
| #94 AI GUI | `64bd7b3` | 3 failures | Mock-backed job prototype/UI only; clippy/build/Tauri coverage FAIL; ChatGPT OAuth unimplemented; no real paid/local provider UAT |

Each PR targets `main`, but their histories overlap and are **not** a clean
linear stack. `#91` vs later `#90` is diverged; `#92` vs `#91` also
diverged; `#94` extends rel26 but lacks source-discovery/UI edits. Therefore
an integrated candidate with explicit conflict reconciliation and complete
end-to-end tests is needed before any unified "ready" verdict.

### AI review risks (STATIC ONLY — must prove or disprove by regression)

`rimloc-services/src/ai_job.rs` on #94:
- Rows are indexed with `format!("{:020}{}", 0, e.id.key)` and the provider
  request uses `TranslateUnit.id = key`; this **collapses two different
  `ThingDef`/`HediffDef` identities sharing `Dup.label`** despite
  `row.identity_json` being correct. Add a two-identity provider/mock test,
  verify independent outputs and no silent omission.
- `accept_job` resolves `snap.acked_revision` at **accept time**, not a
  pinned job-start revision/source fingerprint. A translation/manual draft
  modified during the job may be overwritten; test edit-during-inference,
  target-locale changes, source drift and stale epoch/revision.
- Provider Settings stores secrets in Keychain under `prov-...`
  **instance ID**, while `rimloc-llm::KeySource::Auto` looks up provider
  preset names like `openai` and `zai`. Selecting a configured GUI
  provider may not work. Jobs must select a real provider instance and
  resolve its credentials securely through the existing sink, never
  environment variables or account-cookie scraping.
- Job ids are process-local `AtomicU64` + shortened project name, and the
  declared `.job.json` path is unused; checkpoint/job restart-resume and
  source/model/locale binding are not yet proven. Test restart, independent
  jobs, collisions and incomplete stream.
- UX: the provider selector defaults to `mock`, scope is only untranslated,
  language list is fixed and the UI cannot approve/reject specific preview
  rows; do not claim full model integration or human-in-the-loop control.
- Back-end engine `source_lang` currently hardcodes `en`; use actual
  project source locale and validate target locale. **No live paid call**
  is needed to reproduce these issues.

All observations above are static risks from the PR code, **not** claims that
the packaged .app demonstrably loses user data. Treat as P0/P1 until
reproduced/closed with negative tests and shared code review.

## Required workflow lanes

| Lane | Deliverable, ownership and independent checks |
| --- | --- |
| Coordinator | Durable DAG, worktree/CI checks, branch integration, receipts, owner gates; no hot-spot edits by two writers |
| Security/Search | #91 path containment + CodeQL 4 findings triage, Windows fixture regression, Stage A honest acceptance |
| AI Service/Safety | #94 full structural identity, pinned revision, provider-instance secrets, restart/cancel, mock/live policy; typed Rust/IPC tests |
| GUI/UX | #92 actual Lovable screenshot/code comparison, responsive overflow, ChatBatch layout, filters/scroll/zoom, panel modes, truthful XML preview, full CLI→GUI matrix |
| Independent Verifier | Reproduce critical bugs (without taking ownership of implementation), confirm exact SHA CI/security/Codecov comments, packaged macOS testing, private reviewer handoff |
| Future (separate) | Official ChatGPT OSS OAuth, Source Discovery adoption/audit Stage B/C, live API integrations **only after** safety Stage A/B gates |

The coordinator may adjust the number of agents to available capacity, but
MUST state lane ownership, review independence and why work is serialized
where required. Do not fork many writers on the same shell/contract files.

## Completion contract (machine and human)

Allowed statuses:
`NOT_STARTED`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `WAITING_EXTERNAL_REVIEW`, `OWNER_ONLY`.
No vague `DONE`. Every VERIFIED entry needs:
`lane, feature, exact SHA, tests, screenshots/artifacts, CI and CodeQL status,
reviewer receipt, known limitations, owner gate, timestamp`.

**Gate order:** implementation → local tests → push → exact-SHA GitHub checks
→ CodeQL/Codecov comments triage → integrated cross-feature tests → native
macOS packaged-app E2E → independent reviewer → owner UAT → authorized merge.
A UAT app built while GitHub is red is `EXPERIMENTAL_UNVERIFIED`, not ready.
Source search Stage A is not manual recovery Stage B/C. Provider ping is not
AI translation. A disabled ChatGPT button is not OAuth.

Report owners' actionable choices separately: login consent; paid API use;
RimWorld personal testing; advisories/signing; merge and release. Everything
else is agent-owned, using native permissions and no unnecessary confirmation
loops. Continue safe work through the verified goal, not just a kickoff plan.
