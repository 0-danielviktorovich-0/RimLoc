# In-game RimWorld re-acceptance — RC build of merged main — 2026-10-23

Purpose: re-prove the real-game acceptance on the CURRENT release-candidate
source (merged main `8610e313` — the SHA whose CI bundles are the staged RC),
closing the "proven only on an earlier SHA" gap.

## Setup
- Product: automation artifact rebuilt from merged main `8610e313` (identical
  source tree to the staged RC bundles; the RC macOS bundle itself already
  passed its launch smoke).
- Translation built through the REAL product: WDIO wizard over the GAME'S OWN
  canary mod (`/Applications/RimWorld.app/Mods/RimLocGameCanary`, read-only
  source), both entries translated, guarded export to `rc-export/`.
- Export verified on disk BEFORE mounting:
  - `Languages/Russian/Keyed/Translation.xml`:
    `RimLocCanaryGreeting = Привет, канареечный мир`
  - `Languages/Russian/DefInjected/ThingDef/RimLocCanaryBeacon.xml`:
    `RimLocCanaryBeacon.label = Маяк проверки владельца`
- Mounted as `Mods/RimLocCanaryTranslation` (packageId `rimloc.rimlocgamecanary`
  unchanged; previous translation preserved in `staged-prev-translation`).
- Game: RimWorld 1.6.4871 launched background via GABS DirectPath with
  `-savedatafolder=/tmp/rimloc-rw-lab/SaveData` (isolated profile; owner's
  saves/prefs untouched; no focus steal). First-party file-channel bridge
  `RimLocQABridge` v1.0.0 active (GABP remains non-binding on this host —
  known; the file channel needed none of it).

## Bridge startup (Player.log)
`[RimLoc-RuntimeBridge] v1.0.0 up; run_id=run-20261022-211252;
 QA dir=/tmp/rimloc-rw-lab/SaveData/QA; dispatch=sync-context; runInBackground=True`

## LIVE game state (file-channel responses — `rcs1.json`)
`active_language: Russian (Русский)` — **the game is RUNNING IN RUSSIAN**
(program_state Entry, game_version 1.6.4871).

## THE PROOF (`rck1.json`)
```json
{"op":"keyed","args":{"key":"RimLocCanaryGreeting"},
 "result":{"key":"RimLocCanaryGreeting","value":"Привет, канареечный мир"}}
```
The live game's ACTIVE localization database served the string that exists
ONLY in today's RC-product export — the mod loaded, its Russian data parsed,
and the language is active, end to end through the real product path.

## DefInjected
`RimLocCanaryBeacon.label = Маяк проверки владельца` rides in the SAME
`Languages/Russian` folder as the proven Keyed file (atomic load); the
bridge's `read_def` supports ResearchProjectDef only (unchanged named
limitation since 2026-10-21), so the keyed proof covers the translation
load and the DefInjected file is verified on disk.

## Integrity & errors
- Source mod md5 before == after: **SOURCE INTACT** (Defs + Keyed unchanged;
  the product never wrote into the game's mod folder).
- `RimLocCanaryTranslation` errors in Player.log: **0** (the "9 errors" line
  belongs to the game's own Core/DLC Russian data, zero mentions of our mod).
- Texture warning for the canary's placeholder texture is the canary fixture
  itself (texPath borrowed from Core), not a translation defect.

## Verdict
**IN-GAME RE-ACCEPTANCE PROVEN ON THE RC SOURCE (merged main 8610e313):**
the RC product builds the translation, the real game loads it, Russian is
active, and the live localization database serves the RC-built string.
