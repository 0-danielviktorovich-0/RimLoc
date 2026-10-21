# Branch cleanup — wave 1 execution journal (2026-10-20)

Owner approval: message of 2026-10-20, exactly eight branches, wave 2 items
explicitly NOT approved (PR #90–94 untouched, docs/desktop-first-ai-integration
kept, no worktree removals, no main changes, no auto-delete settings).

Procedure per branch (executed by the audited script's data): fresh
`git fetch --prune` → fresh JSON manifest → pre-flight (content verified,
no worktree, CI quiet, no dependent PRs) → remote ref re-checked against the
audited full SHA → compare-and-delete
`git push origin --force-with-lease=refs/heads/<b>:<sha> :refs/heads/<b>`.

| Branch | Full SHA (expected = actual) | Result |
|---|---|---|
| chore/public-history-hygiene | 7024a13496d3ecd9db8986c7e60a18c0b59160e1 | DELETED |
| chore/remove-dead-glib-vendor | 374f1a828e4edd44c8705e04cec2cca833929e27 | DELETED |
| docs/pre-beta-main-sync | 358e17aa555c8e7ce1472ce2ef904aa05c4aea0f | DELETED |
| fix/codeql-advanced-setup | 1c3d263dd8c6ba6da6b9d18a9e9d14e415800e81 | DELETED |
| fix/llm-feature-matrix | ffe759af280b958473726d2ed6c57ae2e7aabd14 | DELETED |
| fix/tauri-dialog-async-portable-paths | d508e7acdfb9b7f2a9f5f8a0b13b45179033d38b | DELETED |
| fix/zcode-workflow-persistence | f85320de5b7a0b113ecefc0ea18960485afa0a20 | DELETED |
| release/wave-integration-2 | eeb1454e06fa48899b3b476622e4c11483ca4bd8 | DELETED |

Skipped: none — every pre-flight passed on the first pass.
Integration evidence per branch: contained as ancestors of
feat/integration-rel27 (public-history-hygiene, pre-beta-main-sync,
codeql-advanced-setup, tauri-dialog, zcode-workflow, wave-integration-2) or
PR merge commit reachable (remove-dead-glib-vendor #84, llm-feature-matrix
#83); heads unmoved since audit (PR headRefOid matched for the squash pair).
