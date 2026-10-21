# Host matrix (tested rows only)

| Host | Skill install | MCP stdio | Notes |
|---|---|---|---|
| Codex CLI 0.153.4 | n/a (no skill discovery; the scenario is prompt-driven) | **TESTED 2026-10-21** — full scenario LIVE: `codex exec --sandbox workspace-write -c 'mcp_servers.rimloc.command="<rimloc-mcp>"' -c 'mcp_servers.rimloc.env.RIMLOC_DATA_DIR="<profile>"'` against rimloc-mcp 0.2.0 (`7867bb5`); 8 steps executed by the agent itself (list → open → snapshot → search → propose → apply → validate → export.build), applied:1 revision:4, validation succeeded, translation verified in the built mod XML; a malformed propose got a typed `-32602` and the server stayed alive (agent self-corrected). Run summary: `{"project_id":"proj-01a15ca4c304-000-97db","applied":1,"revision_after":4,"validation_status":"succeeded","export_path":"/tmp/rimloc-a3/export-codex"}`, 25 721 tokens | full TESTED |
| ZCode 0.16.9 (project scope) | **TESTED 2026-10-21 (discovery)** — `cp -R skills/rimloc .zcode/skills/rimloc` then `zcode skills list` discovers `rimloc (project/zcode)` with the full routing description | same server binary; the stdio path is host-agnostic (proven via Codex above) | discovery TESTED; the full interactive TUI scenario needs a human-keyboard session and is not claimed |

No row is "universal": untested hosts are NOT claimed. Update this
file with dated evidence per host when tested. The rimloc MCP server
itself is proven live end-to-end (issue #10 A3, 2026-10-21) — see
`docs/development/evidence/`.
