---
name: rimloc-<your-task>
description: One sentence naming the RimLoc task this skill drives (e.g. "Audit DefInjected coverage of a RimLoc project"). Include clear when-to-use triggers and an explicit NOT-for line excluding unrelated XML/game questions.
license: <your license>
compatibility: Requires RimLoc with the rimloc MCP adapter; discover tools via rimloc.capabilities at runtime.
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
- RimLoc installed and the MCP adapter connected
- Capabilities used (list each `rimloc.*` tool and why)

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

## Author / license / provenance
<Author, license, source repo, tested RimLoc + protocol revisions.>
