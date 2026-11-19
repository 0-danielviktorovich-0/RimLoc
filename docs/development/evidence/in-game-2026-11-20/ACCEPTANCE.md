# In-game DefInjected direct read — re-proof on current main — 2026-11-20

Purpose: close the last named in-game gap with a live, hash-verified run —
`read_def` on a **ThingDef** label injected through a RimLoc-built
DefInjected file — and retire a stale claim that travelled through two
documents.

## The doc inconsistency this run settles

- `in-game-2026-10-21/ACCEPTANCE.md` — the bridge's `read_def` was
  generalized to any def database **the same day** (`def_type` arg,
  commit `73296ea6`), and a direct DefInjected proof was recorded
  (ThingDef label `Канареечный маяк` served by the live DefDatabase).
- `in-game-2026-10-23/ACCEPTANCE.md` and
  `docs/development/testing/PRODUCT_ACCEPTANCE_REPORT.md` (2026-10-23)
  repeated the pre-generalization claim: "read_def supports
  ResearchProjectDef only … unchanged named limitation since 2026-10-21".
  That was already false when written.

This run re-proves the DefInjected direct read on the CURRENT main head
(`origin/main 019755ba`, product code `3603e08c` — the SHA of the staged
owner-UAT bundles), against the translation value the product exported on
2026-10-23, with the bridge rebuilt from current main source.

## Setup

- Bridge: rebuilt from `testlab/runtime_bridge` @ `origin/main`
  (content of `73296ea6` — general def-read), dotnet Release,
  `sha256 f94dd66eca348a64f3429ef531f45461bdaf65f5ae403f75e9d3af89a63ad4e6`.
  Installed over the Oct-21 assembly; the previous binary preserved as
  `Assemblies/RimWorldRuntimeBridge.dll.bak-20261120-pre-gendefread`.
- Game: RimWorld **1.6.4871 rev581**, launched background (`open -g`, no
  focus steal) with `-savedatafolder=/tmp/rimloc-rw-lab/SaveData`
  (isolated profile; owner saves/prefs untouched).
- Mod list (fresh `ModsConfig.xml`): Core + 5 DLCs + `rimloc.gamecanary`
  (source canary) + `rimloc.rimlocgamecanary` (the RimLoc-built translation
  mounted 2026-10-23) + `rimloc.qabridge`.
- Language: Russian set before boot via `Config/Prefs.xml`
  `langFolderName` → game booted straight into Russian (state op confirms).
- Translation under test (product export, on disk and mounted):
  `Languages/Russian/DefInjected/ThingDef/RimLocCanaryBeacon.xml` →
  `RimLocCanaryBeacon.label = Маяк проверки владельца`.

## Bridge startup (Player.log)

```
[RimLoc-RuntimeBridge] v1.0.0 up; run_id=run-20261119-181639;
 QA dir=/tmp/rimloc-rw-lab/SaveData/QA; evidence_root=/tmp/rimloc-rw-lab/SaveData/QA;
 dispatch=sync-context; runInBackground=True
```

## LIVE evidence (file-channel responses, committed beside this file)

- `s1.json` (state): `program_state Entry`, `game_version 1.6.4871`,
  `active_language: Russian (Русский)` — **the game runs in Russian**;
  active mods exactly the minimal list above.
- `k1.json` (keyed, control regression of the 10-23 proof):
  `RimLocCanaryGreeting → Привет, канареечный мир`.
- `d1.json` (read_def, **the proof**):

```json
{"def":"RimLocCanaryBeacon","def_type":"ThingDef",
 "label":"Маяк проверки владельца",
 "description":"A game-valid canary building whose label is translated by the RimLoc export."}
```

The live game's **DefDatabase** serves the DefInjected translation from the
RimLoc export: label is the Russian string injected by the product's
DefInjected file, while the untranslated field (`description`) passes
through as the def's own English text — the correct per-field behaviour.

- `q1.json` (lifecycle): quit accepted; game exited cleanly.

## Integrity & errors

- Canary source md5 before == after → **SOURCE INTACT** (the product never
  writes into the game's mod folder).
- Player.log: **0** lines mentioning `RimLocCanaryTranslation` errors and
  **0** lines mentioning `RimLocGameCanary` errors (excerpt file beside this
  document). The two `Could not load Texture2D` warnings are the canary
  fixture's placeholder `texPath` (borrowed from Core paths that moved) —
  the same known fixture artifact recorded on 2026-10-23, not a translation
  defect.

## Verdict

**DefInjected direct in-game read PROVEN on the current main head** with a
hash-verified bridge build: the real game's DefDatabase serves the
DefInjected label injected by the RimLoc export. Combined with the 10-23
Keyed proof on the same RC source, every in-game localization channel the
product writes (Keyed + DefInjected) now has a live programmatic proof.

The "read_def supports ResearchProjectDef only" claim in the 2026-10-23
report and 10-23 acceptance note is superseded by this run and by the
2026-10-21 same-day generalization; both documents remain as
point-in-time records per the archive convention.

Owner-UAT relevance: this closes the third honest caveat listed in
`PRODUCT_ACCEPTANCE_REPORT.md → REAL_RIMWORLD_KEYED_DEF` (and the matching
`openProductGaps` entry "in-game RimWorld acceptance proven on an earlier
source SHA; rerun if export path changes materially" — product source is
unchanged since `3603e08c`, and the DefInjected channel is now directly
proven on it).
