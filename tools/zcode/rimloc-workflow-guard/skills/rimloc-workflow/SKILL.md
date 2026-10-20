---
name: rimloc-workflow
description: Use for RimLoc release campaigns, autonomous multi-subsystem tasks, architecture, security, multi-agent delivery, integration and long-running parallel development. Requires the real ZCode CreateWorkflow tool when available; not for a trivial single-file fix.
license: GPL-3.0
metadata:
  version: "0.1.0"
  source: "RimLoc ZCode orchestration"
---

# RimLoc ZCode native Workflow

Before implementing a substantial RimLoc task, read the current workspace's `AGENTS.md`, `docs/development/ZCODE_WORKFLOW_RUNTIME.md` and `tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator/SKILL.md`. These are current engineering authority; this compact skill is only the discoverable trigger.

For work across two or more subsystems, three or more independent deliverables, architecture/release/security or user-requested parallelism:

1. Inspect live branch/worktrees and the **actual available tool list**, identify the workflow tool `CreateWorkflow` and existing runs. Do not invent them.
2. Invoke native `CreateWorkflow` (or `ResumeWorkflowRun` / `AmendWorkflow` for a verified existing run). Create a bounded script with phases, named agents, isolated worktrees, integration lane and deterministic acceptance checks. A markdown plan is **not** a running workflow.
3. Observe tool receipts and preserve the real run ID or exact tool result; update the campaign ledger. Track CI, CodeQL, reviewer bridge and relevant private owner directives.
4. Delegate implementation; watch/advance lanes and verify real product behavior before declaring DONE. Stop at genuine owner-only release gates. No automatic main merge, public release, Discord post, paid-provider call or new permission regime.
5. If the native tool is **genuinely inaccessible**, report `WORKFLOW_UNAVAILABLE:<exact probe failure>`, then use the best supported bounded fallback and do not represent it as a native workflow.
6. In the final status, report `WORKFLOW_RUN:<observed ID/receipt>` or `WORKFLOW_EXISTING:<verified ID>` or `WORKFLOW_UNAVAILABLE:<reason>`, plus lanes/test evidence.

For a small one-file change, work directly without starting an expensive Workflow.
