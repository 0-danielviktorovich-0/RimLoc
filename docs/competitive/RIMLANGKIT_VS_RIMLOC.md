# Бенчмарк: RimLangKit vs RimLoc на одном корпусе

Дата: 2026-10-05. Прогон оффлайн, без MT и без сетевых вызовов.
Evidence level: **6 SAME_CORPUS_DIFFERENTIAL** — полный прогон обоих инструментов на одном корпусе из 4 модов; главный продукт конкурента (WinForms GUI) не запускался (см. «Честные оговорки»).

## 1. Паспорт прогона

| | RimLangKit (конкурент) | RimLoc (эталон) |
|---|---|---|
| Репозиторий | https://github.com/OneCodeUnit/RimLangKit | worktree `~/Developing/_rimloc-worktrees/ba-main` |
| Зафиксированный SHA | `197df8ddc5821c426c9e4567b9eb3bae6995d6b5` (HEAD == origin/master, «Версия 3.7», 2025-10-26) | `da2fc777267dfec5c936d06a525b3a6f36d63c2b` (2026-10-05) |
| Язык / TFM | C#, `net9.0-windows7.0`, WinForms, WinExe, x64 | Rust, CLI |
| Лицензия | Apache-2.0 (`LICENSE.txt`) | — |
| Тип продукта | **GUI-приложение для обслуживания готового перевода**. Не библиотека, не CLI | CLI: scan/export-po/import/validate/learn-defs |
| Запуск в бенчмарке | модуль `TextExporter/RimFile.cs` (WIP), скомпилирован дословно в харнессе | `rimloc-cli scan` |

Корпус (только чтение): `~/Developing/rimloc-test-corpus/steamcmd-root/steamapps/workshop/content/294100/{3170653412, 818773962, 2023507013, 3242000764}`.

## 2. Инспекция RimLangKit — что он умеет (факты с путями)

**Главное: RimLangKit не извлекает строки из мода.** Все 9 функций GUI работают поверх уже готовой папки перевода (`Languages/<язык>/DefInjected/…`, XML с корнем `LanguageData`). README повторяет 5 раз: «Важно! Это действие подразумевает, что файлы уже обработаны RimTrans или его аналогом» (`README.md`, разделы «Добавить комментарии» … «Предварительный перевод»).

- **DefInjected/Keyed**: да, но только как вход. `Processors/TagCollector.cs:29-31` явно пропускает `LoadFolders.xml`, `About.xml`, `Keyed.xml`; `TagCollector.cs:47-51` отбрасывает папку Keyed. `Parsers/TranslationParser.cs:33-39` проверяет корень `LanguageData`.
- **Patches (PatchOperations)**: поддержки нет нигде — ни один класс не парсит `<Operation>`; `Processors/ChangesFinder.cs:56-74` («Поиск изменений в тексте») читает «мод» тоже только с корнем `LanguageData`, т.е. сравнивает две папки перевода, на сырых Defs падает.
- **Versioned content (LoadFolders, 1.x-папки)**: ноль упоминаний в коде. Единственная работа с версиями — `Processors/FileRenamer.cs` (приписка к имени файла префикса папки перед `Languages`).
- **Извлечение из модных Defs — один мёртвый WIP-модуль**: `TextExporter/RimFile.cs` — теги строго `{label, description, title, titleShort}` (`RimFile.cs:13`), только прямые дети дефа (`RimFile.cs:33-42`), только корень `<Defs>`. Проект `TextExporter (WIP).csproj` (net8.0) **не компилируется как закоммичен**: тип `RimTag` в нём отсутствует. Внутри же есть `FindDefs()` с захардкоженными личными путями `C:\Users\inqui\...`, не вызываемый из `Main`.
- **Форматы экспорта**: LanguageData-XML с комментарием `<!-- EN: … -->` (`RimFile.cs:Save`); ключ дефекта — `"{defName}.{defName}"` (дважды defName вместо defType.defName, `RimFile.cs:57`) — в WIP-коде; кодировка/нормализация — UTF-8 BOM, 2 пробела, CRLF (`Processors/EncodingFixer.cs`).
- **MT**: LLM нет. «Предварительный перевод» — translation memory из своих прошлых переводов в LiteDB (`Modules/AutoTranslation/AutoTranslator.cs`, `Repositories/TranslationRepository.cs`). Склонения — внешний сервис Morpher ws3.morpher.ru, лимит 100 запросов/сутки (`Services/MorpherService.cs:14-51`), версия/загрузка чужих переводов — GitHub API (`Services/GitHubService.cs`, `Modules/GameLocalization/LanguageUpdater.cs`).
- **Валидация**: `Checks/XmlErrorChecker.cs:8-26` — try-parse XDocument + проверка корня `LanguageData`; глубже (битые ссылки,_xpath, отсутствующие ключи) не идёт.
- **Портабельность**: пути парсятся `Split('\\')` (`Processors/TagCollector.cs:21-22`, `ChangesFinder.cs`/`RimFile.cs` — `path.LastIndexOf('\\')`) — Windows-only.

## 3. Харнесс (что пришлось построить, чтобы вообще что-то измерить)

