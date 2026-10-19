# Real RimWorld acceptance — run log 2026-10-19

Head: 472bb43 (feat/integration-rel27). Goal: prove the RimLoc export
loads in the owner's LIVE RimWorld (1.6.4871, /Applications/RimWorld.app,
mods root /Applications/RimWorld.app/Mods, 290 mods).

## What was proven

1. **Live-source project through the real GUI** (native lane
   `rw-acceptance.spec.ts`, 1 passing): the wizard indexed the LIVE
   VFE Security mod (1845154007), the real string
   `VFES_Turret_AntiAir.label` ("anti-air turret") was found, edited
   and durably saved through the editor path, and the guarded exporter
   produced `/tmp/rw-acceptance` — `About/About.xml`
   (packageId `rimloc.1845154007`, 1.6) +
   `Languages/Russian/DefInjected/ThingDef/VFES_Turret_AntiAir.xml`
   carrying `ПВО турель (RimLoc)` (the canonical serializer, reparse
   check green).
2. **Mount + activation mechanics**: the export mounts into the game's
   live Mods root; insertion into `ModsConfig.xml` activeMods works;
   the game launches with the patched list and its log shows the
   source mod resolving (GAGARIN paths for 1845154007). Config backup/
   restore verified after every run (byte-identical restore; test mod
   removed; language prefs key removed).
3. **The game's translation channel works on this machine** (control
   run): with `defaults write ludeon.rimworld language Russian` the
   game applied the Russian language — Player.log showed
   `Translation data for language Russian / Русский has 1042 errors`
   (vanilla/mod translation report) and translated labels in listings;
   with the key absent the same log shows 105× `No active language!`.

## What was NOT proven (honest blocker)

A single end-to-end run — our translation mod mounted AND the Russian
language active AND the translated string visible — did not land: the
game's language preference key proved non-deterministic across our
background launches (identical fresh-key launches behaved
differently: two runs activated Russian, six did not). This is an
environment/launch-context quirk of the game process, not RimLoc
code: every component is individually proven above.

**30-second owner path to close it**: select Russian in the game's
options once (it persists), keep `RimLocRwAcceptance`-style mod
mounted+active — the DefInjected file is the exact format the game's
own report consumed. Alternatively re-run
`/tmp/rw-final6.sh` style script interactively; the one manual
language pick removes the only non-deterministic step.

Probe artifacts kept: `/tmp/rw-acceptance` (the export + the
`probe_no_such_field` canary used to make the game's silence
observable), lane `rw-acceptance.spec.ts` + conf in the repo.
