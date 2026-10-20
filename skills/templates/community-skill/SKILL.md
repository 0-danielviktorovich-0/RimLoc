---
name: rimloc-<your-task>
description: One sentence naming the RimLoc task this skill drives (e.g. "Audit DefInjected coverage of a RimLoc project"). Include clear when-to-use triggers and an explicit NOT-for line excluding unrelated XML/game questions.
license: <your license>
compatibility: Requires RimLoc with the rimloc MCP adapter (a separate component, Issue #10 — until it ships, a community skill cannot run live; discover tools via rimloc.capabilities when it does).
metadata:
  version: "0.1.0"
  repository: <your repo>
  scope: community
---

# <Task title>

<What the skill does in 2-3 sentences. It drives the REAL RimLoc
tools; it never edits source mods, spawns shells for the user or
invents missing tools.>

## Requirements
- RimLoc installed and the rimloc MCP adapter connected (adapter
  status: see the first-party `rimloc` skill's adapter note)
- Capabilities used (list each `rimloc.*` tool and why)

## Inputs / Outputs
- Inputs: <what the caller must provide — mod folder, locale, query,
  selection; where each value comes from>
- Outputs: <what the skill reports — paths, counts, typed results;
  what it never returns — secrets, unbounded dumps>

## Allowed side effects
<Every state change this skill can cause: which writes go through
guarded apply/export, what stays read-only. If a step has no side
effect, say so — "read-only" is a valid and useful entry.>

## Steps
1. `rimloc.capabilities` — verify every tool below exists, else stop.
2. ... (concrete tool calls with the arguments' meaning)

## Positive example
<A short realistic transcript-style example with expected outcomes.>

## Negative example
<What must NOT happen — e.g. applying without the pinned revision,
writing into the source tree, starting a paid provider.>

## Failure handling
<Typed refusals this path can hit and what the skill reports.>

## Tests
<How you verified the skill: which host, which RimLoc + adapter
revision, dated. If not yet run live, write "not run live" — an
untested skill is reviewable, a fake TESTED line is not.>

## Author / license / provenance
<Author, license, source repo, tested RimLoc + protocol revisions.>
