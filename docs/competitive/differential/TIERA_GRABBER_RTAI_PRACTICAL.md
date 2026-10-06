# Волна 6 — Grabber GUI + RimTransAI: практические прогоны (lane grabber-rimtransai)

Дата: 2026-10-06 · Лейн: `grabber-rimtransai` · Рабочая папка: `/tmp/w6-grabber-rtai/`
Дифф-JSON: `/tmp/w6-grabber-rtai/tiera-grabber-rtai-diff.json`
Методика — образец волны 3 (`docs/competitive/differential/TIERA_PYTHON_PRACTICAL.md`).

## 0. Методика и эталон

**Корпус** (копии для записи в `/tmp/w6-grabber-rtai/mods/`, оригиналы только на чтение):
`3170653412` (PatchOperations+LoadFolders, Defs/Patches в 1.4 и 1.5_1.6, условная папка `1.5_1.6/Mods/Royalty/Defs` по IfModActive, **Languages нет**),
`818773962` (HugsLib, root Defs + root Assemblies/*.dll + Languages/English/Keyed + версии `v1.1`–`v1.6`),
`2023507013` (VE Framework, Defs в 1.0–1.6, Languages/English на корне, Assemblies в версиях),
`3242000764` (Anomaly Patch, только Patches+Keyed в 1.5/1.6, Defs нет).

Замечание о расхождении с докой волны 3: там 3170653412 описан как «Languages/English/DefInjected на корне» — в корпусе **сегодня** папки Languages у этого мода нет (проверено `find` по оригиналу). 159 записей `scan` по этому моду — это Defs-ключи, которые RimLoc показывает с виртуальными путями `Languages/English/DefInjected/...` (файл реально лежит в `1.5_1.6/Mods/Royalty/Defs/`). Числа RimLoc при этом совпали с волной 3 — корпус не менялся, менялось описание.

**Эталон RimLoc**: бинарь `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (0.1.0-alpha.1, сборка tgbench, заявлен как HEAD 72259e0b).

**База RimLoc (мои прогоны сегодня, команда ниже):**

| Мод | scan | export-po (msgid) | learn-defs | learn-patches |
|---|---|---|---|---|
| 3170653412 | 159 | 163 (162 уникальных ключа) | 213 сырья → 162 уникальных | **0** |
| 818773962 | 815 | 76 | 7 | — |
| 2023507013 | 764 | 765 | 985 сырья → 171 уникальных в 1.6 | — |
| 3242000764 | 40 | 41 | — | — |

```bash
RIMLOC=/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli
$RIMLOC scan --root mods/<id> --format json --quiet
$RIMLOC export-po --root mods/<id> --out-po po-<id>.po --pot --quiet
$RIMLOC learn-defs --mod-root mods/2023507013 --out-dir …/learn-defs-ve --game-version 1.6 --quiet
$RIMLOC learn-patches --mod-root mods/3170653412 --out-json lp.json --quiet
```

`scan` у RimLoc читает Languages/* **и** Defs (без Languages — виртуализирует Defs в DefInjected-ключи), но root `Defs/` мода с Languages (HugsLib) не читает — 0 записей из `/Defs/` в scan-JSON; Defs-слой у RimLoc живёт в отдельной стадии `learn-defs`. Команды конкурентов — в их секциях. MT/LLM-стадии не запускались (запрещены условиями) → помечены BLOCKED_DEPENDENCY, локальные стадии (извлечение/экспорт) прогнаны.

---

## 1. Grabber GUI (doktorravlik-svg/RimWorld-Translator-Grabber-GUI) — MIT, активный

**Identity**: SHA `35b56bb211f85d41cf2a630495dcfa41b863f888`, последний коммит **2026-09-23** («Add files via upload» — репозиторий пополняется загрузками, не git-потоком), MIT (`LICENSE`, «Copyright (c) 2026 RimWorld Translator Team»). Русскоязычный: README Ru.md/Ua.md, логи и комментарии на русском. Найден `gh search repos rimworld grabber` (по «grabber gui rimworld» — пусто).

**Стек**: Python 3 + ttkbootstrap (Tk), lxml, loguru, rapidfuzz, pyspellchecker, **pymorphy3 + словари ru/uk** (морфология), deep-translator + googletrans-py (MT-fallback-цепочка, подтверждена requirements.txt). В репозиторий закоммичены артефакты: `debug.log` (1.5 МБ), `warnings.log`, `translation_anchors.db`, Windows-venv `env/`, лаунчеры `.bat`.

**Статус прогона: PRACTICALLY_RUN** — GUI-независимое ядро (collectors + loadfolders-парсер) прогнано headless на всех 4 модах. GUI (ttkbootstrap) **не запускался** — при этом он не Windows-only (Tk кроссплатформенный), так что BLOCKED_PLATFORM к нему неприменим; это честное «не прогонялся». MT-стадии (deep-translator/googletrans) — **BLOCKED_DEPENDENCY**, не запускались.

**Workflow**: GUI ведёт готовые переводы: скан мода → сравнение Languages/English с целевым языком → дубли-конфликты → MT-перевод недостающего → запись DefInjected/Keyed. Извлечение — на.collectors: `collect_defs_full` = рекурсивный поиск **всех** элементов с `defName` (включая вложенные) + резолв наследования ParentName/Name (`utils/parent_resolver.py`) + применение патчей (`utils/patch_processor.py`: Add/Replace/Remove/Name/sequence, в индекс строк, не в DOM).

**Поле-модель** (collectors/collectors.py:125-220, utils/rimworld_xml.py:90-276, filters_config.json):
whitelist 60 тегов (label/description/reportString/… + name, slateRef, tKey, rulesStrings, stageLabel, labelSocial, rejectionMessage) ∪ partial-совпадения (Message/Label/Title/Text/gerund/Explanation/description/Hint/Name) ∪ **space-fallback** (любой лист с пробелом в тексте), blacklist 20 тегов (defName, comps, statBases, texture…), 14 blacklist-паттернов (из них regex-подобные `.*\.points\.\d+` ищутся как **подстроки** и не срабатывают никогда — мёртвый код), фильтр векторов, min 2 / max 2000 символов (в коде дефолт 200 — расхождение с конфигом).

**Версии/папки** (utils/loadfolders_parser.py): Defs = LoadFolders-парс + «универсальный» скан (глубина 2: `<папка>/Defs`, `<папка>/<подпапка>/Defs`) + рекурсивный fallback. Languages = корень + LoadFolders + universal. **IfModActive не разбирается** (читается только текст `<li>`), `_detect_version` ищет корневые папки вида `N.N` и на моде с `1.5_1.6` возвращaeт **1.4** (папка `1.4` существует, `1.5` — нет).

**Команды воспроизведения** (драйвер: `/tmp/w6-grabber-rtai/grabber_run.py`):

```bash
python3 -m venv venv && venv/bin/pip install lxml loguru pymorphy3 pymorphy3-dicts-ru pymorphy3-dicts-uk
venv/bin/python grabber_run.py /tmp/w6-grabber-rtai/mods/<id> …
```

**Результаты по корпусу** (ALL = все найденные Defs-папки; NEWEST = auto-версия; KeyedEN = Languages/English):

| Мод | Defs-поля ALL | Defs-поля NEWEST | Keyed EN |
|---|---|---|---|
| 3170653412 | 213 (42 дефа) | **5** (auto-версия взяла 1.4) | 0 (Languages нет) |
| 818773962 | 7 (6 дефов) | 7 | 75 |
| 2023507013 | 306 (146 дефов, 7 версий) | 264 (124 дефа, 1.6) | 593 |
| 3242000764 | 0 (Defs нет — изящно) | 0 | 40 (1.6+1.5, first-wins=1.6) |

**Дифф с RimLoc** (полные корзины в diff-JSON):

| Слой | Корзина | Grabber |
|---|---|---|
| Keyed HugsLib | BOTH 75 / RIMLOC_ONLY 0 / COMP_ONLY 0 / SEMANTIC 7 | полный паритет |
| Keyed VE | BOTH 593 / RIMLOC_ONLY **171** / COMP_ONLY 0 / SEMANTIC 16 | 171 = DefInjected/*.xml (StatDef 91, JobDef, AbilityDef…): RimLoc export-po включает существующие DefInjected-папки, Grabber читает только Keyed+Defs |
| Keyed Anomaly | BOTH 40 / SEMANTIC 2 | версии смёржены first-wins → тексты 1.6 = RimLoc |
| Defs HugsLib | BOTH 7 / остальные 0 | тройной паритет RimLoc learn-defs = Grabber = RimTransAI (7 KeyBinding-полей) |
| Defs 3170653412 | BOTH 126 / RIMLOC_ONLY **36** / COMP_ONLY **87** | 36 = 3 дефа × 4 поля, живущие только в IfModActive-папке `Mods/Royalty/Defs` — Grabber условные папки не активирует; 87 = titleFemale/titleShortFemale/gerund и пр. за счёт partial-матчей |
| Defs VE 1.6 | BOTH 171 / RIMLOC_ONLY 0 / COMP_ONLY **62** | Grabber — **строгий суперсет** словаря RimLoc на 1.6 |

**SEMANTIC-разбор**: 25 из 32 — артефакты PO-экранирования RimLoc (литеральный `\n` → `\\n`, `"` → `\"` — корректный PO-формат, не дефект); **1 реальный случай** — порча разметки в msgid RimLoc: `<b>The HugsLib mod</b>` → `bThe HugsLib mod/b` (ключ `HugsLib_loadOrderWarning_text`) — **дефект волны 3 воспроизведён на tgbench-бинарнике**; остальные — текстовые различия между версиями 1.5/1.6 у Anomaly (у RimLoc честно 1.6).

**Находки:**
- Дефект автоопределения версии: `_detect_version` не понимает именование `1.5_1.6` → «newest»-срез = 1.4 (5 полей вместо 213). В GUI-сценарии «одна версия» инструмент молча берёт устаревший контент.
- Условные LoadFolders-папки (IfModActive) не поддержаны → теряется контент (`Mods/Royalty/Defs`).
- Дубль-политика «все версии сразу» без дедупликации текстов между версиями (сборка union полей).
- Жёстких отказов нет: мод без Defs обработан изящно (0 записей, не ошибка) — в отличие от RimTranslate волны 3.
- DLL/C#-извлечения нет вообще; BitB-анализа сборок нет — только XML.

**Verdict-классификации (для RimLoc):**
- Резолв ParentName/Name-наследования при извлечении (у Grabber есть, у RimLoc learn-defs нет) — **HIGH_VALUE_AFTER_BETA**: бэкстори/оружейные моды с абстрактными базами теряют унаследованные поля.
- Расширение defs-словаря полями из COMPETITOR_ONLY (62 на VE 1.6: gerund/verb/inspectString — игроку-видимые) — **ROADMAP**.
- Морфология pymorphy3 → **INTENTIONAL_NON_GOAL** (у RimLoc уже есть `morph` c pymorphy2-провайдером).
- Скан всех версий → **INTENTIONAL_NON_GOAL** (у RimLoc `--include-all-versions` есть; дефолт «новейшая» для перевода правильнее).
- Anchors/TM-база Grabber → **NOT_MATERIALLY_RELEVANT** для extraction-лейны (TM-сравнение — волна 3).

---

## 2. RimTransAI (mmjio-xy/RimTransAI) — GPL-3.0, активный

**Identity**: SHA `fb5d0d08c9c4c5805520d85e9a4c0d98bb3d438e`, последний коммит **2026-09-02** («统一产品版本管理并升级至 2.0.0», v2.0.0), LICENSE = GPL-3.0 (35 КБ, текст сверен). Китайскоязычный инструмент ханьализации. Найден `gh search repos RimTransAI` (задание ждало `RimTransAI/RimTransAI` — реальный канонический репо `mmjio-xy/RimTransAI`; identity Avalonia+Mono.Cecil+GPL из волны 3 сошлась).

**Стек**: `net9.0` (**не** `net9.0-windows`!) + Avalonia 12.1 (Semi.Avalonia) + Mono.Cecil 0.11.6 + OpenAI 2.6.0 + MiniExcel + Serilog; сателлит `RimTransAI.LocalTranslator` — LLamaSharp 0.27 (CPU) для оффлайн-перевода. `global.json` пинит SDK 9.0.315, на машине только 10.0.401.

**Статус прогона: PRACTICALLY_RUN** — с одной заявленной оговоркой:
- **Сборка на macOS удалась**: `dotnet build RimTransAI/RimTransAI.csproj -c Release` из cwd `/tmp` (обход global.json: SDK резолвится от рабочего каталога; `DOTNET_ROLL_FORWARD` на выбор SDK не влияет) → **0 ошибок, 0 предупреждений**. Задание предполагало «Avalonia → BLOCKED_PLATFORM» — **не подтвердилось**: проект generic-net9.0 и компилируется вне Windows. Запуск самого GUI не проверялся.
- **Extraction-ядро прогнано harness'ом** (`/tmp/w6-grabber-rtai/rtai-harness/`, net10.0-консоль с ProjectReference): вызывается **продуктовый путь** `ModParserService.ScanModFolder` (тот же, что конструирует MainWindowViewModel:82-88), LLM не вызывается.
- **Оговорка**: жёсткая зависимость ядра от Assembly-CSharp.dll игры (`ModParserService.cs:249-253` — без него скан отказывает целиком, return null). RimWorld на машине не установлен → LoadCore по референсной копии `/Users/danielviktorovich/Developing/compare/RimTrans/Reflection/References/Assembly-CSharp.dll` (только чтение). Это ограничение самого конкурента: у пользователя без настроенного пути к DLL инструмент не сканирует ничего.
- **LLM/MT-стадии (OpenAI-совместимый API, LLamaSharp) — BLOCKED_DEPENDENCY**, не запускались; прогнаны только локальные стадии скан/извлечение.

**Workflow**: LoadCore (Mono.Cecil-парс Assembly-CSharp → 120 «переводимых» типов) → опциональный анализ модовых DLL (`Assemblies/` в корне и в папках `N.N`/`Common`) → GameLoadOrderPlanner (LoadFolders.xml с **IfModActive/IfModActiveAll/IfModNotActive**) → LanguageDirectoryResolver → XmlSourceCollector (Defs/Keyed/DefInjected/Backstories/Strings/WordInfo) → DefFieldExtractionEngine. Причины извлечения в данных: `Defs.Whitelist`, `Defs.SmartSuffix`, **`Defs.ReflectionField`** (поле найдено в C#-сборке), `Defs.ListItem` (только [TranslationCanChangeCount]-списки типа rulesStrings), `Keyed.Leaf`.

**Команды воспроизведения:**

```bash
cd /tmp && dotnet build /tmp/w6-grabber-rtai/rimtransai/RimTransAI/RimTransAI.csproj -c Release
cd /tmp && dotnet run --project /tmp/w6-grabber-rtai/rtai-harness -c Release -- \
  /tmp/w6-grabber-rtai/mods/<id> …   # → /tmp/w6-grabber-rtai/rtai-items.json
```

**Результаты по корпусу** (items = поля+ключи; diagnostics из ScanDiagnostics):

| Мод | Items | Уникальных ключей | Mono.Cecil: типов из DLL | Диагностика |
|---|---|---|---|---|
| 818773962 | 82 (75 Keyed + 7 KeyBindingDef) | 82 | **+1** (`HugsLib.UpdateFeatureDef`) | Defs-файлов 2, Keyed 1, ошибок 0 |
| 2023507013 | **2209** (1616 Defs + 593 Keyed) | 1633 | **+192** (`VFECore.*`, `KCSG.*`…) | ReflectionField **712**, Whitelist 678, SmartSuffix 76, ListItem 150; LoadFolders=8 |
| 3170653412 | 53 (BackstoryDef) | 53 | +0 | Defs-файлов 4 (1.4 + 1.5_1.6, включая Royalty-pапку в плане) |
| 3242000764 | 40 Keyed | 40 | +0 | **конфликтов 32** (1.5 vs 1.6), ошибок 0 |

**Дифф с RimLoc:**

| Слой | Корзина | RimTransAI |
|---|---|---|
| Keyed HugsLib | BOTH 75 / SEMANTIC 7 | паритет; 7 = экранирование PO + 1 случай порчи разметки msgid у RimLoc |
| Keyed VE | BOTH 593 / RIMLOC_ONLY 171 / SEMANTIC 16 | те же 171 DefInjected, что и у Grabber: RimLoc берёт их в экспорт, RTAI не читает DefInjected-папки (DefInj=0 во всех сканах) |
| Keyed Anomaly | BOTH 40 / SEMANTIC 7 | RTAI дедуплицирует версии first-wins: для 32 ключей каноничным остался **текст 1.5**, RimLoc берёт 1.6 — 7 текстовых расхождений; приоритета «новейшей версии» нет |
| Defs HugsLib | BOTH 7 / остальные 0 | паритет с learn-defs |
| Defs 3170653412 | BOTH 53 / RIMLOC_ONLY **109** | RTAI покрывает Whitelist+SmartSuffix (53); RimLoc-словарь для BackstoryDef шире (titleFemale/titleShort…) — 109 ключей RimLoc-only |
| Defs VE 1.6 | BOTH 166 / RIMLOC_ONLY 5 / COMP_ONLY **52** | 5 = `stages.N.label` (RTAI **не извлекает подписи стадий** — только [TranslationCanChangeCount]-списки); 52 = gerund/verb/inspectString и пр., найденные через reflection |

**Находки:**
- **Уникальная фича подтверждена практически**: Mono.Cecil-анализ сборок даёт извлечение полей, которых нет ни в одном статическом словаре (712 записей ReflectionField на VE; +192 типа из модовых DLL). Ни Grabber, ни RimLoc этого не умеют.
- **Жёсткий входной барьер**: без Assembly-CSharp.dll не сканируется вообще (ModParserService отказывает до收集 файлов). RimLoc сканирует без игры.
- **Все версии сразу, без дефолта-новейшей**: VE 2209 items с дублированием по 7 версиям (в 1.2 — 758). Dedup между версиями Keyed — first-wins (Anomaly: остались тексты 1.5 при наличии 1.6).
- **stages.N.label не извлекаются** (политика «списки только с TranslationCanChangeCount») — RimLoc learn-defs и Grabber подписи стадий отдают.
- **Папки `v1.6/Assemblies` пропускаются**: regex версий `^\d+\.\d+$` не матчит `v1.6` (HugsLib) — у HugsLib спасает root `Assemblies/`; у модов с DLL только в `vN.N/` анализ сборок потеряется.
- LoadFolders-резолв с IfModActive — образцовый среди всех виденных конкурентов (в план 3170653412 вошли и условные Royalty-папки; не активированы корректно, т.к. Royalty не в ActivePackageIds).
- DefInjected-папки существующих переводов не читаются (не нужны его сценарию генерации, но RimLoc экспортирует их как источник).
- GUI-проект компилируется на macOS, но запуск не проверялся — «запуск Avalonia на macOS» остаётся непроверенным, сборка — проверенной.

**Verdict-классификации (для RimLoc):**
- **Reflection-extraction из C#-сборок** (Assembly-CSharp + модовые DLL через Mono.Cecil) — **ROADMAP**: единственный структурный дифференциатор лейны; тяжёлый (требует DLL игры, резолвер зависимостей, поддержку обфускации), но закрывает класс «поля, которых нет в словаре» (те самые 52 COMP_ONLY на VE). Бета-критичности нет — словарь RimLoc + learn-defs + пользовательские `--defs-dict` покрывают ядро.
- Читаемость существующих DefInjected как источника (RimLoc это уже делает) — подтверждение правильности текущего пути; конкурентного действия не требует — **INTENTIONAL_NON_GOAL**.
- Дефект RimLoc, подтверждённый обоими конкурентами независимо (Grabber и RimTransAI хранят сырой текст): **порча разметки в msgid export-po** — **MUST_FIX_BEFORE_BETA** (повтор находки волны 3 на новом бинарнике: `<b>X</b>` → `bX/b`).
- Defs-словарь в основном экспортном потоке (HugsLib KeyBindingDef.label есть в learn-defs, но не в export-po при наличии Languages) — **HIGH_VALUE_AFTER_BETA** (подтверждает волну 3: конкурент берёт эти строки в основной конвейер, RimLoc — только отдельной стадией).

---

## 3. Сводка лейны

| | Grabber GUI | RimTransAI |
|---|---|---|
| Repo / HEAD | doktorravlik-svg/RimWorld-Translator-Grabber-GUI `35b56bb` (2026-09-23) | mmjio-xy/RimTransAI `fb5d0d08` (2026-09-02, v2.0.0) |
| Лицензия | MIT (сверена) | GPL-3.0 (сверена) |
| Статус | **PRACTICALLY_RUN** (ядро headless, 4/4 мода); MT = BLOCKED_DEPENDENCY; GUI не запускался (не Windows-only) | **PRACTICALLY_RUN** (сборка + extraction-ядро на 4/4 мода через продуктовый путь); LLM = BLOCKED_DEPENDENCY; GUI-запуск не проверялся (сборка OK) |
| Язык/стек | Python + ttkbootstrap | C# net9.0 + Avalonia + Mono.Cecil |
| Keyed EN | 0 / 75 / 593 / 40 | 0 / 75 / 593 / 40 (идентично) |
| Defs-поля | 213 / 7 / 306 (264 в 1.6) / 0 | 53 / 7 / 1616 (все версии; всего items 2209 с Keyed) / 0 |
| Уникальное | ParentName-резолв, partial+space-fallback покрытие, pymorphy3 | ReflectionField из DLL (712 на VE), IfModActive-резолв |
| Критичные дыры | IfModActive-папки (−36 ключей), auto-version ломается на `1.5_1.6` | без Assembly-CSharp.dll не работает вовсе; stages.N.label пропускает; версии first-wins |
| Verdict для RimLoc | HIGH_VALUE_AFTER_BETA (ParentName) + ROADMAP (расширение словаря) | ROADMAP (reflection) |

**Blockers лейны**: MT/LLM-стадии обоих инструментов не прогонялись (запрещены условиями — BLOCKED_DEPENDENCY, честно). Запуск Avalonia-GUI RimTransAI не выполнялся (сборка проверена, запуск — нет). RimWorld не установлен — LoadCore RimTransAI шёл по референсной копии Assembly-CSharp.dll из `/Users/danielviktorovich/Developing/compare/RimTrans/Reflection/References/` (только чтение; влияние на состав core-типов не оценивалось — DLL чужой сборки RimTrans, версия не проверялась).

Артефакты: `TIERA_GRABBER_RTAI_PRACTICAL.md` (этот документ), `tiera-grabber-rtai-diff.json`, `grabber-items.json`, `rtai-items.json`, `rimloc-baseline/`, `grabber_run.py`, `rtai-harness/`, `mods/` (копии корпуса). В репозиторий RimLoc ничего не коммитилось.
