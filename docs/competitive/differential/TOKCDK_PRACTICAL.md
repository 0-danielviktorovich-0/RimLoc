# TokcDK/RimworldModTranslator — практический прогон (волна 3, lane tocdk)

Дата: 2026-10-06 · Автор: бенчмаркер lane tocdk · Статус прогона: **PRACTICALLY_RUN** (ядро headless на macOS), GUI: **BLOCKED_PLATFORM**

## 1. Идентификация

- Репозиторий: `https://github.com/TokcDK/RimworldModTranslator`, клон в `/tmp/w6-tocdk/tocdk`
- **Frozen SHA: `7752a0d6d106a3b7ad4dbb975ef6342349bfb9ee`** (последний коммит 2025-04-27, «set version using get versioning») — подтверждает «спит с 2025-04»
- Лицензия GPL-3.0, решение `RimworldModTranslator.sln`, **один проект** `RimworldModTranslator.csproj`, 51 файл `.cs`

## 2. WPF или переносимое ядро? (п.1 задачи)

`RimworldModTranslator.csproj:3-10`:

```xml
<OutputType>WinExe</OutputType>
<TargetFramework>net8.0-windows</TargetFramework>
<EnableWindowsTargeting>true</EnableWindowsTargeting>
<UseWPF>true</UseWPF>
```

- Отдельной библиотеки ядра (netstandard/net6+) **нет** — только WinExe. Формально `net*-generic`-таргета тоже нет, т.е. по букве правила — BLOCKED_PLATFORM.
- Но `EnableWindowsTargeting=true` (csproj:6) — потому что автор сам разрабатывал на Rider под не-Windows? — позволил **скомпилировать весь WPF-проект на macOS**: `dotnet build -c Release -p:EnableWindowsTargeting=true` → **0 ошибок, 61 warning** (dotnet SDK 10.0.401, arm64). Это компиляция, не запуск: WPF-рантайм на macOS отсутствует, GUI не стартует (BLOCKED_PLATFORM для рантайма).
- Ядро при этом **фактически изолировано в одном классе** `Helpers/EditorHelper.cs` (1722 строки): чтение/извлечение/запись не трогают WPF-типы. WPF используется только в methods для DataGrid (ClearSelectedCells и т.п., EditorHelper.cs:1277-1339), которые headless-прогон не вызывает.

**Вывод: GUI = BLOCKED_PLATFORM; ядро = PRACTICALLY_RUN** через reflection-харнесс поверх скомпилированной `RimworldModTranslator.dll` (прецедент Text Grabber lane).

## 3. Как прогонялся (п.2 задачи)

Харнесс: `/tmp/w6-tocdk/harness/` (net10.0 console) + `/tmp/w6-tocdk/run/` (сборка + все managed-зависимости + `RES/` для NGettext). Загрузка `RimworldModTranslator.dll` через кастомный `AssemblyLoadContext`; вызовы public static методов `EditorHelper` через reflection:

