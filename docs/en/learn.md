---
title: Learn DefInjected / Keyed
---

# Learn DefInjected / Keyed

RimLoc can discover translatable fields automatically and update dictionaries to reduce manual work.

## Dictionaries

- Defs dictionary (JSON):
```
{
  "ThingDef": ["label", "description", "comps.li.label"],
  "RecipeDef": ["label", "jobString"]
}
```

- Keyed dictionary (JSON):
```
{
  "include": ["^MyMod_.*"],
  "exclude": ["^Debug_.*"]
}
```

## Learn DefInjected

```
rimloc-cli learn-defs --mod ./Mods/MyMod \
  --dict base_defs.json --no-ml \
  --lang English --threshold 0.8 --out out/
```

Outputs:
- `out/missing_keys.json` – [{ defType, defName, fieldPath, confidence, sourceFile }]
- `out/suggested.xml` – DefInjected template with empty values
- `out/learned_defs.json` – audit log with timestamps.

Retrain dictionary:
```
rimloc-cli learn-defs --mod ./Mods/MyMod --dict base_defs.json \
  --no-ml --lang English --threshold 0.8 --out out/ \
  --retrain --retrain-dict base_defs.json
```

## Learn Keyed

```
rimloc-cli learn-keyed --mod ./Mods/MyMod \
  --dict keyed.json --no-ml \
  --source-lang-dir English --lang-dir Russian \
  --threshold 0.8 --out out/
```

Outputs:
- `out/missing_keyed.json` – [{ key, value, confidence, sourceFile }]
- `out/_SuggestedKeyed.xml` – Keyed template with empty values
- `out/learned_keyed.json` – audit log with timestamps.

Retrain keyed dict (append exact keys as regex):
```
rimloc-cli learn-keyed --mod ./Mods/MyMod --dict keyed.json \
  --no-ml --threshold 0.8 --out out/ \
  --retrain-dict keyed.json
```

## REST model contract

POST `{ url }`

Request:
```
{ "def_type": "ThingDef", "def_name": "Beer", "field_path": "label", "value": "beer" }
```
or for Keyed (same fields used, def_type="Keyed", def_name=key).

Response:
```
{ "score": 0.92 }
```

