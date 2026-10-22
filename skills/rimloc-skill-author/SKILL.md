---
name: rimloc-skill-author
description: Scaffold and validate a new task-specific RimLoc Agent Skill against the live rimloc MCP capabilities. Use when building or reviewing a community skill that drives RimLoc tools; covers the SKILL.md spec, safety rules, trigger hygiene and validation. Not for translating mods (use the rimloc skill) or general Agent Skills questions.
license: GPL-3.0
compatibility: Agent Skills specification (agentskills.io); validated with skills-ref where available.
metadata:
  version: "0.1.0"
  repository: https://github.com/0-danielviktorovich-0/RimLoc
  scope: first-party
---

# Authoring a RimLoc skill

1. Copy `skills/templates/community-skill/SKILL.md` and fill every
   field; the `description` is the ONLY always-loaded text — make it
   route (when to use / when NOT).
2. Discover reality before writing steps: run `rimloc.capabilities`
   and reference ONLY tools that exist. The adapter is a separate
   component (Issue #10) — until it ships, build against the planned
   Phase 1 surface (`docs/development/MCP_SERVER_PHASE1.md`) and mark
   the skill as awaiting the adapter instead of implying live tools.
   Capability checks belong in the skill's step 1, not in prose.
3. Safety is not optional: no source-tree writes, no bypassing the
   pinned revision/epoch, no silent paid-provider starts, no invented
   fallbacks. Each rule needs a matching negative example.
4. Cover every MUST of the standard §3 in the template: trigger,
   requirements, versions, capabilities, allowed side effects,
   recovery, inputs/outputs, positive+negative examples, tests,
   author/license. Keep SKILL.md compact; details live in
   `references/`. RU+EN when user-facing.
5. Validate: `skills-ref validate` (frontmatter/name/structure) where
   the runner exists; if it does not, fall back to the structural
   checklist — six frontmatter fields, name == folder, description
   routes with a NOT-for line, every referenced file exists — plus
   the policy checklist in
   `docs/development/AGENT_INTEROPERABILITY_QUALITY_STANDARD.md` §2-3.
   Record the tested RimLoc + protocol revisions in the skill.
6. Community skills are reviewed PRs with license/provenance; they are
   never auto-installed or "official" by existence.
