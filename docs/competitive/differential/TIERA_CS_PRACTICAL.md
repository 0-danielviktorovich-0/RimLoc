# Tier A — практические прогоны C#/PowerShell/Node конкурентов RimLoc (лейн tierA-practical-2)

Дата: 2026-10-05 · Рабочая папка: `/tmp/w3-cs/` · Корпус: только чтение, `/Users/danielviktorovich/Developing/rimloc-test-corpus/steamcmd-root/steamapps/workshop/content/294100/{3170653412,818773962,2023507013,3242000764}`

## Эталон RimLoc

Заявленный готовый бинарь `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc` **отсутствовал** (проверено `ls`). Собран свой по регламенту:

- источник: `/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main`, HEAD **72259e0b98c78ca0f34057d86a5ad32f7deb24dd** (заявленный в задаче da2fc77 не проверялся — бинаря нет, строил из текущего HEAD worktree; файлы источников не менял);
- команда: `mkdir -p /Volumes/Portable-SSD/caches/targets/w3-cs && CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/w3-cs CARGO_INCREMENTAL=0 cargo build --release -p rimloc-cli` — успех за 7m29s;
- бинарь: `/Volumes/Portable-SSD/caches/targets/w3-cs/release/rimloc-cli`, версия `rimloc 0.1.0-alpha.1`.

Прогон эталона (выводы в `/tmp/w3-cs/out/rimloc/`): `scan --root <mod> --format json --out-json scan-<id>.json` по всем 4 модам + `learn-patches --mod-root` по двум патч-модам (обе дали `[]`).

Важно для методики: `rimloc scan` по умолчанию извлекает **все языки** (HugsLib: 815 записей суммарно по 13+ языкам, из них English = 75). Дифф ниже считается по **English-подмножеству** RimLoc против source-language-выдачи конкурентов.

## Сводка классификаций (словарь статусов)

