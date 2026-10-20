---
name: rimloc-release-orchestrator
description: Use for RimLoc autonomous release convergence, multi-subsystem development, 3+ deliverables, parallel agents, workflow campaigns, security/release/architecture work; invoke real ZCode CreateWorkflow rather than merely writing a plan. Not for trivial one-file fixes.
license: GPL-3.0
metadata:
  version: "1.0.1"
---

# Discoverable ZCode workflow entrypoint

This workspace skill advertises the existing `rimloc-release-orchestrator` to ZCode's native Skills discovery at `.zcode/skills/<name>/SKILL.md`. The full canonical methodology remains:

- `tools/zcode/rimloc-orchestrator/skills/rimloc-release-orchestrator/SKILL.md`
- `docs/development/ZCODE_WORKFLOW_RUNTIME.md`
- `docs/development/PRODUCT_CLOSURE_PREFLIGHT.md`
- `AGENTS.md`

Read those, check actual Workflow tool availability, create/resume/amend the native workflow with real receipt for multi-lane work and keep owner release gates. An `AGENTS.md` reference alone is not evidence of actual skill activation or workflow execution.

If the tool is absent, state `WORKFLOW_UNAVAILABLE:<specific observation>` and continue via a truthful bounded fallback. For a trivial one-file change do not over-orchestrate.
