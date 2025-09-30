---
title: Learn DefInjected / Keyed
---

# Learn DefInjected / Keyed

This page explains how to use RimLoc to discover missing strings, keep dictionaries up to date, and auto‑generate templates. It is written for beginners: you can copy commands as‑is and tweak later.

What you get
- Find all translatable fields from Defs (labels, descriptions, nested items) and Keyed (by key name). RimLoc also merges list items (<li>) and <LineBreak/> fragments into a single string automatically.
- Save “learned” datasets for auditing and team review.
- Update your dictionaries automatically (optional).
- Generate ready‑to‑fill XML templates for translators.

Before you start
- Install rimloc-cli and point to a RimWorld mod folder (the root that contains Defs/ and/or Languages/...). If your mods live under `./mods`, replace paths below accordingly.
- Keep Defs and Keyed dictionaries separate (they have different formats and purposes).

## Dictionaries (quick reference)

Defs dictionary (JSON)
- Maps a Def type to a list of dot‑paths (case-insensitive tag names):
```
{
  "ThingDef": [
    "label",
    "description",
    "comps.li.label",
    "tools.li.label"
  ],
  "RecipeDef": ["label", "jobString"]
}
```
Tips:
- Use `li` for items inside lists (e.g., comps.li.label).
- Use `li{h}` to prefer human‑readable handles in lists (e.g., `comps.li{h}.label`) — RimLoc derives a handle from `Class`/`defName`/`label` and de‑duplicates as `foo`, `foo-1`, …
- Use aliases with `|` to match alternative tag names in a single step, e.g. `degreeDatas.li{h}.label|labelShort`.
- Put your dictionaries into your repo (e.g., ./dicts/defs.json).

Keyed dictionary (JSON)
- Controls which Keyed names to include/exclude via regular expressions:
```
{
  "include": ["^MyMod_.*"],
  "exclude": ["^Debug_.*"]
}
```
Tips:
- Start with a broad include (e.g., ^MyMod_) and add excludes as needed.
- You can maintain several dict files (per module or feature) and pass multiple paths.
- Optional: treat nested elements under `<LanguageData>` as dot‑keys (add `--keyed-nested`). By default keys remain flat (`<Key>value</Key>`).

## Typical workflow

1) Scan/Validate/Diff with dictionaries (Defs)
- Scan JSON for quick inspection:
```
rimloc-cli --quiet scan --root ./mods/MyMod --format json \
  --defs-dict ./dicts/defs.json --defs-field labelFemale,title
```
- Validate (reports empty/duplicate/placeholders):
```
rimloc-cli validate --root ./mods/MyMod --format json --defs-dict ./dicts/defs.json
```
- Diff English vs target language (with dict; suggests missing keys):
```
rimloc-cli diff-xml --root ./mods/MyMod --format json \
  --defs-dict ./dicts/defs.json --defs-dir 1.6/Defs
```

2) Learn DefInjected (auto‑discover + template)
```
rimloc-cli learn-defs --mod ./mods/MyMod \
  --dict ./dicts/defs.json --no-ml \
  --lang English --threshold 0.8 --out ./out \
  --learned-out ./out/learned_defs.json
```
Outputs:
- `out/missing_keys.json` – list of new keys to translate [{ defType, defName, fieldPath, confidence, sourceFile }]
- `out/suggested.xml` – DefInjected template with empty values (RimLoc includes `<!-- EN: ... -->` comments with originals)
- `out/learned_defs.json` – audit log with timestamps

Update the dictionary automatically (append learned fields):
```
rimloc-cli learn-defs --mod ./mods/MyMod \
  --dict ./dicts/defs.json --no-ml \
  --lang English --threshold 0.8 --out ./out \
  --retrain --retrain-dict ./dicts/defs.json
```
If `--retrain-dict` is omitted, RimLoc writes `<name>.updated.json` near your first dict, or into `out/` as a fallback.

3) Learn Keyed (auto‑discover + template)
```
rimloc-cli learn-keyed --mod ./mods/MyMod \
  --dict ./dicts/keyed.json --no-ml \
  --source-lang-dir English --lang-dir Russian \
  --threshold 0.8 --out ./out \
  --learned-out ./out/learned_keyed.json
```
Outputs:
- `out/missing_keyed.json` – list of missing Keyed entries [{ key, value, confidence, sourceFile }]
- `out/_SuggestedKeyed.xml` – Keyed template with empty values (also annotated with `<!-- EN: ... -->`)
- `out/learned_keyed.json` – audit log with timestamps

Update the Keyed dictionary (append exact keys as regex `^key$`):
```
rimloc-cli learn-keyed --mod ./mods/MyMod \
  --dict ./dicts/keyed.json --no-ml \
  --threshold 0.8 --out ./out \
  --retrain-dict ./dicts/keyed.json
```

## ML scoring (optional)
If you have a classifier service, you can enable it for more precise filtering.

REST endpoint: POST `{url}`

Request (Defs):
```
{ "def_type": "ThingDef", "def_name": "Beer", "field_path": "label", "value": "beer" }
```
Keyed uses the same shape, with `def_type: "Keyed"` and `def_name: key`.

Response:
```
{ "score": 0.92 }
```

Use with:
```
rimloc-cli learn-defs  --mod ./mods/MyMod --dict ./dicts/defs.json --ml-url http://127.0.0.1:8080/score --threshold 0.85 --out ./out
rimloc-cli learn-keyed --mod ./mods/MyMod --dict ./dicts/keyed.json --ml-url http://127.0.0.1:8080/score --threshold 0.85 --out ./out
```

## Tips & pitfalls
- Keep Defs and Keyed dictionaries separate — they solve different problems.
- Start with `--no-ml` and a higher `--threshold` (e.g., 0.8), then tune.
- Use `li` for list items in Defs dot‑paths (e.g., comps.li.label).
- Save outputs (`out/…`) in VCS to help reviewers.
- For Windows paths in dicts use forward slashes or relative paths.
