# RimLoc ZCode Workflow Guard (opt-in)

## Why

RimLoc already says `CreateWorkflow` is required for a real multi-subsystem campaign, but a 30KB `AGENTS.md` does not activate a ZCode tool. The original orchestrator skill lives in `tools/zcode/rimloc-orchestrator/skills/`, **not** in ZCode's auto-discovered `.zcode/skills/` or `~/.zcode/skills/`. A long-running agent can consequently do serial edits and report "all done" without ever launching the runtime Workflow.

ZCode docs (checked 2026-10-20): [Agent instructions](https://www.zcode.network/en/docs/agents/), [Skills](https://www.zcode.network/en/docs/skill/), [Hooks](https://www.zcode.network/en/docs/hooks/), [Plugins](https://www.zcode.network/en/docs/plugin/).

## What this does

- A project-discoverable skill at `.zcode/skills/rimloc-release-orchestrator/SKILL.md` exposes the existing methodology without a user mentioning it in every prompt.
- This **optional plugin** injects the actual Workflow requirement at `SessionStart` and on substantial `UserPromptSubmit` requests, detects successful parent `PostToolUse(CreateWorkflow|AmendWorkflow|ResumeWorkflowRun)`, and at `Stop` gives limited feedback if a campaign was reported without a native receipt or precise `WORKFLOW_UNAVAILABLE:<reason>`.
- Only acts inside a checkout containing the actual RimLoc files. Trivial single-file fixes do not need a Workflow.
- It **does not invoke CreateWorkflow itself**; hooks cannot directly run the Agent tools. They prompt/guard the **main ZCode Agent** which can invoke them.
- Stores only hashed per-session IDs and boolean/timing receipts under `ZCODE_PLUGIN_DATA`. No prompts/transcripts, tokens, worktree file changes or network access. Reviews remain subject to native permission rules. Does not auto-merge, publish or silently make paid calls.

## One-time local installation (must be done in your own ZCode)

1. Update/pull the branch containing this plugin into your **actual local** RimLoc workspace.
2. In ZCode, go to **Settings → Plugins → Create → Add marketplace** and select **the existing directory** `<actual RimLoc root>/tools/zcode` (its `marketplace.json` is the marketplace entry point).
3. Find `rimloc-workflow-guard` in the Personal section → **Install** → enable. It will automatically include the `rimloc-workflow` skill.
4. Check **Settings → Hooks** for `SessionStart`, `UserPromptSubmit`, `PostToolUse`, `Stop`. Check **Settings → Skills** for the enabled `rimloc-workflow` skill; click Refresh if necessary. A second workspace-discoverable skill `rimloc-release-orchestrator` is in `.zcode/skills/`.
5. **Start a NEW ZCode task/session** inside the real RimLoc workspace. Hook configuration is snapshotted at session startup. An already-running session may keep its previous configuration; a plugin committed to GitHub is NOT a plugin installed on the user's Mac.
6. Verify by requesting a harmless two-lane campaign; look for a real `CreateWorkflow` tool call, its run receipt/board, or an explicit `WORKFLOW_UNAVAILABLE:<actual reason>`. Neither an agent's prose nor a file containing a Workflow plan proves execution.

You only install once. Later user prompts do **not** need to mention workflows unless explicitly overriding the usual classification. Disable or uninstall the plugin via Settings → Plugins to remove the hooks.

## Critical ZCode limitations

- **Project-level hooks do not execute in current ZCode**; adding `hooks` to `<workspace>/.zcode/config.json` will not help. User-level or enabled plugin hooks are the supported source. Do not weaken ZCode permissions or bypass native approval.
- The parent-session `PostToolUse` receipt only proves invocation of a native Workflow. It does **not** prove the agents' product work passed. Workflow actors/subagents do not necessarily run the parent's hooks.
- Skill auto-triggering can fail if many enabled skills exhaust ZCode's metadata context budget, a skill is disabled, or a session was opened in the wrong directory. Check Settings → Skills and the actual workspace root.
- An existing active workflow may continue across turns; agent must check it and report `WORKFLOW_EXISTING:<verified run ID>` rather than launch duplicate expensive workflows.
- The Stop hook provides **at most two** continuation nudges per detected campaign turn, avoiding infinite response loops. It is a quality-control guard, not a formal proof of successful application execution.
- The permission mode and Goal mode are separate from Workflow execution. `/goal` keeps the long task moving across rounds but does not itself create a multi-lane Workflow.

## Developer tests

```bash
node --test tools/zcode/rimloc-workflow-guard/tests/workflow-guard.test.mjs
```

The test suite uses synthetic temp workspaces and no network, real ZCode session or production files. Run a **real local ZCode hook smoke** after installation before calling this fully verified.

If a hook does not fire, inspect `~/.zcode/cli/log/zcode-<date>.jsonl` for `hook.run.failed` or `config_project_hooks_ignored` and check ZCode plugin enable status; consult ZCode's current Hooks docs. Never share private logs containing session metadata publicly.

## Acceptance checklist

- A new multi-lane RimLoc request launches or resumes **real** ZCode Workflow and gives a tool run receipt.
- A simple one-file change is not over-orchestrated.
- In non-RimLoc workspaces, the plugin stays silent.
- No tool available => exact diagnostic, not a fictitious Workflow.
- Concurrent lanes still use separate worktrees; CI/security/review evidence and owner gates remain.
