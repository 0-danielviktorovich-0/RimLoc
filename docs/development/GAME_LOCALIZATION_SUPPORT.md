# GAME LOCALIZATION SUPPORT — RimLoc как движок всей игры (не только модов)

Дата: 2026-09-23 · Итерация 2 · Мандат: LocalizationSource = Mod | BaseGame | DLC | LanguagePack.

## Аудит архитектуры: что уже source-агностично

Ядро RimLoc оперирует деревом «корень + Defs/ + Languages/<язык>/» — **Data-корни игры это то же дерево**:

| LocalizationSource | Корень | Работает сегодня? |
|---|---|---|
| Mod | папка мода (About + Defs + Languages) | ✓ основной сценарий |
| BaseGame | `RimWorld.app/Data/Core` | ✓ доказано: `scan --root Data/Core --lang en` = **11549 записей** |
| DLC | `RimWorld.app/Data/{Royalty,Ideology,Biotech,Anomaly,Odyssey}` | ✓ та же механика (не прогонялась — помечено NOT TESTED) |
| LanguagePack | распакованный тар официального пака | ✓ доказано: официальный RU Core = **16681 записей, 93% пересечения ключей** с EN-извлечением |

Общий конвейер (extraction/diff/TM/glossary/translation/validation/compare/reporting) не дублируется — всё работает от `--root`.

## Внесённые адаптеры (малой ценой)

1. **version-resolver без About.xml**: game-Data-корни не объявляют версий → плоский fallback без ошибки (было: hard error при настроенном `game_version`).
2. **Распаковка пака = адаптер LanguagePack**: тар `Russian (Русский).tar` распаковывается ВНУТРЬ `Languages/<Язык>/` (содержимое тара — уже уровень языка). После этого весь пайплайн (scan/coverage/validate/compare/translate) работает над официальным паком как над модом.

## Проверено на контролируемых данных игры (read-only, Data не изменён)

- EN-извлечение Core: 11549 ключей (Defs + Languages/English).
- Официальный RU: 16681 ключей; 93% совпадение по ключам с EN-извлечением; расхождение = Strings/Backstories/WordInfo, не покрываемые EN-сканом Defs (категории раздельного учёта — см. слепой бенчмарк §8).

## Translation Maintainer workflow (пак + новая версия игры)

Уже существующие механизмы покрывают каркас:

| Шаг воркфлоу | Механизм RimLoc |
|---|---|
| новая версия игры → что изменилось | `rimloc version-diff` (сейчас по папкам мода; для игры — распакованные Defs двух версий) |
| reusable translations | version-diff: unchanged + TM-prefill `export-po --tm-root` |
| sourceChanged | version-diff: changed (review-очередь) |
| new / obsolete | version-diff: new / removed |
| валидация | `validate` + (todo) категории Translation Report |
| WordInfo | `rimloc word-info` (диагностика + каркас) |
| экспорт валидного пака | `import-po` / `build-mod` |

## Остаточная работа (за пределами текущего RC, не блокирует)

1. **Tar-адаптер чтения** (`Languages/<lang>.tar` in-place) — сейчас распаковка руками в контролируемый каталог.
2. **Strings/Backstories в EN-извлечении** для полной симметрии с официальными паками.
3. **NoTranslate/TKey-атрибуты** (P2 из RIMWORLD_REFERENCE_AUDIT).
4. **Translation Report-дифф** как автоматическая сверка (нужен прогон игры с генерацией отчёта — Desktop-путь из §9 research).
5. **DLC-прогоны** (механика та же; помечено NOT TESTED).
