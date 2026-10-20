# RimLoc QA Bridge (standalone)

A file-channel runtime bridge for automated RimWorld 1.6 QA: a RimWorld
mod assembly that watches a `QA/` directory inside the ACTIVE save-data
folder for JSON commands and writes JSON results — no network listener,
no foreground takeover (`Application.runInBackground = true`), no
proprietary game files shipped.

This is TEST TOOLING, not gameplay content. It exists so agents and CI
can observe live game state (localization, defs, frames) inside an
isolated save profile.

## Install (from a staged package)

1. Unzip so `RimLocQABridge/` lands in your RimWorld `Mods/` folder.
2. Enable "RimLoc QA Bridge" in the mod list (needs Harmony only if a
   companion uses it; the bridge itself has no mod dependencies).
3. Launch the game with `-savedatafolder=<isolated profile>` for QA
   work — the bridge keys its channel to that profile's save data.
4. Drop a command into `<savedata>/QA/commands/<id>.json`:

```json
{"protocol_version":1,"request_id":"c1","run_id":"<run id from the game log>","op":"capabilities","args":{}}
```

5. Read `<savedata>/QA/results/<id>.json`.

The game log prints the bridge's `run_id` at startup; commands with a
different `run_id` are rejected (`RUN_ID_MISMATCH`). Keep that id per
session.

## Build from source

Requires `dotnet` and a licensed RimWorld install:

```bash
GAME_MANAGED=/path/to/RimWorld.app/Contents/Resources/Data/Managed \
  bash build.sh --package out/
```

Never commit or distribute the game's DLLs; the build references them
in place only.

## Protocol v1 — operations

`capabilities`, `state`, `keyed {key}`, `research_tab {tab}`,
`screenshot {path}`, `frame_freshness {path}`, `quit`. Full contract:
`capability-manifest.json` (generated from the source; the source is
the one authority).

License: GPL-3.0 (this package). RimWorld is a trademark of Ludeon
Studios; this project is not affiliated with or endorsed by them.