`~/Developing/_competitive/rimlangkit-harness/` — консоль net10.0 с дословным `<Compile Include="../rimlangkit/TextExporter/RimFile.cs">` и шимом `RimTagShim.cs`, восстановленным строго по call-site `RimFile.Open()` (в Open: `new RimTag(tagName, value, tempDefType, tempDefName)`). Клон конкурента не правится. Особенности:

- публичного чтения результата нет: единственный акцессор `Save()` пишет `*_New.xml` в корпус — в бенчмарке запрещено; `tags` достаются reflection приватного поля (задокументировано в шапке Program.cs);
- Per-file статусы `Open()` (все файлы не-`<Defs>` отбрасываются с «Не удалось найти корневой элемент 'Defs'») пишутся в `skip_reasons`;
- вывод: `{file, key, source_text, def_type, def_name, tag}`.

Сборка: `dotnet build` — 0.7 s, 0 ошибок. Прогон: `dotnet run --no-build -- <modRoot> <out.json>`.

## 4. Команды и время

RimLangKit (харнесс): 28 / 32 / 94 / 23 мс на мод (Stopwatch внутри), 54/48/364/11 XML-файлов отсканировано.

RimLoc:
```bash
cd ~/Developing/_rimloc-worktrees/ba-main
CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/rimloc cargo build --release -p rimloc-cli
# (первая попытка упала os error 60 — внешний SSD уснул; повтор прошла, 2m02s инкрементально)
rimloc-cli scan --root <mod> --out-json rimloc-<id>.json --format json \
  --include-all-versions --with-patches --fuzzy
```
Wall-clock на мод (тёплый): 567 / 106 / 232 / 48 мс.

Дифф: `python3 /tmp/rimloc-diff/rlk_vs_rimloc_diff.py` — нормализация пробелов, ключ=(key, нормализованный текст), 4 корзины + разбор источников.

## 5. Дифф по модам (уникальные пары «ключ+текст»)

| Мод | RimLangKit | RimLoc | BOTH | RIMLOC_ONLY | RIMLANGKIT_ONLY | SEMANTIC |
|---|---|---|---|---|---|---|
| 3170653412 (GTF Backstories, Defs+патчи+свой EN-перевод) | 157 | 164 | **157** | 7 | 0 | 0 |
| 818773962 (HugsLib, flat Defs + 10 языков переводов) | 7 | 718 | **1** | 717 | 6 | 6 |
| 2023507013 (VE Framework, Defs 1.2–1.6 + Keyed) | 171 | 816 | **128** | 688 | 43 | 0 |
| 3242000764 (Anomaly Patch — только Languages, без Defs) | **0** | 40 | 0 | 40 | 0 | 0 |
| **Итого** | 335 | 1738 | **286** | **1452** | **49** | **6** |

Расшифровка RIMLOC_ONLY по источникам (см. `rlk-diff-<id>.json → rimloc_only_kind_breakdown`):
- 3170653412: 6 raw_defs (поля словаря BackstoryDef, которых нет в 4-теговом наборе конкурента: `baseDesc`, `workDisables`, `titleFemale`/`titleShortFemale` — два последних из патч-дефов в `1.5_1.6/Mods/Royalty`) + 1 existing_translation;
- 818773962: 717 existing_translation — HugsLib возит переводы 10+ языков; raw-Defs записи RimLoc **не дал вовсе** (см. семантическую разницу №1);
- 2023507013: 634 existing_translation + 54 raw_defs;
- 3242000764: 40 — мод состоит только из `1.6/Languages`; конкурент не извлёк ни одной строки.

Сырые записи vs уникальные: у конкурента на VE Framework 450 записей → 171 уникальная (279 — межверсионные дубли: один и тот же defName.field копируется в 1.2/1.3/1.4/1.5/1.6, и TextExporter без разбора версий берёт всё).

## 6. Семантические разницы (разобраны вручную)

