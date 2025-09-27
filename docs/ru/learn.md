---
title: Learn DefInjected / Keyed
---

# Learn DefInjected / Keyed

RimLoc умеет автоматически искать переводимые поля и дополнять словари.

## Словари

- Defs (JSON):
```
{
  "ThingDef": ["label", "description", "comps.li.label"],
  "RecipeDef": ["label", "jobString"]
}
```

- Keyed (JSON):
```
{
  "include": ["^MyMod_.*"],
  "exclude": ["^Debug_.*"]
}
```

## Обучение DefInjected

```
rimloc-cli learn-defs --mod ./Mods/MyMod \
  --dict base_defs.json --no-ml \
  --lang English --threshold 0.8 --out out/
```

Результаты:
- `out/missing_keys.json` – [{ defType, defName, fieldPath, confidence, sourceFile }]
- `out/suggested.xml` – шаблон DefInjected с пустыми значениями
- `out/learned_defs.json` – журнал (с метками времени)

Пополнение словаря:
```
rimloc-cli learn-defs --mod ./Mods/MyMod --dict base_defs.json \
  --no-ml --lang English --threshold 0.8 --out out/ \
  --retrain --retrain-dict base_defs.json
```

## Обучение Keyed

```
rimloc-cli learn-keyed --mod ./Mods/MyMod \
  --dict keyed.json --no-ml \
  --source-lang-dir English --lang-dir Russian \
  --threshold 0.8 --out out/
```

Результаты:
- `out/missing_keyed.json` – [{ key, value, confidence, sourceFile }]
- `out/_SuggestedKeyed.xml` – шаблон Keyed
- `out/learned_keyed.json` – журнал (с метками времени)

Пополнение словаря для Keyed (добавляются точные ключи как regex `^key$`):
```
rimloc-cli learn-keyed --mod ./Mods/MyMod --dict keyed.json \
  --no-ml --threshold 0.8 --out out/ \
  --retrain-dict keyed.json
```

## REST модель

POST `{ url }`

Запрос:
```
{ "def_type": "ThingDef", "def_name": "Beer", "field_path": "label", "value": "beer" }
```
или для Keyed (def_type="Keyed", def_name=имя ключа).

Ответ:
```
{ "score": 0.92 }
```

