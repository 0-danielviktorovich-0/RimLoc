# RimWorld autonomous in-game lab (Issue #9, lane A+B harness)

Background-safe acceptance of RimLoc translations inside the REAL game
on this Mac. One command per milestone; the game runs in an ISOLATED
`-savedatafolder` profile, never touches the owner's saves/prefs, and
never steals focus (verified empirically: frontmost app unchanged
across launches and screenshots).

## Components

- `gabs_mcp.py` — minimal MCP stdio client (MCP 2026-07-28 shape,
  stateless `_meta` per request) for the local GABS game-bridge server.
  Auto-connects the running game, lists 128 mirrored `rimworld_*` tools,
  fuzzy-matches names, prints structured results.
- GABS v1.1.2 (darwin-arm64) at `~/.gabs-tools/`, config `~/.gabs/config.json`:
  game `rimworld`, DirectPath to `/Applications/RimWorld.app/Contents/MacOS/…`,
  args `-savedatafolder=/tmp/rimloc-rw-lab/SaveData`.
- RimBridgeServer v2.3.0 (MIT, pardeike) installed at
  `/Applications/RimWorld.app/Mods/RimBridgeServer`, enabled ONLY in the
  ISOLATED profile's ModsConfig (Core+DLC+Harmony+bridge — additive
  install, the owner's real mod list is untouched).
- `evidence/` — local-only screenshots (gitignored).

## A1 — autonomous language acceptance (PROVEN 2026-10-20)

```
gabs games start rimworld        # isolated profile, background
python3 gabs_mcp.py call games_connect '{"gameId":"rimworld"}'
python3 gabs_mcp.py call games_call_tool '{"gameId":"rimworld","tool":"rimworld_rimworld_list_languages","arguments":{}}'
# → 29 languages, active identified (semantic state, not pixels)
python3 gabs_mcp.py call games_call_tool '…switch_language… {"language":"English"}'   # pref-correct baseline
python3 gabs_mcp.py call games_call_tool '…switch_language… {"language":"Russian"}'  # → "Queued … switch" (REAL switch)
# wait long event idle → take_screenshot → RU main menu rendered
python3 gabs_mcp.py call games_call_tool '…take_screenshot… {"fileName":"a1_menu_ru_APPLIED","outputDirectory":"/tmp/rimloc-rw-lab/shots"}'
gabs games stop rimworld
```

Result: main menu fully Russian (`СОЗДАТЬ МИР / НОВАЯ КОЛОНИЯ / ОПЦИИ /
МОДЫ…`), captured while another app held focus. `Player.log` records
`Translation data for language Russian / Русский` — zero
`No active language!` lines.

**Root cause of the old blocker found**: `defaults write
ludeon.rimworld language Russian` only sets the PREF; the in-game
UI switch never ran, and with a patched mod list the game didn't pick
the pref up at boot either. The bridge's `switch_language` performs the
actual picker-level switch (queued long event) — the "30-second owner
action" is obsolete: the agent switches languages itself.

## Notes

- `-savedatafolder` isolates Saves/Config/ModsConfig; the language PREF
  lives outside it, which is why the bridge tool (game-native switch) is
  the correct mechanism — and why the pref-only approach of the 2026-10-19
  runs was unreliable.
- The tool name normalization is `rimworld_rimworld_<name>` (GABS
  namespace + mod namespace); use `games_tool_names` with cursors to
  discover, `games_tool_detail` for schemas.
- `switch_language` answers "already configured" from the pref alone —
  force a real switch with an EN→RU cycle when the rendered UI matters.
- Game version 1.6.4871; GABS 1.1.2; RimBridgeServer 2.3.0 (requires
  Harmony 2.4.2 — already present in the owner's Mods root).