1. **HugsLib: RimLoc не увидел raw Defs мода — только его возимые переводы.** Flat `Defs/KeyBinding{Category,}Defs/*.xml` (типы `KeyBindingDef`/`KeyBindingCategoryDef` отсутствуют во встроенном словаре `crates/rimloc-parsers-xml/assets/defs_fields.json` из 49 типов; словарная ветка `scan_defs_with_dict_meta` для типов вне словаря не извлекает ничего, а fuzzy-проход исключает label/description — `lib.rs:1230-1236` DEFAULTS skip-list). Итог: 6 ключей (`RestartRimworld.label`, `OpenLogFile.label`, `HLOpenUpdateNews.label`, `HLOpenModSettings.label`, `PublishLogs.label`, `HugsLibShortcuts.description`) у конкурента = английский источник, у RimLoc = испанский/немецкий/японский/турецкий/французский тексты из `Languages/*`. Обе стороны правы по-своему, но RimLoc пропустил единственный английский источник строк мода.
2. **Дыра словаря RimLoc на типизированных Defs (VE Framework): конкурент нашёл 43 пары, которых нет у RimLoc.** В `scan_defs_with_dict_meta` (`crates/rimloc-parsers-xml/src/lib.rs:1501-1505`) `all_fields` с DEFAULT_FIELDS (label/labelShort/labelPlural/description/helpText/reportString/gerundLabel) строится, но **не используется** — для типов из словаря берутся только словарные пути (у `HediffDef` в словаре лишь `stages.li.label/stages.li.description`), для типов вне словаря (`TrainableDef`, `ScenPartDef`, `OptionalFeaturesDef`, `ApparelLayerDef`, `KeyBindingDef`, `StatCategoryDef`) — ничего. Конкурент же берёт label/description у любого дефа фиксированным 4-теговым набором: 20 label + 19 description легитимных (например, `VEF_AcidBuildup.label` «acid burn» в 1.2–1.6, есть в файлах, проверено grep по корпусу). Это **отыгрыш конкурента** и кандидат на фикс в RimLoc.
3. **FP-классы у обеих сторон.** Конкурент: 4 записи `.label` «Mote» — абстрактный ThingDef без `<defName>` (1.3–1.6 `Defs/Motes/Motes.xml`), ключ вырождается в `.label`; RimLoc такие пропускает корректно (`lib.rs:1516-1518`, `def_name.is_empty() → continue`). RimLoc: fuzzy-проход считает «человекоподобным» любой текст с ≥2 словами — RGB-кортежи `(0.7, 1.0, 0.7)` проходят (3 токена) → 19 записей `*.defaultLabelColor` + 5 `*.color` — нетранслируемые цвета (FP); конкурент фиксированным тег-листом этот класс FP дать не может, зато не имеет и защиты от абстрактных дефов.
4. **Патчи и версионирование.** RimLoc с `--with-patches` достал `titleFemale`/`titleShortFemale` «ship girl» из патч-дефов `1.5_1.6/Mods/Royalty/Defs/BackstoryDefs/GTF_Backstories.xml` и разруливает версии через modview/LoadFolders; у конкурента обе способности отсутствуют на уровне кода (см. §2).

## 7. Метрики

- **Discovered (нашёл конкурент, подтверждено эталоном)**: 286 уникальных пар.
- **Missed (пропущено конкурентом, нашёл RimLoc)**: 1452 (83% из 1738 уникальных пар эталона). Причина структурная: у конкурента нет Keyed/DefInjected-скана мода, патчей, версионного рельефа и словаря полей — только 4 тега прямых детей Defs.
- **Competitor-only**: 49, из них 45 легитимных строк, пропущенных RimLoc из-за дыры словаря (отыгрыш конкурента), 4 — FP конкурента (абстрактные дефы).
- **FP-кандидаты RimLoc**: 24 записи-цвета (fuzzy), 1 класс «список значений» (`showDevelopmentalStageFilter` = «Child, Adult») — на фоне 1738 записей это ~1.7%.
- **Время**: конкурент-харнесс 23–94 мс/мод; RimLoc 48–567 мс/мод (тёплый). Сопоставимо; узкое место не в скорости, а в покрытии.
- **Сложность workflow до первой извлечённой строки**: RimLoc — 2 команды (build + scan), JSON с file/line/key/value из коробки. RimLangKit — клон → обнаружить, что продукт это WinForms GUI net9.0-windows (на macOS невыполним) → найти единственный читатель Defs в WIP-проекте, который не компилируется → восстановить отсутствующий тип по call-site → написать харнесс с reflection приватного поля, т.к. публичного акцессора нет (Save() пишет в корпус). 5 шагов, ~2 часа археологии.

## 8. Честные оговорки

- **Главный продукт RimLangKit (WinForms GUI) не запускался**: net9.0-windows рантайма на macOS нет, GUI-воркфлоу (FolderBrowserDialog) не автоматизируется. Измерен единственный компилируемый код извлечения — WIP `TextExporter/RimFile.cs`, дословно включённый в харнесс. Это лучшее из достижимого для этого репозитория; альтернативы (RimTrans-совместимый вход GUI) лежат вне «извлечения из мода».
- RimLoc в прогоне с флагами `--include-all-versions --with-patches --fuzzy`; без них цифры были бы другими (например, без --fuzzy пропали бы labelNoun/gerund/verb, без --include-all-versions — старые версии).
- Сравнение ключей — по (key, текст); формат ключей у сторон различается (у конкурента `defName.tag`, у RimLoc `defName.fieldPath` c путями вида `stages.li.label`), что учтено при ручном разборе, но длинные пути (comps.li…) в BOTH не попадают по построению.

## 9. Артефакты

- `/tmp/rimloc-diff/rimlangkit-<id>.json` ×4 — списки строк харнесса конкурента
- `/tmp/rimloc-diff/rimloc-<id>.json` ×4 — списки строк RimLoc scan
- `/tmp/rimloc-diff/rlk-diff-<id>.json` ×4 — 4-сторонний дифф с разбором источников
- `/tmp/rimloc-diff/rlk_vs_rimloc_diff.py` — нормализация и дифф (воспроизводимо)
- `~/Developing/_competitive/rimlangkit-harness/` — харнесс (obj/bin внутри своей папки)
- `~/Developing/_competitive/NOTES.md` — зафиксированные SHA/лицензия/TFM/структура
