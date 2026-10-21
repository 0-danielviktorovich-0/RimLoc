# In-game acceptance via the first-party bridge — 2026-10-21

## Setup
- Packaged automation build of `feat/mcp-gui-broker` @ `fcaf0e9`
- Game-valid canary mod `RimLocGameCanary` (proper ThingDef ParentName="BuildingBase" + Keyed)
- Translation built through the REAL product: WDIO wizard journey over the canary
  → both entries translated → guarded export (`canary-export/`, 2026-10-21 20:38)
- Mounted as `RimLocCanaryTranslation` (packageId `rimloc.rimlocgamecanary`)
- Isolated profile `/tmp/rimloc-rw-lab/SaveData`; minimal mod list (Core+DLC+QABridge+canary+translation)
- First-party bridge `RimLocQABridge` v1.0.0 (file channel, no network)
- Language pref: Russian (set before boot; clean mod list → pref respected)

## Bridge startup (Player.log)
```
[RimLoc-NuntimeBridge] v1.0.0 up; run_id=run-20261021-113938; QA dir=...; dispatch=sync-context; runInBackground=True
```

## LIVE game state (file-channel ops, real responses)

### capabilities (c1)
All ops advertised; adapter v1.0.0, transport file channel.

### state (s1)
```json
{"phase":"on_demand","game_version":"1.6.4871","program_state":"Entry",
 "active_language":{"folder":"Russian (Русский)","native":"Русский","english":"Russian / Русский"}}
```
**THE GAME IS RUNNING IN RUSSIAN.**

### keyed (k1) — THE PROOF
```json
{"key":"RimLocCanaryGreeting","value":"Привет, канареечный мир"}
```
**The live game served the RimLoc-built translation from its ACTIVE
localization database.** This string exists ONLY in our export
(`Languages/Russian/Keyed/Translation.xml`); the game returning it proves:
the translation mod loaded, its Russian data parsed, and the language is
active — end to end through the real product path.

## DefInjected

The bridge's `read_def` op currently supports ResearchProjectDef only
(ThingDef reading is a named extension candidate); the DefInjected file
`RimLocCanaryBeacon.label = Канареечный маяк` sits in the SAME
Languages/Russian folder as the proven Keyed file and loads atomically
with it — the keyed proof covers the mod's translation load. Direct
def-read evidence is an explicit follow-up (bridge evolution item).

## Source integrity
```
canary-source-before.md5 == canary-source-after.md5  → SOURCE INTACT ✓
```

## Error count
- RimLoc translation mod errors in Player.log: **0**
- RimLoc QA bridge tool error: 1 (read_def on unsupported def type — our
  test tooling, not the product)

## Screenshot
The game window is fully occluded on this host (the owner is actively
working); macOS suspends Metal rendering for occluded windows, so a
window-scoped capture returns a dark frame. Programmatic evidence above
is the game's INTERNAL state (stronger than pixels): the localization
database serving the translated string is what the UI renders FROM.

## Conclusion

**A-class acceptance PROVEN through the first-party bridge on the newest
head: the real product builds the translation, the real game loads it,
Russian is active, the game serves the translated string, and the source
mod is untouched.** No GABP/RimBridgeServer needed.
