# BRANCH CLEANUP — WAVE 2 (2026-10-22)

Baseline: `main` = `19c20f7d3b8a50d982e16d6cb709e19cc0c7755b` (post PR #110).
Authority: owner mandate 2026-10-22 (wave-2 permission granted explicitly;
diverged branches still require separate approval per branch).
Method: exact `git merge-base --is-ancestor` per ref against current main +
`git cherry` patch-equivalence for diverged refs + `git worktree list` activity
check + `gh pr list --state all` provenance. SHA manifest fixed BEFORE deletion:
`RimLoc-review-handoff/inbox/zcode/branch-hygiene/2026-10-22/BRANCH_INVENTORY.md`.

Note on the independent audit: its ahead/behind numbers were computed against
its own baseline and disagree with direct ancestry for several refs; exact
ancestry against current main is authoritative and is what this wave used.

## Deleted (8) — every head an ancestor of main, no open PR, none remote-active

| Branch | SHA (pre-delete) | Integration proof |
|---|---|---|
| feat/integration-rel27 | 9dae18acfeb56d6fa854323c25db4914f099027d | PR #110 merge commit (fixes merged; owner mandate condition met) |
| feat/mcp-gui-broker | 1f77f85488ff3d380b90a7215a9c59fa36be6fe9 | PR #105 MERGED |
| feat/mcp-server-phase1 | 59be66abbe1ad3f948730b6b16aa1ce810695043 | PR #104 MERGED |
| feat/ui-spatial-quality | 7e944780210a020e26aa476abb62d17ab944c41e | PR #107 MERGED |
| fix/build-mod-po-optional | 79647fda6a7acff3bef6a9133fe860bf6b311a31 | ancestor of main |
| fix/funding-buymeacoffee-native | a9d9464e4f1fdbaccc456971ad51d24ef4a40487 | PR #109 MERGED |
| fix/provider-surface | 7de84cc4d2b6d1246688b3a9b89aced6e5c36201 | ancestor of main |
| fix/ui-gaps-2 | fb6bd0b127645224e883e8c3e3d0e8ec5b72851e | ancestor of main |

Each deletion verified by post-check `git ls-remote` (empty = gone).

## Preserved (mandate)

- `staging/beta-rc` (08475c05) — until Owner UAT and release-candidate
  reconciliation close, per owner mandate.
- `ci/truth-sync-19c20f7d` — head of open PR #111.
- Dependabot heads for PR #95/#96/#108 — already auto-deleted by GitHub after
  those merges; verified absent.

## Deferred — REVIEW_DIVERGED (11), owner approval required per branch

| Branch | unique commits (patch-id) | last activity | Assessment |
|---|---:|---|---|
| ci/codecov-integration | 26 | 2026-10-06 | coverage CI work; coverage workflow already lives in main — likely superseded by a different implementation |
| ci/codecov-integration-v2 | 1 | 2026-10-06 | successor of the above; one docs commit |
| ci/public-git-hygiene | 13 | 2026-10-07 | commit-subject enforcement — equivalent guards (hook + check-public-git-language.py) already in main |
| ci/workflow-audit-hardening | 4 | 2026-10-06 | old sync-merge lineage |
| docs/desktop-first-ai-integration | 5 | 2026-10-14 | reviewer-continuity docs; partially superseded by later docs |
| docs/pre-beta-documentation-refresh | 46 | 2026-10-06 | pre-beta doc refresh — superseded by the current documentation set |
| feat/ai-translation-gui | 61 | 2026-10-15 | ACTIVE lane (worktree occupied, Issue Stage B); do not delete while AI acceptance stage is open |
| feat/source-discovery-scanner-audit | 51 | 2026-10-14 | worktree occupied; unique audit work unreconciled |
| feat/ui-ux-rel27 | 58 | 2026-10-14 | worktree occupied; superseded content reached main via integration merges |
| feature/ui-r1-convergence | 3 | 2026-10-06 | CI semantics commits; the P0 counterexample branch from wave 1 review |
| release-plz-2026-09-20T17-07-02Z | 50 | 2026-09-20 | automated release-plz output; release workflows are parked by owner decision |

Local branches (~80) are untouched: 25 are bound to active worktrees; the rest may
belong to other agent sessions. Remote deletions never destroy commits — every
deleted head remains reachable from main.

## Post-deletion validation

- remote branch count 25 → 17 (main included)
- `main` still `19c20f7d` (untouched), tags untouched (21)
- open PRs resolve to live heads only (PR #111)
- 55 worktrees intact
