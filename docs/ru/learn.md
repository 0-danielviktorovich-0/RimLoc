---
title: Learn DefInjected / Keyed
---

# Learn DefInjected / Keyed

Разбор “на пальцах”, как автоматически искать строки, пополнять словари и генерировать шаблоны. Подходит новичкам — можно копировать команды и подставить свой путь к моду.

Что это даёт
- Находит переводимые поля в Defs (label/description/вложенные элементы; списки <li> и <LineBreak/> объединяются автоматически) и ключи в Keyed.
- Сохраняет “выученные” наборы для ревью.
- По желанию — дополняет словари автоматически.
- Генерирует пустые шаблоны XML для переводчиков.

Перед началом
- Поставьте rimloc-cli и укажите корень мода (где есть Defs/ и/или Languages/...).
- Держите словари Defs и Keyed отдельно — у них разные форматы и задачи.

## Словари (кратко)

Defs dictionary (JSON)
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
Подсказки:
- Для списков используйте `li` (например, comps.li.label).
- Храните словари в репозитории (например, ./dicts/defs.json).

Keyed dictionary (JSON)
```
{
  "include": ["^MyMod_.*"],
  "exclude": ["^Debug_.*"]
}
```
Подсказки:
- Начните с широкого include (например, ^MyMod_) и при необходимости добавляйте exclude.
- Можно указывать несколько файлов словаря (по частям проекта).

## Типичный сценарий

1) Scan/Validate/Diff c учётом словарей (Defs)
- Скан JSON для быстрого просмотра:
```
rimloc-cli --quiet scan --root ./mods/MyMod --format json \
  --defs-dict ./dicts/defs.json --defs-field labelFemale,title
```
- Проверка (пустые/дубли/плейсхолдеры):
```
rimloc-cli validate --root ./mods/MyMod --format json --defs-dict ./dicts/defs.json
```
- Diff исходник/перевод (с учётом словаря):
```
rimloc-cli diff-xml --root ./mods/MyMod --format json \
  --defs-dict ./dicts/defs.json --defs-dir 1.6/Defs
```

2) Обучение DefInjected (поиск + шаблон)
```
rimloc-cli learn-defs --mod ./mods/MyMod \
  --dict ./dicts/defs.json --no-ml \
  --lang English --threshold 0.8 --out ./out \
  --learned-out ./out/learned_defs.json
```
Результаты:
- `out/missing_keys.json` – список новых ключей [{ defType, defName, fieldPath, confidence, sourceFile }]
- `out/suggested.xml` – шаблон DefInjected (с комментариями `<!-- EN: ... -->` с оригиналами)
- `out/learned_defs.json` – журнал с датами

Авто‑пополнение словаря:
```
rimloc-cli learn-defs --mod ./mods/MyMod \
  --dict ./dicts/defs.json --no-ml \
  --lang English --threshold 0.8 --out ./out \
  --retrain --retrain-dict ./dicts/defs.json
```
Если `--retrain-dict` не указан, файл `<имя>.updated.json` появится рядом с первым словарём или в `out/` (резервно).

3) Обучение Keyed (поиск + шаблон)
```
rimloc-cli learn-keyed --mod ./mods/MyMod \
  --dict ./dicts/keyed.json --no-ml \
  --source-lang-dir English --lang-dir Russian \
  --threshold 0.8 --out ./out \
  --learned-out ./out/learned_keyed.json
```
Результаты:
- `out/missing_keyed.json` – [{ key, value, confidence, sourceFile }]
- `out/_SuggestedKeyed.xml` – шаблон Keyed (также с `<!-- EN: ... -->`)
- `out/learned_keyed.json` – журнал с датами

Пополнение словаря Keyed (добавляет точные ключи как regex `^key$`):
```
rimloc-cli learn-keyed --mod ./mods/MyMod \
  --dict ./dicts/keyed.json --no-ml \
  --threshold 0.8 --out ./out \
  --retrain-dict ./dicts/keyed.json
```

## ML оценка (опционально)
Если есть модель‑сервис, включите её для более аккуратной фильтрации.

REST: POST `{ url }`

Запрос (Defs):
```
{ "def_type": "ThingDef", "def_name": "Beer", "field_path": "label", "value": "beer" }
```
Для Keyed: `def_type="Keyed"`, `def_name=имя_ключа`.

Ответ:
```
{ "score": 0.92 }
```

Запуск с ML:
```
rimloc-cli learn-defs  --mod ./mods/MyMod --dict ./dicts/defs.json --ml-url http://127.0.0.1:8080/score --threshold 0.85 --out ./out
rimloc-cli learn-keyed --mod ./mods/MyMod --dict ./dicts/keyed.json --ml-url http://127.0.0.1:8080/score --threshold 0.85 --out ./out
```

## Советы
- Держите Defs и Keyed словари отдельно.
- Начните с `--no-ml` и порога 0.8; потом настройте под проект.
- Для списков в Defs используйте `li` (например, comps.li.label). Можно использовать `li{h}` — тогда RimLoc постарается именовать элементы списка по «ручкам» (Class/defName/label) вместо индексов; повторы будут как foo, foo-1. Также возможны алиасы в шаге пути: `a|b`. (например, comps.li.label).
- Логи обучения (`learned_*.json`) удобно хранить в репозитории для ревью.
