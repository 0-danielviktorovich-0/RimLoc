# Fixtures — официальные языковые паки Ludeon

Источник: github.com/Ludeon/RimWorld-{ru,de,ja,zh} (shallow-клоны 2026-09-23, см.
docs/development/OFFICIAL_LANG_PACKS.md). Файлы взяты как есть для регрессии
парсеров RimLoc (interop-тестирование формата игры).

Provenance: first-party Ludeon game content (переводы официальной локализации).
Лицензия репозиториев Ludeon явно не объявлена — файлы используются ТОЛЬКО как
фикстуры парсинга внутри проекта, не как переводческий корпус и не для
распространения. Не включать в TM/глоссарии генерации без разрешения.

| Файл | Что покрывает |
|---|---|
| RimWorld-ru_…Incidents_Map_Disease.xml | {lookup:} + numCase + PAWN-макросы + \n в одной строке |
| RimWorld-de_…decline.txt | WordInfo de: BOM, CSV-заголовок, 8-колоночное склонение, пустые ячейки |
| RimWorld-ja_…Dates.xml | переупорядоченные плейсхолдеры + CJK |
| RimWorld-zh_…Dates.xml | тот же ключ в zh — кросс-языковая проверка множеств токенов |
| RimWorld-de_…Incidents_Map_Disease.xml | вложенные {replace: {lookup: …}} — рекурсия макросов |