| Конкурент | Классификация | Прогон на корпусе | Дифф-JSON |
|---|---|---|---|
| RimWorld-zh/RimTrans (канон RimTrans, TS) | **PRACTICALLY_RUN** | да, все 4 мода | `out/diff-rimtrans-zh.json` |
| Aironsoft/RimTrans (форк старой C#-линии) | **BLOCKED_PLATFORM** | нет (попытка компиляции задокументирована) | — |
| inkitter/RimTrans (однофамилец-инструмент 2017) | **BLOCKED_PLATFORM** | нет (WinForms net45, статически) | — |
| chance496/RimWorldAiTranslator | **PRACTICALLY_RUN** | да, все 4 мода + их тест-сьют | `out/diff-rwat.json` |
| DrizztGaming/Mod-Translation-Toolkit | **BLOCKED_PLATFORM** (pwsh отсутствует) + MT-стадии DOC_ONLY/BLOCKED_DEPENDENCY | нет | — |
| kelvinauta/Rimworld-Mod-Translator | **NOT_MATERIALLY_RELEVANT** | нет (не за чем) | — |

Окружение: macOS arm64, dotnet SDK **10.0.401** (только runtime 10.x), node **v26.10.0**, **pwsh не установлен** (`pwsh --version` → command not found), yarn/lerna не установлены (обошло npm-инсталлом).

---

## 1. RimTrans lineage — идентичность разрешена

### Канонический репозиторий: `RimWorld-zh/RimTrans` (MIT, © 2016-2019 duduluu)

- Заморозка: `RimWorld-zh-RimTrans` @ **8595889901623a34ffe5caba15f6739fc302ec08** (master, последний содержательный коммит «WIP» **2020-01-19**; последние движения в репо — dependabot-ветки до **2022-12-10** → «мёртв с 2022-12» подтверждено; `gh api repos/RimWorld-zh/RimTrans`: forks 23, stars 83 — «+19 форков» в задаче близко, измерено 23).
- Структура (lerna-монорепо): `app/` — GUI на Vue+Electron (`@rimtrans/app` 4.0.0-alpha.1), **`extractor/` — ядро (`@rimtrans/extractor` 4.0.0-alpha.1, TypeScript, jest-тесты)**, `Core/` — данные RimWorld Core (`@rimtrans/core`: Defs+English, без зависимостей), `Reflection/` — C# dotnet-консоль для выгрузки типов из DLL игры. То есть ядро — **не C#, а TypeScript**; C#-часть только вспомогательная и требует сборки игры.
- Ветки тегов: последний тег v0.18.2.6 — старая C#-линия; master — незавершённый TS-рерайт v4.

### Прогон ядра (успешный, PRACTICALLY_RUN)

`@rimtrans/*` не опубликованы в npm (404), yarn/lerna отсутствуют. Обход без правки их файлов: `npm install --no-save` зависимостей по месту (`fs-extra@8, globby@10, jsdom@12, prettier@1`, `../Core` как `@rimtrans/core`, babel 7 из корня) → `npx babel extractor/src -x .ts -d extractor/dist/cjs` (12 файлов) → драйвер `/tmp/w3-cs/builds/rimtrans-zh-driver.cjs` вызвал `Extractor.extract()` с `languages:['Template']`, `outputAsMod:true`, вывод в `/tmp/w3-cs/out/rimtrans-zh/<id>/`. **Все 4 мода отработали без ошибок.**

Результат: 3170653412 — 0 XML; 818773962 — 3 XML (Keyed + KeyBindingDef + KeyBindingCategoryDef); 2023507013 — 21 Keyed-XML; 3242000764 — 0.

### Дифф RimLoc(EN) vs RimTrans-zh

| Мод | RimLoc EN | RimTrans | BOTH | RIMLOC_ONLY | RIMTRANS_ONLY | SEMANTIC |
|---|---|---|---|---|---|---|
| 3170653412 | 159 | 0 | 0 | 159 | 0 | 0 |
| 818773962 | 75 | 87 | 71 | 0 | 12 | 4 |
| 2023507013 | 764 | 593 | 593 | 171 | 0 | 0 |
| 3242000764 | 40 | 0 | 0 | 40 | 0 | 0 |

Разбор корзин:
- **RIMLOC_ONLY 317 (159) и 324 (40)** — находка, не мусор: RimTrans не понимает LoadFolders.xml и version-папки (`1.4`, `1.5_1.6`, голый `1.6/` без LoadFolders) и не умеет синтезировать DefInjected из Defs вне своей type-package-модели — извлечение пустое.
- **VE (171)** — находка: у RimLoc 593 Keyed совпали с RimTrans **1:1** (BOTH=593, включая числовые `-1`/`+10` — RimTrans их тоже тащит), плюс 171 синтезированный DefInjected из `1.6/Defs` (label/description/reportString StatDef/JobDef и пр.), которых у RimTrans нет вообще.
- **HugsLib RIMTRANS_ONLY (12)** — RimTrans извлёк 12 KeyBinding-записей из корневых `Defs/KeyBindingDefs`, RimLoc scan по умолчанию — 0 (см. находку ниже). Находка против дефолтного режима RimLoc.
- **SEMANTIC (4)** — те же 4 rich-text ключа, что и с RWAT: RimLoc портит эскейпленные сущности (см. раздел RWAT).

### Aironsoft/RimTrans — BLOCKED_PLATFORM

Заморозка `Aironsoft-RimTrans` @ **588242e8abf903c1ee288f5b9e37c2fbfcb0b1e6** (все коммиты за один день 2021-09-13). MIT © 2016-2017 **duduluu** — тот же автор; README дословно: «Source of original code which I used: https://github.com/RimWorld-zh/RimTrans» → это **форк старой C#-линии** (v0.21.9.13) с правками «Defs в подпапках версий, поиск от новой версии к старой». Проекты: `RimTrans.Builder` (Library — ядро без WinForms), `RimTrans.Trans`/`TemplateExporter` (console Exe), `RimTrans.Lite` (WPF), все `<TargetFrameworkVersion>v4.6.1</TargetFrameworkVersion>`.

Попытка прогона ядра (честная): standalone-проект net8.0 в `/tmp/w3-cs/builds/aironsoft-attempt/`, компилирующий `RimTrans.Builder/**/*.cs` (Properties исключён как шум AssemblyInfo) → **22 ошибки CS0246: `Verse` (6), `RimWorld` (6), `UnityEngine`** — ядро напрямую пользуется классами игры; csproj ссылает `Assembly-CSharp.dll` по абсолютному пути `D:\Steam\steamapps\common\RimWorld\...` (RimTrans.Builder.csproj, HintPath). Итог: net461 (Windows-сборка) + зависимость от DLL установленной игры с Windows-путём = **BLOCKED_PLATFORM**, двойное основание.

### inkitter/RimTrans — BLOCKED_PLATFORM

Заморозка `inkitter-RimTrans` @ **cbfcb850be51dd0f3bdd73f57b13430011d56e2e** (последний коммит 2017-05-16). WinForms-приложение (`frmTranslator.cs`), `<TargetFrameworkVersion>v4.5</TargetFrameworkVersion>`, **лицензии нет** (в корне и вложенных папках файл LICENSE отсутствует — использовать код нельзя). К каноническому RimTrans отношения не имеет — независимый одноимённый инструмент китайского автора (releases содержит готовый `.exe`). Прогон на macOS невозможен по платформе (net45+WinForms), статический разбор.

---

## 2. chance496/RimWorldAiTranslator — PRACTICALLY_RUN

Заморозка @ **9264ade289fa1a3689416695488252dc593041ba** (HEAD master, последний коммит 2026-07-17, VERSION **1.1.0**). MIT © 2026 wjdck. Корейская ниша (README на корейском, применение в `Languages\Korean`, интеграция RMK). Активный.

Структура: `src/Core` (net8.0, 55 .cs — портативное ядро), `src/App` (WinExe net8.0-windows, UseWindowsForms → **GUI = BLOCKED_PLATFORM**, WinForms на macOS не поддерживается), `src/Native` (P/Invoke-слой), `tests/` (42 .cs, самописный Exe-раннер; заявленные в задаче «46 тест-файлов» — измерено 42), `tools/` (GlossaryTool, Tooling — CLI).

### Сборка и прогон ядра (успешный)

`global.json` пинует SDK 8.0.422; установлен только 10.0.401. Обход без правки их файлов: сборка из внешнего cwd (резолюция SDK по cwd) + `-p:TreatWarningsAsErrors=false` (их анализаторы на SDK 10 дают 3× CA1859 как ошибки — дрейф версий анализаторов, на их пиновом SDK 8 собирается чисто) + запуск с `DOTNET_ROLL_FORWARD=LatestMajor`. Harness `/tmp/w3-cs/builds/rwat-harness` вызывает `SourceExtractor.Extract(modRoot)` из `Core/Extraction/SourceExtractor.cs:81`.

Extraction на корпусе (корпус только читался): **3170653412: 51 · 818773962: 82 · 2023507013: 587 · 3242000764: 0**.

Важное в модели: патчи сознательно не извлекаются — `SourceExtractor.cs:119`: «Patch XML translation is disabled because RimWorld patch conditions and list handles cannot be resolved safely outside the game».

### Их тест-сьют

`dotnet test` ничего не гоняет (нет тест-фреймворка — самописный раннер). Собран host `/tmp/w3-cs/builds/rwat-tests-host` с их тест-исходниками + прямыми ссылками на Core и Native (их csproj Tests ссылается только на Core, и тип из Native не протекает — `dotnet build` их Tests падает CS0246 `RimWorldTranslatorRmkHistoryRow` (native/RimWorldTranslatorNative.cs:99, тип в глобальном namespace); вопрос к их сборке, не к macOS).

Результат их раннера: **82 теста, 45 PASS / 37 FAIL**. Первоначально 23/82 — падения от macOS TMPDIR (`/var` симлинк); с `TMPDIR=/private/tmp/w3-cs/tmp` — 45/82. Все 37 падений классифицированы: 19× `PlatformNotSupportedException: Stable directory identity requires Windows`, 5× `DllNotFoundException: kernel32.dll` + 3 каскадных IOException RMK-rollback, 2× `PlatformNotSupportedException: Atomic RMK … requires Windows`, остальное — ассерты платформозависимого fail-closed-поведения. То есть **ядро (extraction, storage, translation API, glossary) зелёное на macOS; native write-boundary/архив — Windows-only by design**.

### Дифф RimLoc(EN) vs RWAT

| Мод | RimLoc EN | RWAT | BOTH | RIMLOC_ONLY | COMPETITOR_ONLY | SEMANTIC |
|---|---|---|---|---|---|---|
| 3170653412 | 159 | 51 | 51 | 108 | 0 | 0 |
| 818773962 | 75 | 82 | 71 | 0 | 7 | 4 |
| 2023507013 | 764 | 587 | 587 | 177 | 0 | 0 |
| 3242000764 | 40 | 0 | 0 | 40 | 0 | 0 |

Разбор корзин (полные данные в `out/diff-rwat.json`):
- **317 RIMLOC_ONLY 108** — находка: поля `title` (52), `titleShort` (52), `titleFemale` (2), `baseDesc` (1), `titleShortFemale` (1) — RimLoc извлекает титулы бэкстори из Defs, а RWAT-список разрешённых полей (`SourceExtractor.cs:20-26`) их не содержит и молча пропускает. RWAT-захват (51) полностью совпал с RimLoc (BOTH=51, значения идентичны).
- **VE RIMLOC_ONLY 177** — раскладывается точно: 171 DefInjected-кандидат, синтезированных из `1.6/Defs` (RWAT без LoadFolders не спускается в version-папки — его `GetActiveContentRoots` (`SourceExtractor.cs:143-221`) без LoadFolders.xml возвращает только корень мода; у VE Defs живут в `1.0-1.6/`) + **6 мусор-кандидатов Keyed** (`'-1'`, `'+1'`, `'+10'`, `'-10'`, `'{0}: {1}'`, `'{0}'` — числа и плейсхолдеры, переводу не подлежат; RWAT их отфильтровал — в его пользу).
- **324 RIMLOC_ONLY 40** — находка: мод-патч с голой `1.6/` без LoadFolders.xml; RWAT дал 0, RimLoc нашёл `1.6/Languages/English/Keyed` (40).
- **818 COMPETITOR_ONLY 7** — RWAT сгенерировал DefInjected-кандидаты из корневых `Defs/KeyBindingDefs` (label), RimLoc scan по умолчанию — нет. Причина найдена и проверена: у HugsLib есть version-папки `v1.1-v1.6`, авто-выбор версии уводит Defs-сканирование в `v1.6/Defs`, которой нет; `scan --defs-dir <root>/Defs` возвращает 61 KeyBinding-запись (проверено: `out/rimloc/scan-818773962-defsdir.json`) → **дефолтный промах RimLoc, чинится флагом**.
- **SEMANTIC 4** — топ-находка против RimLoc: `HugsLib_updateRequired_text`, `HugsLib_loadOrderWarning_text`, `HugsLib_features_confirmIgnore`, `HugsLib_features_description` в исходнике содержат эскейпленные сущности (`&lt;b&gt;{0}&lt;/b&gt; …` — файл `818773962/Languages/English/Keyed/English.xml`). RWAT возвращает корректное `<b>{0}</b> requires version <b>{1}</b>…`; **RimLoc (72259e0b) отдаёт испорченное `b{0}/brequires versionb{1}/bof thebHugsLib library/b…`** — сущности разворачиваются и затем парсятся как XML, теги теряются с артефактами. У RimTrans-zh те же 4 ключа корректны. Перевод по такой исходной строке даст битый результат — чинить в RimLoc.

### TM-подход RWAT vs наш (выжимка для нашего TM)

RWAT («atomic storage»):
1. **Единый JSON project-store** (`ProjectStoreDocument`, `Core/Projects/`) — проект = один атомарный документ: исходные записи, переводы, статус ревью, заметки, история прогонов. «기존 번역, 메모, 검토 상태 보존» — существующие переводы/статусы переживают повторный анализ.
2. **Транзакционная запись** — `FileTransaction.Execute` (`Core/Storage/FileTransaction.cs:7`): снапшот целевых путей (SHA256+длина+mtime, до 16 384 целей — `FileSnapshotJournal.cs:33`), действие, откат при сбое; отдельный `FileRollbackResult` с детекцией конкурирующих записей (ConcurrentPaths) и recovery-сессией. Тесты «Storage.AtomicFaults» гоняют fault-injection атомарности.
3. **Идентичность записи** — `GetLocalizationIdentity(ns, key)` = `namespace:{ns}|key:{key}` (`SourceExtractor.cs:386`); чтение существующих переводов `ReadExistingLanguageMap` (`SourceExtractor.cs:316`) с ambiguity-детекцией.
4. **Write-boundary safety** — `PathSafety` (рефлекс-симлинки, network drive, workshop-путь как output запрещён `RequireNonWorkshopOutputRoot`).

Наш TM (`crates/rimloc-domain/src/tm.rs`): записи по ключу **(source_text, target_locale)** — изоляция по локалям; provenance AUTO (только ACCEPTED-ack apply)/IMPORT (никогда неtrusted: DRAFT)/MANUAL; статусы DRAFT<ACCEPTED<REVIEWED с ранками («сильная запись не ослабляется слабым путём»); fuzzy — bounded Levenshtein с порогом; персистенция в конверте проекта (`Project.tm`).

Выводы для нашего TM: (а) у RWAT статус ревью и TM живут в одном атомарном документе с журнальным откатом — у нас TM уже в конверте проекта, но атомарность у нас на уровне файла-конверта, многотarget-транзакций как у RWAT нет; (б) их идентичность `ns|key` привязана к структуре мода, наша (source_text, locale) survives-рефакторинг ключей — наше решение для переиспользования между модами сильнее, их — для точного ревью-цикла одного мода; (в) идея worth-воровства: детекция ConcurrentPaths при откате (кто ещё писал файл во время операции) и recovery-сессия на рестарте.

---

## 3. DrizztGaming/Mod-Translation-Toolkit — BLOCKED_PLATFORM (+MT: DOC_ONLY / BLOCKED_DEPENDENCY)

Заморозка @ **6cb58ad97d0dca6b0614027f618c4cd01ece630a** (v0.10.26, коммит 2026-09-11). MIT © 2026. Активный, PowerShell-монолит `src/RimWorld/ModTranslationToolkit.ps1` — **13 858 строк** + `launcher/*.vbs` (VBS-лаунчер).

Прогон невозможен: `pwsh --version` → command not found (pwsh на macOS не установлен; установка тяжёлой зависимости вне рамок — классифицирую BLOCKED_PLATFORM для всего инструмента). Дополнительно код Windows-центричен: строки 5-6 `Add-Type -AssemblyName PresentationFramework` + `System.Windows.Forms` (WPF/WinForms GUI — под macOS не работает даже с pwsh).

Статический разбор (источник фактов — сам монолит):
- **Extraction**: сканирует `Languages/` XML с различением `DefInjected\*` vs Language (`:5342`), Defs через `Add-Root (Join-Path $modPath "Defs")` + LoadFolders.xml (`:4971, :5402`) и fallback «version directories without LoadFolders.xml» (`:5444`), фильтр по корню `<Defs>` (`:5722`). Список ~38 переводимых полей Defs (`:6005-6010`: label, description, jobString, reportString, gerund, verb, labelShort, labelTendedWell, destroyedLabel, helpText, …) — шире RWAT-овского, сопоставим с RimLoc; резолв ParentName-наследования абстрактных Defs (двухпроходный скан, `:6026+`); генерация `labelPlural` для PawnKind «RimTrans/RimWorld compatibility» (`:5980-5998`); list-контейнер `rulesStrings` — единственный разрешённый (`:6017`).
- **MT-стадии**: DeepL (ключ в настройках, `:143-144`, план Free/Pro; док интерфейса `:253-271`) — BLOCKED_DEPENDENCY (платный ключ, ограничение №3); Google Cloud Translation v2 (`translation.googleapis.com`, `:797`, ключ console.cloud.google.com `:225`) — BLOCKED_DEPENDENCY; LibreTranslate — **локальный по умолчанию** `LibreEndpoint = "http://localhost:5000"` (`:145`), опциональный ключ, «Install LibreTranslate locally» (`:286`). Поднять LibreTranslate без тяжёлой установки нельзя (pip-стек с моделями), ключей нет → MT-стадии в целом DOC_ONLY (разобрано по коду, не запускалось).
- **Глоссарий EN→PL трёхслойный** (`:1210-1223`): RimWorld-контекстный `rimworld-glossary.csv` + общий `general-en-pl-glossary.csv` + бандловый словарь `data/en-pl-dictionary.csv` (138 терминов, формат `Term;Preferred;Alternatives;Notes` — с альтернативами и заметками «когда что предпочесть»).
- **Update-diff**: трекинг обновлений мода подтверждён тест-репортами в корне (`TEST_REPORT_v0.10.16_UPDATE_DETECTION_FIX…`), CSV-автосейвы (`:7316`).
- DLL-сканирование (`:5580-5595`) — только диагностика (строки из бинарей), не перевод.

Итого: парсерная часть — самый серьёзный PowerShell-парсер RimWorld из виденных (наследование, LoadFolders, plural-генерация), но прогон на macOS закрыт платформой; для нас материал для чтения, не конкурент в нашей нише (EN→PL, Windows, ручной GUI-процесс).

## 4. kelvinauta/Rimworld-Mod-Translator — NOT_MATERIALLY_RELEVANT

Подтверждено одной строкой: `Rimworld-Mod-Translator` @ **9eb39d4542471d07b0810402220577f344edf056** (один коммит «Core: first commit» 2024-06-25, мёртв) — `index.js` на 97 строк: DeepL-only (`https://api-free.deepl.com/v2/translate`, index.js:13; нужен ключ — сам по себе BLOCKED_DEPENDENCY), рекурсивный перевод всего 3 тегов `label`/`description`/`thoughtStageDescriptions` (index.js:52-57), захардкоженные `input`/`output` каталоги (index.js:87-88), лицензии нет, npm-имя `rimtranslate` 1.0.0. Ни Keyed/DefInjected-модели, ни LoadFolders, ни форматов вывода RimLoc — материально не релевантен.

---

## Блокеры и честные оговорки

1. Эталонный бинарь `rimloc-tgbench` отсутствовал — собран свой из HEAD **72259e0b** worktree ba-main (SHA зафиксирован выше); поведение заявленного da2fc77 не сверялось.
2. GUI и native-слой RWAT — Windows-only: 37/82 их тестов падают на macOS по PlatformNotSupported/kernel32; это архитектура инструмента, не дефект прогона.
3. Тест-сьют RWAT в их собственном csproj **не собирается** на SDK 10 (CS0246, Tests не ссылаются на Native напрямую); гонял их исходники через свой host-проект с добавленной ссылкой. На их пиновом SDK 8.0.422 не проверялось (SDK 8 не установлен).
4. Mod Translation Toolkit: только статический разбор; pwsh и LibreTranslate-сервер не поднимались (тяжёлые установки вне рамок).
5. RimTrans-zh: собран и прогнан только extractor; Electron-GUI и Reflection (нужен Assembly-CSharp.dll игры) не запускались.
6. «+19 форков» из задачи: измерено 23 форка у RimWorld-zh/RimTrans; полная инспекция 19+ форков не проводилась (вне фокуса — канон идентифицирован).
7. Диффы считаются на нормализованных (ключ, значение) с NFC+схлопыванием пробелов; ключи приведены к нижнему регистру без пробелов — форматные различия (dotted keys) совпадают у всех трёх инструментов, коллизий не обнаружено.

## Артефакты

- Отчёт: `/tmp/w3-cs/TIERA_CS_PRACTICAL.md`
- Дифф-JSON: `/tmp/w3-cs/out/diff-rwat.json`, `/tmp/w3-cs/out/diff-rimtrans-zh.json`
- Сканы RimLoc: `/tmp/w3-cs/out/rimloc/scan-<id>.json` (+`patches-<id>.json`, `scan-818773962-defsdir.json`)
- Выдачи RWAT: `/tmp/w3-cs/out/rwat/<id>.json`; RimTrans-zh: `/tmp/w3-cs/out/rimtrans-zh/<id>/`
- Клоны (заморожены, SHA в тексте): `/tmp/w3-cs/repos/…`; harness'ы: `/tmp/w3-cs/builds/…`