- `GetTranslatableFolders` → список «папок» мода (версии/LoadFolders)
- `LoadDefKeyedStringsFromTheDir` — **чтение готовых переводов** (DefInjected + Keyed XML построчным regex + Strings/*.txt + tar)
- `ExtractStrings` — извлечение из Defs по словарю тегов
- `CreateTranslationsTable` → `FillTranslationsData` → `WriteFiles` — roundtrip записи

Единственный трюк: первая папка в таблице — заглушка `*` (GUI-инвариант: `TranslationEditorViewModel.cs:368`, `EditorHelper.CreateTranslationsTable` делает `folders.Skip(1)`, EditorHelper.cs:735).

Команда прогона: `DOTNET_ROLL_FORWARD=Major ./harness /tmp/w6-tocdk/corpus/<id> /tmp/w6-tocdk/out-<id> load|write`

Корпус — рабочие копии 4 модов в `/tmp/w6-tocdk/corpus/` (источник только для чтения). Эталон: rimloc-cli `0.1.0-alpha.1` из `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (HEAD 72259e0b).

## 4. Результаты чтения готовых переводов (DefInjected/Keyed/Strings)

**Да, TokcDK работает с готовыми переводами** — фильтр в `XmlReaderBase.cs:19-24`: читает только `DefInjected/` и `Keyed/` XML (+ `Strings/` .txt через `DirTxtReader`, + `.tar` архивы языков через `TarXmlReader`/`TarTxtReader` на SharpCompress).

| Мод | rimloc scan | TokcDK stage A (готовые) | Совпадение |
|---|---|---|---|
| 818773962 HugsLib | 815 записей / 83 ключей | 815 строк, 13 языков, 16 subPath | **815 == 815, обе корзины пусты** — идентичное покрытие |
| 3242000764 Anomaly Patch | 40 (v1.6) | 40 (v1.6) + 32 (v1.5) | 40==40; TokcDK дополнительно держит v1.5-дубли |
| 2023507013 VE Framework | 764 (592 готовых + 172 defs) | 592 готовых (Keyed English) | **592 == 592**, rim_only_ready = 1 |
| 3170653412 патч-мод | 159 (из Defs+Patches, синтетические DefInjected-пути) | 0 готовых (Languages нет) | n/a |

**Единственная потеря на готовых**: `VEF.HiringDesc` в `2023507013/Languages/English/Keyed/UI.xml:13-14` — **многострочный** Keyed-элемент. Парсер TokcDK — построчный regex `^\s*<(?<tag>[^>]+)>(?<value>.*)</\k<tag>>\s*$` (EditorHelper.cs:586) — открывающий тег без закрывающего в той же строке не матчится, значение теряется **молча**. Rimloc собрал многострочное значение полностью.

### Roundtrip load→write (818773962)

`WriteFiles` (EditorHelper.cs:957-1021) записал 26 файлов, ключи круговые: Keyed 47/47, DefInjected 3/3, **порядок ключей сохранён**. Отличия от исходника — только косметика: BOM отброшен, отступ 2 пробела вместо таба/4, пустые строки и комментарии потеряны. Данные не искажены.

Ограничение формата записи: XML пишется **плоскими dot-тегами** (`<{stringId}>{value}</{stringId}>`, EditorHelper.cs:1007) — вложенная структура DefInjected не воспроизводится (для RimWorld валидно, но diff «на уровне дерева» всегда будет плоским).

## 5. Извлечение из Defs (словарь тегов) — diff с rimloc

ТокцDK извлекает только по плоскому словарю 38 имён (`_defsXmlTags`, EditorHelper.cs:60-101, переопределяется `RES/data/tags2extract.txt`), матч **case-sensitive** `List.Contains` (EditorHelper.cs:854), структура через `XDocument` + `Descendants()`, id строится из `defName` + путь с li→индекс (EditorHelper.cs:853-877).

| Мод | both | COMPETITOR_ONLY (TokcDK нашёл, rimloc нет) | RIMLOC_ONLY |
|---|---|---|---|
| 3170653412 | 106 | 2 (`GTF_WanderingBard.title/baseDesc`) | **53: все `titleShort` (52) + `titleShortFemale` (1)** |
| 2023507013 | 162 | 75 (`label` 41, `description` 16, `verb` 15, `labelNoun` 2, `baseInspectLine` 1 — поля `JobDef`/`WorkGiverDef`) | 9 (4 `Mote_*.label` — наследование ParentName; 5 `stages.li.label` — нотация) |

### Найденные дефекты TokcDK (полезные уроки для RimLoc)

1. **Case-bug словаря: `titleshort`/`titleshortFemale`** (EditorHelper.cs:97-99 и tags2extract.txt:39,42) против реальных полей RimWorld `titleShort`/`titleShortFemale`. Матч case-sensitive → 53 поля потеряны на одном BackstoryDef-моде. RimLoc берёт свои 53. Урок: словарь полей хранить в точном регистре RimWorld-XML.
2. **Многострочные значения Keyed/DefInjected теряются молча** (построчный regex) — см. `VEF.HiringDesc`.
3. **Нет резолвинга `ParentName`/`Abstract`**: `Mote_Firetrail` наследует `<label>Mote</label>` от abstract `ETE_MoteBase` (`2023507013/1.6/Defs/Motes/Motes.xml:33-35`), у rimloc ключ есть, у TokcDK нет.
4. **Дубли по версиям**: TokcDK держит отдельный набор ключей на каждую папку-версию (1.0-1.6 у VE → 51 ключ извлечён в 2+ версиях) и складывает их в одну таблицу с колонкой Folder. Rimloc дедуплицирует до уникальных ключей (764 vs сумма ~1300 у TokcDK).
5. **Нотация списков**: TokcDK строит `stages.0.label` (li→индекс, EditorHelper.cs:862-866), rimloc пишет `stages.li.label` — SEMANTIC_DIFFERENCE схемы ключей, одинаковое содержимое.

### Что TokcDK знает, а rimloc на корпусе не взял (COMPETITOR_ONLY — проверить наш словарь)

- **`WorkGiverDef.label`/`verb`**, `JobDef.label`/`verb`/`labelNoun`/`baseInspectLine` и т.п. (75 ключей на VE Framework) — у плоского словаря TokcDK нет типов, поэтому он берёт эти поля во всех Def'ах; rimloc на корпусе их не выдал.
- 7 ключей HugsLib `SettingsDef`-подобных (`HugsLibShortcuts.label` и др.) из Defs HugsLib — rimloc scan не выдал ни одного defs-ключа для 818773962.

RIMLOC_ONLY здесь не трактуется как «rimloc хуже»: пункт 2 (desc/name/text-класс имён) у TokcDK — источник ложных срабатываний; но `verb`, `labelNoun`, `structureLabel`, `stuffAdjective`, `adjective`, `summary`, `ideoName` — кандидаты для **dict-gap находок** в наш `defs_fields.json` (ниже).

## 6. Словарь полей TokcDK vs наш defs_fields.json (п.3 задачи)

Словарь TokcDK — **не словарь**: 38 плоских имён тегов без привязки к типам Def (одинаков для ThingDef и JobDef). Наш: 49 DefType → typed field-paths (`crates/rimloc-parsers-xml/assets/defs_fields.json`).

- **THEIR-ONLY** (нет у нас даже как первого сегмента, 26): `adjective, baseInspectLine, commandDesc, commandLabel, customLabel, customLetterLabel, customLetterText, desc, headerTip, ideoName, ingestCommandString, ingestReportString, labelNoun, member, name, outOfFuelMessage, pawnsPlural, slateRef, structureLabel, stuffAdjective, summary, text, theme, titleshort*2 (case-bug), verb`
- **OUR-ONLY** (их плоского аналога нет, 19): `comps, degreeDatas, gerundLabel, helpText, ingestible, ingredients, labelFemale, labelMale, labelShort, letterLabel, lifeStages, rulesStrings, scenario, stages, subSounds, titleShort, titleShortFemale, tools, verbs`
- Реальная находка из корпуса под.dict-gap: **WorkGiverDef `label`/`verb`/`gerund`**, JobDef-поля — TokcDK практикой показал, что такие дефы транслируемы.

## 7. Форматы read/write и обработка версий (статразбор)

**Читает:** DefInjected/*.xml, Keyed/*.xml (построчный regex); Strings/*.txt (id = `FileName.N`, EditorHelper.cs:665-686); языковые `.tar` (SharpCompress, `TarXmlReader`/`TarTxtReader`); `LoadFolders.xml` (EditorHelper.cs:363-417); `About.xml` (name, author, url, modVersion, packageId, supportedVersions, loadAfter, modDependencies с packageId/displayName/steamWorkshopUrl/downloadUrl — ModHelper.cs:23-59); `ModsConfig.xml` (чтение+**запись** актив-листа — GameHelper.cs:87-145); собственный кеш `RMT.DB.xml` (DataSet.WriteXml, EditorHelper.cs:163-247).

**Пишет:** DefInjected/Keyed XML плоскими тегами, Strings .txt, новый мод-перевод (`<mod>_Translated` + About.xml + LoadFolders.xml, EditorHelper.cs:1086-1275), ModsConfig.xml, RMT.DB.xml.

**Версии:** `VersionDirRegex = [0-9]+\.[0-9]+$` (EditorHelper.cs:30) — не заякорен слева, матчит и `v1.4`, и `1.5_1.6`; `EnumerateSupportedVersions` срезает только префикс `v` (EditorHelper.cs:1230-1236) — «1.5_1.6» уйдёт в supportedVersions как есть. **`IfModActive` в LoadFolders.xml игнорируется** — условные li считаются безусловными (на корпусе 3170653412: `1.5_1.6/Mods/Royalty` попал в папки без проверки атрибута). PatchOperations (`Patches/`) не читаются вообще — патч-моды для TokcDK непереводимы кроме Defs-подпапок (3170653412: нашёл только 108 строк в Defs, rimloc 159).

**Прочее:** черные/белые списки языков на чтение и запись; флаг «читать DefInjected только для уже извлечённых id» (`LoadOnlyStringsForExtractedIds`, XmlReaderBase.cs:10,33); автосохранение; поиск по таблице.

## 8. Статус и классификация

- **Статус прогона: PRACTICALLY_RUN** — компиляция и headless-исполнение ядра на всех 4 модах корпуса + roundtrip записи, локально, без платных API.
- GUI-рантайм: BLOCKED_PLATFORM (WPF, Windows-only) — не запускался.
- Классификация фич конкурента:
  - **MUST_FIX_BEFORE_BETA (для нас как уроки/гейты):** нет прямых аналогов — это его дефекты: case-bug словаря (`titleShort`), молчаливая потеря многострочных значений, отсутствие ParentName-резолва. Наши проверки: словарь в точном регистре, многострочные значения, inheritance — уже покрыты (rimloc взял все эти ключи).
  - **HIGH_VALUE_AFTER_BETA (наши dict-gap кандидаты):** WorkGiverDef/JobDef поля (`label`, `verb`, `labelNoun`, `baseInspectLine`), `structureLabel`, `stuffAdjective`, `adjective`, `summary`, `ideoName`, `pawnsPlural` — сверить с defs_fields.json (49 типов) и дозаполнить обоснованно.
  - **ROADMAP:** tar-архивы языков как источник готовых переводов (уникальная фича TokcDK; у rimloc на корпусе не проверялась — not run), RMT.DB.xml-кеш (внутренний формат, неinterop), ModsConfig-менеджмент (вне фокуса переводчика).
  - **INTENTIONAL_NON_GOAL:** дублирование по версиям в одной таблице; игнор IfModActive; построчный regex-парсер как замена XML-парсеру.

## 9. Артефакты

- `/tmp/w6-tocdk/TOKCDK_PRACTICAL.md` — этот отчёт
- `/tmp/w6-tocdk/w6-tocdk-diff.json` — diff-корзины BOTH/RIMLOC_ONLY/COMPETITOR_ONLY/SEMANTIC_DIFFERENCE по 4 модам + словарное сравнение + roundtrip-данные
- `/tmp/w6-tocdk/out-<id>/harness-report.json`, `/tmp/w6-tocdk/out-<id>/rimloc-scan.json` — сырые прогоны
- `/tmp/w6-tocdk/harness/` — headless-харнесс (переиспользуем для других C#-конкурентов)
- Клон с замороженным SHA: `/tmp/w6-tocdk/tocdk` (7752a0d6d106a3b7ad4dbb975ef6342349bfb9ee)

## 10. Честные оговорки

- Rimloc-стадии с MT-ключами не запускались (BLOCKED_DEPENDENCY) — сравнение только со scan-эталоном, как и предписано.
- Тесты TokcDK в репо отсутствуют (папки Tests нет) — прогон «его тестов» невозможен по определению; проверялось исполнением интерфейса на корпусе.
- tar-чтение реализовано в коде (TarXmlReader), но на корпусе tar-файлов нет — фактическое исполнение этой ветки не проверено.
- NGettext-каталог RES/Locale подложен в run-папку; перевод строк UI в headless не проверял (не относится к ядру).
