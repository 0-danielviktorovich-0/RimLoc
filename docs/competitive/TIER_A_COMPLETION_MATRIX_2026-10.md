# TIER A — финальная матрица завершённости конкурентной очереди

Дата: 2026-10-05 · Лейн: tierA-reconcile (мандат §8) · Реконсиляция входов: wave 1 (source-инспекция, `COMPETITOR_DEEP_DIVE_2026-10.md` + `competitors.json`), wave 2 (same-corpus прогоны Text Grabber и RimLangKit), wave 3 (practical прогоны Python- и C#/PowerShell/Node-конкурентов), свежий discovery 2026-10-05.

**Статусный словарь**: PRACTICALLY_RUN / SOURCE_CONFIRMED_ONLY / DOC_ONLY / BLOCKED_PLATFORM / BLOCKED_DEPENDENCY / OBSOLETE_BUT_RELEVANT / IDENTITY_UNRESOLVED / NOT_MATERIALLY_RELEVANT.
**Шкала evidence**: 0 NOT_IMPLEMENTED · 1 DOC_ONLY · 2 SOURCE_CONFIRMED · 3 UNIT_TESTED · 4 INTEGRATION_TESTED · 5 BUILT_APP_E2E · 6 SAME_CORPUS_DIFFERENTIAL · 7 IN_GAME_PROVEN.
**Корзины диффа**: BOTH / RIMLOC_ONLY / COMPETITOR_ONLY / SEMANTIC_DIFFERENCE.

---

## 0. Реквизит эталона и валидность цифр (проверено в этом заходе)

- Эталон-бинарь **существует**: `ls /Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/` → `rimloc-cli` (14 282 640 байт, собран 2026-10-05 20:09), версия 0.1.0-alpha.1. Заявленная сборка — HEAD `da2fc777267dfec5c936d06a525b3a6f36d63c2b`.
- **Валидация «scan-поведение идентично текущему»**: `git diff da2fc77..72259e0b -- crates/rimloc-cli/src/commands/scan.rs crates/rimloc-parsers-xml/` → пусто (scan-путь не менялся); `git diff --stat 72259e0b..70fa342` (HEAD на момент реконсиляции `70fa342d2c4d05ddec82fd3d621a2654b43b6d78`, 2026-10-05 22:32) — только CI-воркфлоу, `gui/`, `session.rs`/`validate.rs` (мелочь) и тест-гейты. Цифры wave 2/3 валидны для текущего HEAD.
- Единственный крейс-файл, менявшийся `da2fc77→72259e0b` из затронутых прогонами, — `learn_patches.rs`, и дельта **только про пути записи** (write-guard `resolve_cli_out_path`/`ensure_free_output_path`), экстракция `scan_patches_texts` не тронута → вывод «learn-patches = 0 текстовых записей на корпусе» валиден, а побочное наблюдение w3-py про относительный out-dir на текущем HEAD уже починено.
- Расхождение лейнов разрешено: w3-cs рапортовал «бинарь отсутствует», потому что искал файл `rimloc` (без суффикса `-cli`); файл называется `rimloc-cli` и существовал с 20:09. w3-cs собрал собственный бинарь из `72259e0b` (`/Volumes/Portable-SSD/caches/targets/w3-cs/release/rimloc-cli`, 7m29s) — базовые числа совпали с tgbench (159/75EN·815/764/40), что дополнительно подтверждает идентичность.
- Корпус (только чтение): `~/Developing/rimloc-test-corpus/.../294100/{3170653412, 818773962, 2023507013, 3242000764}`.

---

## 1. Сводная матрица — 24 строки, пропусков нет

| # | Инструмент | Автор/репо | Язык | Лицензия | Посл. коммит | Статус | Ev. | SAME-CORPUS | Вердикт-класс |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Text Grabber | kamikadza13 | Python | **нет** | 2026-03-15 (v1.7.x) | PRACTICALLY_RUN | 5 | DONE (все 4 мода) | MUST_FIX-источник ×3 |
| 2 | RimLangKit | OneCodeUnit | C# .NET 9 WinForms | Apache-2.0 | 2025-10-26 (v3.7) | PRACTICALLY_RUN (ядро; GUI=BLOCKED_PLATFORM) | 6 | DONE | словарные дыры RimLoc |
| 3 | RimTrans (RimWorld-zh) + 23 форка | RimWorld-zh, duduluu | TS/Electron (ядро) + legacy C# | MIT | 2020-01-19 (код), 2022-12 (dependabot) | PRACTICALLY_RUN (extractor; GUI/Reflection нет) | 6 | DONE | исторический эталон |
| 4 | RimTrans (Aironsoft) | Aironsoft (= duduluu) | C# net461 | MIT | 2021-09-13 (1 день) | BLOCKED_PLATFORM | 2 | NOT DONE (попытка компиляции задокументирована) | форк старой C#-линии |
| 5 | RimTrans (inkitter) | inkitter | C# net45 WinForms | **нет** | 2017-05-16 | BLOCKED_PLATFORM | 2 | NOT DONE (платформа) | однофамилец, не lineage |
| 6 | RimTranslate | winterheart | Python (polib+lxml) | GPL-3.0 | 2024-04-27 | PRACTICALLY_RUN | 5 | DONE (полный цикл на HugsLib) | PO-предшественник |
| 7 | Translation Forge | Momaomao8787 | Python 3.11+ | MIT | **2026-10-05** (активен) | PRACTICALLY_RUN | 5 | DONE (scaffold/check/export; Defs-слой) | ROADMAP-воркфлоу |
| 8 | rimworld-mod-translator | laskinss27-cmyk | Python ( tkinter GUI + CSV) | MIT | 2026-08-13 | PRACTICALLY_RUN (локальные стадии; Sheets-шаг — ручной у пользователя) | 5 | DONE (extract+build) | INTENTIONAL_NON_GOAL |
| 9 | RimWorldModTranslator | NicoriciN89 | Python | Apache-2.0 | 2026-07-22 | PRACTICALLY_RUN (scanner+patches; Argos-MT не запускался) | 5 | DONE (фокус-дифф патчей) | **главный дифференциатор** |
| 10 | RimWorld AI Translator | chance496 | C# net8 (Core+WinForms) | MIT | 2026-07-17 (v1.1.0) | PRACTICALLY_RUN (ядро + их тест-сьют; GUI/native=BLOCKED_PLATFORM) | 6 | DONE | TM-инженерия |
| 11 | Mod Translation Toolkit | DrizztGaming | PowerShell 13.8k строк | MIT | 2026-09-11 (v0.10.26) | BLOCKED_PLATFORM (+MT-стадии DOC_ONLY / BLOCKED_DEPENDENCY) | 2 | NOT DONE (pwsh+WPF отсутствуют) | статический разбор |
| 12 | Remis | Drlinglong | Python PySide + FastAPI-слой | AGPL-3.0 | 2026-09-26 (v3.2.1) | SOURCE_CONFIRMED_ONLY | 2 | NOT DONE (вне объёма wave 3) | **стратегический компаратор** |
| 13 | RW Translator Grabber GUI | doktorravlik-svg | Python 3.14 | MIT (файл LICENSE, проверен) | 2026-09-23 (активен) | DOC_ONLY | 1 | NOT DONE | кандидат на py-лейн |
| 14 | RimTransAI | mmjio-xy | C# / Avalonia | GPL-3.0 (файл LICENSE, проверен) | 2026-09-02 (активен) | DOC_ONLY | 1 | NOT DONE | кандидат на cs-лейн |
| 15 | RimWorldModTranslator (JP) | etejasdgjjjj532 | Python | MIT | 2026-08-17 | SOURCE_CONFIRMED_ONLY | 2 | NOT DONE | non-goal (JP-XLSX) |
| 16 | RimworldModTranslator | TokcDK | C# WPF | GPL-3.0 | 2025-04-27 (спит) | SOURCE_CONFIRMED_ONLY | 2 | NOT DONE | non-goal (редактор) |
| 17 | rimworld-mod-llm-auto-translator | JalapenoLabs | TypeScript | MIT | 2025-08-05 (заброшен) | BLOCKED_DEPENDENCY (OpenAI-ключ — единственная функция) | 2 | NOT DONE | non-goal |
| 18 | Rimworld-Mod-Translator | kelvinauta | JS/Node | **нет** | 2024-06-25 (1 день жизни) | NOT_MATERIALLY_RELEVANT | 2 | NOT DONE (не за чем: 97 строк DeepL-only) | non-goal |
| 19 | rimworld-autonomous-translator | AutonomoAI | — (кода нет) | — | 2026-01-14 | DOC_ONLY | 1 | NOT DONE (нечего запускать) | маркетинг-репо; нарратив |
| 20 | RimTrans_PY | masakitenchi (Manifold Paradox) | Python | MIT (файл LICENSE, прочитан) | 2024-08-22 | SOURCE_CONFIRMED_ONLY | 2 | NOT DONE | **третье подтверждение патч-линии** — NEW |
| 21 | rimworld-rtl-translation-tools | mtimoustafa | Ruby | **нет** | 2024-05-05 | DOC_ONLY | 1 | NOT DONE | RTL-заметка — NEW |
| 22 | RimWorldTranslationTool | lenhare | Python | нет (API) | 2024-07-02 (1 день) | NOT_MATERIALLY_RELEVANT | 1 | NOT DONE | анти-паттерн секретов — NEW |
| 23 | rwmt (Multiplayer) | rwmt org | C# | MIT | 2026-08-03 | NOT_MATERIALLY_RELEVANT | 2 | NOT DONE | ложное срабатывание |
| 24 | Ludeon official workflow | Ludeon | данные+PR | — | активны | NOT_MATERIALLY_RELEVANT (как тул) | 1 | NOT DONE | контекст-ниша RimLoc |

### 2. Сводка завершённости

- **Строк в матрице: 24** (16 wave-1 записей − rwmt-омонимы и Ludeon-контекст оставлены строками, + RimLangKit/Text Grabber из wave 2, + 4 разбиения RimTrans-линии, + 3 новых discovery).
- **PRACTICALLY_RUN: 8** — Text Grabber, RimLangKit, RimTrans-zh, RimTranslate, Translation Forge, laskinss27, NicoriciN89, RimWorldAiTranslator.
- **SAME-CORPUS дифф получен для 8 из 24**; NOT DONE: 16, из них:
  - **7 feasible** (кандидаты следующего practical-wave): Remis, Grabber GUI, RimTransAI, RimTrans_PY, etejasdgjjjj532, TokcDK, rtl-tools;
  - **3 BLOCKED_PLATFORM**: Aironsoft (net461+DLL игры с Windows-путём), inkitter (net45 WinForms), MTT (pwsh+WPF отсутствуют на macOS);
  - **1 BLOCKED_DEPENDENCY**: JalapenoLabs (OpenAI-ключ — вся суть тулза; ограничение №3);
  - **5 не за чем / нечего запускать**: kelvinauta, rwmt, lenhare, AutonomoAI (кода нет), Ludeon workflow (не тул).
- **IDENTITY_UNRESOLVED: 0** — все линии идентифицированы, включая третий «RimTrans» (inkitter — независимый однофамилец) и Aironsoft (форк старой C#-линии duduluu, а не самостоятельный наследник).

---

## 3. Детальные строки

Формат каждой строки: IDENTITY · SOURCE REVIEW · PRACTICAL RUN · SAME-CORPUS RUN · GAPS FOUND (что вскрыто в RimLoc) · VERDICT.

### 3.1 Text Grabber — kamikadza13/Text-grabber

- **IDENTITY**: kamikadza13/Text-grabber, SHA `48697bc…` (main, «Update 1.7.2 Bugfixes», 2026-03-15), Python 3 + tkinter/ttkbootstrap/win32, лицензии НЕТ. Русское сообщество.
- **SOURCE REVIEW**: ev. **5** — родное ядро (поиск папок, Defs/Keyed-экстракция, Patch_grabber) запускалось на всех 4 модах через headless-драйвер поверх родных модулей; штатный продукт не запущен нигде (BLOCKED_GUI+BLOCKED_WINDOWS: `Text_Grabber.py:6-13`, `:397`, `:1346`). Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: headless-драйвер 170 строк, мод копировался в scratch (продукт пишет внутрь папки мода: `os.chdir`, `makedirs('_Translation')`, `rmtree`); настройки отклонялись от дефолта (все версии, патчи вкл.). GUI-этапы (merge абстрактных родителей) не воспроизведены.
- **SAME-CORPUS RUN**: DONE. Итого по 4 модам: **BOTH 1031 · COMPETITOR_ONLY 364 · RIMLOC_ONLY 4 · SEMANTIC 6** (TG всего 1401, RimLoc 1041 идентичностей). Детали: `differential/tgdiff-*.json`, `differential/text-grabber-diff-summary.json`, отчёт `TEXT_GRABBER_VS_RIMLOC.md`.
- **GAPS FOUND (RimLoc)**: (а) словарь транслируемых полей уже конкурента (~90 из 364 TG-only закрывается данными: WorkGiverDef/TrainableDef/KeyBindingDef/HediffDef labelNoun/MentalStateDef…); (б) `stages.li.label` схлопывает стадии 1..N (19 строк на VE); (в) полные дефы из PatchOperationAdd не извлекаются (163 строки на патч-моде); (г) entity-разметка `&lt;b&gt;` портится в msgid (4 ключа HugsLib); (д) баг `is_version_directory` (`version.rs:8-30`): числовой workshop-id считается версией → `--game-version 1.5` молча игнорируется (probe-проверено на 3242000764); (е) union-режим first-file-wins берёт текст старейшей версии (1.4 вместо 1.6).
- **VERDICT**: перенять пункты (а)–(г) как данные/алгоритмы; (д)–(е) — чистые дефекты RimLoc. **MUST_FIX-источник №1.** RimLoc сильнее: CLI/JSON-детерминизм, IfModActive/effective-view, отсутствие фабрикаций (TG синтезировал 125 фантомных titleFemale и перезаписал реальное значение), отсутствие substring-FP (`debugLabel` по подстроке `label`).

### 3.2 RimLangKit — OneCodeUnit/RimLangKit

- **IDENTITY**: OneCodeUnit/RimLangKit, SHA `197df8d…` («Версия 3.7», 2025-10-26), C# net9.0-windows WinForms, Apache-2.0. GUI обслуживания ГОТОВОГО перевода, не экстрактор.
- **SOURCE REVIEW**: ev. **6** SAME_CORPUS_DIFFERENTIAL — полный прогон обоих на одном корпусе; главный WinForms-GUI не запускался (net9.0-windows нет на macOS). Измерен единственный компилируемый код извлечения — WIP `TextExporter/RimFile.cs` в дословном харнессе. Статус: **PRACTICALLY_RUN** (ядро).
- **PRACTICAL RUN**: харнесс net10.0 с шимом `RimTag`, reflection приватного поля (публичного акцессора нет; `Save()` пишет в корпус — запрещено). 28/32/94/23 мс на мод.
- **SAME-CORPUS RUN**: DONE. Итого: **BOTH 286 · RIMLOC_ONLY 1452 · RIMLANGKIT_ONLY 49 · SEMANTIC 6** (RimLangKit 335, RimLoc 1738 уникальных пар). Детали: `differential/rlk-diff-*.json`, отчёт `RIMLANGKIT_VS_RIMLOC.md`.
- **GAPS FOUND (RimLoc)**: 45 из 49 competitor-only легитимны — дыра словаря: для типов вне `defs_fields.json` (TrainableDef, ScenPartDef, KeyBindingDef, StatCategoryDef…) RimLoc не извлекает ничего, а `all_fields`/DEFAULT_FIELDS построены, но не используются (`crates/rimloc-parsers-xml/src/lib.rs:1501-1505`); fuzzy-проход даёт ~1.7% FP (RGB-цвета как «человекоподобные»).
- **VERDICT**: конкурент не конкурент (83% missed структурно), но фикс словаря забрать (см. 3.1а). Склонения (Morpher ws3.morpher.ru, 100 запр./сутки) и TM на LiteDB — **INTENTIONAL_NON_GOAL** (внешний платный сервис; у RimLoc TM-модель богаче).

### 3.3 RimTrans (RimWorld-zh) + форки

- **IDENTITY**: RimWorld-zh/RimTrans, MIT © 2016-2019 duduluu, 83★ / **23 форка** (измерено w3-cs; «19+» deep dive уточнено). master заморожен `8595889…` (содержательный коммит 2020-01-19, dependabot до 2022-12-10). Уточнение реконсиляции: **master — lerna-монорепо с ядром на TypeScript** (`@rimtrans/extractor` 4.0.0-alpha.1, jest-тесты); C# WPF — только legacy-тег v0.18.2.6. Формулировка мастер-листа «C# WPF» исправлена.
- **SOURCE REVIEW**: ev. **6** — extractor собран и прогнан; GUI/Electron и C# Reflection (нужен Assembly-CSharp.dll игры) не запускались. Статус: **PRACTICALLY_RUN** (ядро).
- **PRACTICAL RUN**: `@rimtrans/*` не опубликованы в npm; обход — `npm install --no-save` по месту + `npx babel extractor/src` (12 файлов) + драйвер `Extractor.extract()` с `languages:['Template'], outputAsMod:true`. Все 4 мода без ошибок.
- **SAME-CORPUS RUN**: DONE (RimLoc отфильтрован до EN): **BOTH 664 · RIMLOC_ONLY 370 · RIMTRANS_ONLY 12 · SEMANTIC 4**. По модам: 317 — 0/159/0/0; 818 — 71/0/12/4; VE — 593/171/0/0; 324 — 0/40/0/0. Детали: `differential/diff-rimtrans-zh.json`.
- **GAPS FOUND (RimLoc)**: RIMTRANS_ONLY 12 на HugsLib — KeyBinding-записи из корневых Defs, которых нет в дефолтном scan RimLoc (та же находка, что у RWAT; чинится `--defs-dir`, проверено: 61 запись). RIMLOC_ONLY 370 — подтверждение силы RimLoc: LoadFolders, version-папки, синтез DefInjected из Defs (171 на VE) у RimTrans отсутствуют. SEMANTIC 4 — те же битые entity-разметки.
- **VERDICT**: исторический родоначальник жанра, мёртв ~4-6 лет; 19+ форков мертвы (крайний пуш 2023-01). Перенять: нечего, кроме подтверждения, что Keyed+DefInjected-модель RimLoc — правильное обобщение. **non-goal**.

### 3.4 RimTrans (Aironsoft)

- **IDENTITY**: Aironsoft/RimTrans, MIT © 2016-2017 **duduluu** — тот же автор; SHA `588242e…`, все коммиты за один день 2021-09-13. README дословно: «Source of original code which I used: https://github.com/RimWorld-zh/RimTrans».
- **SOURCE REVIEW**: ev. **2** (csproj/структура/README прочитаны; попытка компиляции). Статус: **BLOCKED_PLATFORM**. **Исправление мастер-листа**: это не «самостоятельный наследник», а **форк старой C#-линии** (v0.21.9.13) с правками «Defs в версионных подпапках, поиск от новой к старой».
- **PRACTICAL RUN**: попытка честная: standalone net8.0-проект, компилирующий `RimTrans.Builder/**/*.cs` → **22× CS0246** (`Verse`, `RimWorld`, `UnityEngine`) — ядро пользуется классами игры; csproj ссылает `Assembly-CSharp.dll` по абсолютному Windows-пути (`D:\Steam\…`, HintPath). Двойное основание блокировки: net461+WinForms и игровая DLL.
- **SAME-CORPUS RUN**: NOT DONE (платформа).
- **GAPS FOUND**: единственная идея — «поиск от новой версии к старой» уже покрыт effective-view RimLoc.
- **VERDICT**: **non-goal**, мёртв в день создания.

### 3.5 RimTrans (inkitter)

- **IDENTITY**: inkitter/RimTrans, SHA `cbfcb85…`, последний коммит 2017-05-16, C# net45 WinForms, **лицензии нет** (проверено по дереву; releases с готовым .exe).
- **SOURCE REVIEW**: ev. **2** (структура `frmTranslator.cs` frozen-клона). Статус: **BLOCKED_PLATFORM**.
- **PRACTICAL RUN**: не запускался — net45+WinForms на macOS невыполнимо; статический разбор.
- **SAME-CORPUS RUN**: NOT DONE (платформа).
- **GAPS FOUND**: нет.
- **VERDICT**: независимый однофамилец 2017 года, к каноническому RimTrans отношения не имеет, без лицензии использовать код нельзя. **non-goal**; строка закрыта «если существует» из мандата — существует, идентифицирован.

### 3.6 RimTranslate — winterheart/RimTranslate

- **IDENTITY**: winterheart/RimTranslate, SHA `ad1a8f7…`, последний коммит **2024-04-27** (≈17.5 мес тишины), GPL-3.0, v0.6.7, один файл `RimTranslate.py` (372 строки), polib+lxml.
- **SOURCE REVIEW**: ev. **5** — полный цикл extract→PO→XML прогнан. Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: `--source-dir` → PO (Defs по whitelist 24 полей + Keyed), `--output-dir` → DefInjected XML из PO, round-trip проверен тестовым переводом (statistics 81/0/1/82).
- **SAME-CORPUS RUN**: DONE. PO-записи: **0 / 85 / 323 / 0** по модам. Дифф на HugsLib (лучший случай): **BOTH 71 · COMPETITOR_ONLY 7 · SEMANTIC 4**; RIMLOC_ONLY на 317 — 163 (DefInjected конкурентом не читается).
- **GAPS FOUND (RimLoc)**: SEMANTIC 4 — **RimLoc портит разметку в msgid** (`<b>The HugsLib mod</b>` → `bThe HugsLib mod/b`); конкурент хранит сырой текст. COMPETITOR_ONLY 7 — KeyBindingDef.label из Defs попадает в его PO-конвейер, у RimLoc — только learn-defs-отчёт. Жёсткий дефект конкурента (не наш): нет root Defs → `quit()` **до** Keyed-стадии с exit 0 — 3 мода из 4 ему недоступны.
- **VERDICT**: спящий PO-предшественник RimLoc; как конвейер — **INTENTIONAL_NON_GOAL**, но его SEMANTIC-находка — **MUST_FIX_BEFORE_BETA** (msgid-разметка).

### 3.7 Translation Forge — Momaomao8787/Translation-Forge

- **IDENTITY**: SHA `5a21f9d…`, коммит **2026-10-05 (день аудита)** — самый активный в выборке, MIT, Python 3.11+, CLI `forge` + tkinter-UI на 15 языках.
- **SOURCE REVIEW**: ev. **5** — scaffold/check/export прогнаны. MT в инструменте нет вообще (human-in-the-loop), BLOCKED_DEPENDENCY не возникает. Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: `pip install -e` → `forge scaffold/check/export` по модам.
- **SAME-CORPUS RUN**: DONE (частично по слоям). Pending: **156 / 7 / 202 / err.defs_not_found** (безDefs-мод не обрабатывается, отказ изящный, exit 1). Дифф Defs-слоя на VE 1.6: **BOTH 196 · COMPETITOR_ONLY 6 · RIMLOC_ONLY 0** (4× Mote-заглушки — спорная ценность; 2× HediffDef.labelNoun — реальная находка). LoadFolders-резолв корректен.
- **GAPS FOUND (RimLoc)**: `labelNoun` и единичные поля вне словаря; больше ничего критичного — whitelist forge почти совпадает с learn-defs.
- **VERDICT**: **ROADMAP** — workflow сопровождения готового языкового пакета (check/stale/import) — отдельный продуктовый срез, не для беты. Автор сам документирует 13 классов пропусков (только статические Defs, нет Keyed/патчей).

### 3.8 rimworld-mod-translator — laskinss27-cmyk

- **IDENTITY**: SHA `8dc978d…`, коммит 2026-08-13, MIT, один файл 1796 строк (tkinter GUI Windows-first) + CSV→Google Sheets flow.
- **SOURCE REVIEW**: ev. **5** — extract+build прогнаны вызовом методов без GUI (логика 1:1, `rimworld_translator.py:1419-1560`). Sheets-перевод — **ручной шаг пользователя** (формула `=GOOGLETRANSLATE(...)`), не API-вызов тулза; ключи не заводились по ограничению №3 → классификация стадии: ручной шаг вне MT-API, не BLOCKED_DEPENDENCY (в тулзе API-вызовов нет). Статус: **PRACTICALLY_RUN** (локальные стадии).
- **PRACTICAL RUN**: `find_xml_files` → `collect_identifiers` → `extract_from_file`; build_translation_package → корректный перевод-мод (Keyed XML + About.xml + generation report).
- **SAME-CORPUS RUN**: DONE. Extract: **181 / 83 / 1701 / 72** строк CSV. Нюанс: 1701 на VE — все 7 версий в кучу (нет резолва версий), 72 на 324 — смесь 1.5+1.6.
- **GAPS FOUND (RimLoc)**: критичного ничего. Обратные находки (его регрессии, наши преимущества): пропускает `patches/` целиком (`rimworld_translator.py:1375`), не читает Languages/English/DefInjected как источник (`:1383-1385` → 163 записи RimLoc мимо), нет версионности.
- **VERDICT**: **INTENTIONAL_NON_GOAL**. На заметку: трёхслойный фильтр `is_translatable` + кросс-проверка defName-идентификаторов («Fire/Wood не переводить») — умная эвристика-фильтр для наших fuzzy-FP (RGB-цвета).

### 3.9 RimWorldModTranslator — NicoriciN89

- **IDENTITY**: SHA `ccb1df5…`, коммит 2026-07-22, Apache-2.0, Python, Windows GUI (PyInstaller). Оффлайн: Argos Translate (bundled en→ru/uk/de/fr) + Ollama-полировка.
- **SOURCE REVIEW**: ev. **5** — scanner (354 стр.) и patches.py (132 стр.) прогнаны на всём корпусе; MT-стадия (Argos) не запускалась (вне объёма ask — отдельно классифицировать нечего: бандлы локальны, ключей не требуют). Статус: **PRACTICALLY_RUN** (scanner/patches).
- **PRACTICAL RUN**: программный вызов `src.scanner.scan_mod` на 4 модах + фокус-дифф патч-экстракции на 3170653412 v1.5_1.6.
- **SAME-CORPUS RUN**: DONE. Scan: **313 / 82 / 1113 / 40** (Keyed+DefInjected-задачи; 313 = 213 Defs-fallback + **100 патч-производных**). Фокус-дифф патчей: у RimLoc **0**, у конкурента **100 refs, из них 54 игроку-видимых** (description 26, title 11, baseDesc 6, titleShort 5…) и 46 шума (bodyType* 30, requiredWorkTags 11, spawnCategories 3). Пересечение ключей с export-po RimLoc — 0 (патчи бьют по ванильным/чужим def-ам).
- **GAPS FOUND (RimLoc)**: **патч-слой — крупнейшая подтверждённая дыра**: learn-patches=0, scan --with-patches Δ=0 на моде с 54 игроку-видимыми строками в патчах. Урок из шума конкурента: экстракция обязана идти с полевым blacklist (bodyType*, spawnCategories, requiredWorkTags, backstoryFilters*) — иначе ~46% шума с риском порчи матчинга идентификаторов.
- **VERDICT**: **главный дифференциатор и MUST_FIX_BEFORE_BETA**: семантику «полные значения PatchOperationAdd/Replace/Insert + жёсткий фильтр полей» брать в бету. Оффлайн Argos+Ollama — категория «локальный MT», у RimLoc это PROVIDER-слой; не копировать (качество Argos ниже облачного), но слот «оффлайн-провайдер» в роадмапе подтверждён спросом.

### 3.10 RimWorld AI Translator — chance496/RimWorldAiTranslator

- **IDENTITY**: SHA `9264ade…`, HEAD master, коммит 2026-07-17, VERSION 1.1.0, MIT © 2026 wjdck. Корейская ниша (RMK-интеграция). Core net8.0 (55 .cs) + App net8.0-windows WinForms + Native P/Invoke + tests (42 .cs — **уточнение: 42, не 46**).
- **SOURCE REVIEW**: ev. **6** — ядро собрано, прогнано на корпусе, их тест-сьют исполнен. Статус: **PRACTICALLY_RUN** (ядро; GUI/native=BLOCKED_PLATFORM).
- **PRACTICAL RUN**: сборка из внешнего cwd (обход их пина SDK 8.0.422 при установленном 10.0.401) + `DOTNET_ROLL_FORWARD=LatestMajor`; харнесс `SourceExtractor.Extract()` (`Core/Extraction/SourceExtractor.cs:81`). Их раннер: **82 теста, 45 PASS / 37 FAIL** — все 37 падений классифицированы как Windows-only by design (19× PlatformNotSupportedException «Stable directory identity requires Windows», kernel32, RMK-rollback) — ядро зелёное на macOS.
- **SAME-CORPUS RUN**: DONE: **BOTH 709 · RIMLOC_ONLY 325 · COMPETITOR_ONLY 7 · SEMANTIC 4**. По модам: 317 — 51/108/0/0; 818 — 71/0/7/4; VE — 587/177/0/0; 324 — 0/40/0/0. Детали: `differential/diff-rwat.json`.
- **GAPS FOUND (RimLoc)**: (а) COMPETITOR_ONLY 7 — DefInjected-кандидаты из корневых Defs при версии-папках без контента: дефолтный промах RimLoc, **чинится флагом** `--defs-dir` (проверено: 61 запись); (б) SEMANTIC 4 — те же битые entity (`&lt;b&gt;` → разворачиваются → парсятся как XML, теги теряются; RWAT и RimTrans-zh возвращают корректно — **топ-находка против RimLoc**); (в) RIMLOC_ONLY 6 на VE — мусор-кандидаты Keyed (`'-1'`, `'{0}: {1}'`…) — **в пользу конкурента**: наш union-режим тащит числа-плейсхолдеры, RWAT отфильтровал.
- **VERDICT**: перенять: (1) детекцию ConcurrentPaths при откате + recovery-сессию (их `FileTransaction`/`FileSnapshotJournal`); (2) фильтр чисел/плейсхолдеров-заглушек из выдачи. Наша TM-идентичность (source_text, locale) сильнее их `ns|key` для межмодового переиспользования — не менять. Корейская ниша/RMK — **non-goal**.

### 3.11 Mod Translation Toolkit — DrizztGaming/Mod-Translation-Toolkit

- **IDENTITY**: SHA `6cb58ad…` (v0.10.26, 2026-09-11), MIT, PowerShell-монолит 13 858 строк + VBS-лаунчер, WPF-GUI.
- **SOURCE REVIEW**: ev. **2** — статический разбор монолита (grep-карта по строкам). MT-стадии: DeepL (`:143-144`) и Google Cloud v2 (`:797`) — **BLOCKED_DEPENDENCY** (платные ключи, ограничение №3); LibreTranslate — локальный по умолчанию (`:145`), но подъём сервера — тяжёлая установка вне рамок → MT в целом **DOC_ONLY**. Статус: **BLOCKED_PLATFORM**.
- **PRACTICAL RUN**: не запускался: `pwsh --version` → command not found; даже с pwsh WPF/WinForms (`Add-Type PresentationFramework`, `:5-6`) на macOS не работает.
- **SAME-CORPUS RUN**: NOT DONE (платформа).
- **GAPS FOUND (RimLoc)**: по статике — их парсерная часть серьёзная: LoadFolders + fallback «version directories без LoadFolders.xml» (`:5444`), ParentName-наследование двухпроходным сканом (`:6026+`), ~38 полей (`:6005-6010`) шире RWAT, `labelPlural`-генерация для PawnKind (`:5980-5998`), `rulesStrings` единственный list-контейнер (`:6017`). Для нас: сверить наш словарь полей с их списком (пересечение с находками TG/RimLangKit).
- **VERDICT**: EN→PL + Windows + ручной GUI — **non-goal** в нашей нише; парсер — материал для чтения. Глоссарий-слои (Mod > RimWorld > General с запрещёнными вариантами) — идея на ROADMAP глоссария.

### 3.12 Remis — Drlinglong/Remis

- **IDENTITY**: Drlinglong/Remis, Python (PySide-десктоп + SQLite + агент-слой), AGPL-3.0, 26★, последний коммит 2026-09-26, релизы v3.x, 2295 файлов. Мультиигровая платформа (Paradox — фокус; RimWorld в preview).
- **SOURCE REVIEW**: ev. **2** — `scripts/core/game_adapters/rimworld.py` (439 строк, полностью), `api_handler.py`, `translation_reuse.py` + дерево (wave 1). Статус: **SOURCE_CONFIRMED_ONLY**.
- **PRACTICAL RUN**: НЕ ЗАПУСКАЛСЯ. Причина: не входил в объём wave 3 (оба лейна закрывали Python-CLI и C#/PS/Node-инструменты; Remis — тяжёлый PySide-десктоп с мультиигровым ядром ~2295 файлов, требует отдельного лейна с собственным харнессом). **Feasible** — кандидат на следующий practical-wave (ядро headless-драйвером, как Text Grabber).
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND (RimLoc)**: по исходникам — у конкурента есть то, чего у RimLoc нет продуктово: MT-роутер 7+ провайдеров + локальные LLM + model arena; консервативное переиспользование переводов с диска; чекпойнты `.remis_checkpoint_*`; glossary health review; встроенные агенты (Codex/Copilot) и MCP-адаптер; publication identity. Патчи: честная диагностика «offline extraction cannot resolve» — НЕ извлекает (наша ниша).
- **VERDICT**: **стратегический компаратор (threat HIGH)**. Перенять: модель чекпойнтов и модель-арену (сравнение провайдеров) — в ROADMAP суперсета; переиспользование с диска — сверить с нашим TM. AGPL-3.0 защищает код, не идеи. RimLoc сильнее: патчи (у них 0), PO/XLIFF, Rust-кроссплатформенность, сфокусированность.

### 3.13 RW Translator Grabber GUI — doktorravlik-svg/RimWorld-Translator-Grabber-GUI — NEW подтверждение

- **IDENTITY**: Python 3.14, **MIT** (файл LICENSE прочитан сегодня: «MIT License, Copyright (c) 2026 RimWorld Translator Team»; API-детект GitHub отдаёт null — расхождение разрешено в пользу файла), создан 2026-04-14, push 2026-09-23, активен. Русский проект.
- **SOURCE REVIEW**: ev. **1** — заявленные фичи мастер-листа wave 1 (8+ MT-движков от Google до Argos, PyMorphy3-морфология, SQLite-кеш) не перепроверены построчно в этом заходе; сегодня подтверждены метаданные и дерево (`collectors/`, `core/`, `gui.py`, `filters_config.json`, `analyze_locales.py`). Статус: **DOC_ONLY** (до чтения исходников).
- **PRACTICAL RUN**: NOT DONE (не входил в wave 3). **Feasible**: чистый Python — кандидат на py-лейн тем же способом, что Text Grabber.
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: заявленная MT-fallback-цепочка — прямой аналог планируемого PROVIDER-слоя RimLoc; PyMorphy3 — русский морфологический слот (у RimLoc Russian morphology = LEVEL 0).
- **VERDICT**: приоритет №1 следующего practical-wave: единственный активный русскоязычный инструмент с MT-цепочкой. Verdict отложен до прогона.

### 3.14 RimTransAI — mmjio-xy/RimTransAI — NEW подтверждение

- **IDENTITY**: C# / Avalonia, **GPL-3.0** (файл LICENSE прочитан сегодня: «GNU GENERAL PUBLIC LICENSE Version 3»), создан 2026-01-09, push 2026-09-02, активен. Китайский сегмент.
- **SOURCE REVIEW**: ev. **1** — заявленное (Mono.Cecil-рефлексия типов из DLL + LLM-батч) не перепроверено построчно; подтверждены метаданные, дерево (`RimTransAI/`, `RimTransAI.LocalTranslator/`, `tests/`, `Version.props`), лицензия. Статус: **DOC_ONLY**.
- **PRACTICAL RUN**: NOT DONE. **Feasible** частично: Avalonia кроссплатформенна — кандидат на cs-лейн.
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: Mono.Cecil-подход (строки из скомпилированных DLL) — единственный в выборке кандидат на закрытие C#-TKey-пробела без запуска игры (у RimLoc C# TKey = LEVEL 4 только через собственный Runtime Bridge).
- **VERDICT**: приоритет №2 cs-лейна. Verdict отложен до прогона.

### 3.15 RimWorldModTranslator — etejasdgjjjj532 (JP)

- **IDENTITY**: Python (translator.py 180 строк + gui.py), MIT, 2026-08-17, 1★.
- **SOURCE REVIEW**: ev. **2** (wave 1, translator.py прочитан). Статус: **SOURCE_CONFIRMED_ONLY**.
- **PRACTICAL RUN**: NOT DONE — микропроект: Defs/Keyed/Patches → XLSX для японского, merge существующих JP-переводов. Не за чем: способность дублируется нашими export-форматами (PO/CSV/XLIFF).
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: заявляет патч-извлечение — по исходникам это чтение Patches в таблицу, без xpath-резолва (не competitor-находка).
- **VERDICT**: **non-goal** (JP-XLSX-ниша, микро).

### 3.16 RimworldModTranslator — TokcDK

- **IDENTITY**: C# WPF, GPL-3.0, 51 .cs, последний коммит 2025-04-27 (спит), 0★, README RU+EN.
- **SOURCE REVIEW**: ev. **2** (wave 1: ModHelper.cs + дерево). Статус: **SOURCE_CONFIRMED_ONLY** (BLOCKED_PLATFORM вероятен, но попытки не было — честнее SOURCE_CONFIRMED_ONLY).
- **PRACTICAL RUN**: NOT DONE — WPF-редактор таблицы переводов с автосейвом; не конвейер.
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: нет (редактор дублирует наш React-workspace LEVEL 5).
- **VERDICT**: **non-goal**.

### 3.17 rimworld-mod-llm-auto-translator — JalapenoLabs

- **IDENTITY**: TypeScript (Node, openai SDK), MIT (LICENSE прочитан сегодня — MIT-текст подтверждён), последний коммит 2025-08-05 — заброшен через день после создания. `llm.ts` сам помечен «TO BE DEPRECATED in favor of LangGraph».
- **SOURCE REVIEW**: ev. **2** (wave 1: `src/index.ts`, `src/llm.ts`). Статус: **BLOCKED_DEPENDENCY** — единственная функция тулза это OpenAI-API-вызов (платный ключ, ограничение №3); локальных стадий, которые можно прогнать без ключа, нет (разве что `npm install`).
- **PRACTICAL RUN**: NOT DONE (зависимость). 
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: уникальный анти-урок: правила конвертации путей (включая `1.6/Defs/… → 1.6/Languages/<lang>/DefInjected/…`) зашиты в LLM-промпт — структура выходных путей определяется моделью. Хрупко; подтверждает наш dict-подход.
- **VERDICT**: **non-goal**; анти-паттерн задокументирован.

### 3.18 Rimworld-Mod-Translator — kelvinauta

- **IDENTITY**: JS/Node (axios+xml2js), лицензии НЕТ, один коммит 2024-06-25 («Core: first commit», SHA `9eb39d4…`), 4★. Сегодня переподтверждено w3-cs: `index.js` 97 строк, DeepL-only (`https://api-free.deepl.com/v2/translate`, `index.js:13`), 3 тега (`index.js:52-57`), захардкоженные каталоги (`:87-88`).
- **SOURCE REVIEW**: ev. **2**. Статус: **NOT_MATERIALLY_RELEVANT**.
- **PRACTICAL RUN**: NOT DONE (не за чем; DeepL-ключ = BLOCKED_DEPENDENCY по ограничению №3).
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: нет.
- **VERDICT**: **non-goal**, мёртв день-в-день.

### 3.19 rimworld-autonomous-translator — AutonomoAI

- **IDENTITY**: кода в репо нет (README, INVESTORS.md, картинки; ветка trunk), последний коммит 2026-01-14. Закрытая платформа; публичный репо — маркетинг.
- **SOURCE REVIEW**: ev. **1**. Статус: **DOC_ONLY**. Заявлено (непроверяемо): self-correcting пайплайн XML/keyed/C#, zero-trust валидация; кейс «Core+DLC 124 996 слов → играбельная арабская за $8.86, 9.37 ч».
- **PRACTICAL RUN**: NOT DONE — нечего запускать. Попутное подтверждение discovery 2026-10-05: `BetterRimworlds/Rimworld-Urdu` («Rimworld translated into Urdu via Autonomo AI», C#, push 2026-04-05) — их пайплайн реально штампует контент-репо.
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: как инструмент — ничего; как маркер рынка — готовый маркетинговый нарратив «полная локализация с нуля за копейки», который стоит перехватить с честными цифрами.
- **VERDICT**: **non-goal** как тул; ориентир по амбиции качества и маркетингу.

### 3.20 RimTrans_PY — masakitenchi — NEW (discovery 2026-10-05)

- **IDENTITY**: masakitenchi/RimTrans_PY, Python (lxml + regex), **MIT** (LICENSE прочитан сегодня: «Copyright (c) 2023-24 Manifold Paradox»), создан 2024-03-18, последний push **2024-08-22** (спит ~2 года), 3★, китайский сегмент (README-zh_cn).
- **SOURCE REVIEW**: ev. **2** — сегодня прочитаны: `src/Rimtrans_py/TranslationExtractor.py` и `src/Rimtrans_py/ModLoadFolder.py`. Ключевое: xpath-regex `Defs/<defType>[defName="X"]/<field>`; извлечение **полных дефов из PatchOperationAdd** через `anomaly_xpath` (`//*[@Class="PatchOperationAdd"]/xpath[text()="Defs"]/../value/*[not(@Abstract)]`) с фильтром абстрактных; теги label/labelNoun/description/jobString/labelShort; list-контейнеры stages/lifeStages/tools/degreeDatas; `ModLoadFolder.py` — dataclass Loadfolders с **IfActive/IfNotActive** и версиями 1.0–1.5; XmlInheritanceResolver.py. Статус: **SOURCE_CONFIRMED_ONLY**.
- **PRACTICAL RUN**: NOT DONE (мёртв 2 года; не входил в wave 3). **Feasible** (чистый Python) — низкий приоритет.
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: независимое (третье после NicoriciN89 и Text Grabber) подтверждение, что патч-линия — канонический класс задачи, и что **IfModActive должен жить в нашем LoadFolders-слое** (у нас IfModActive = LEVEL 3, у них — в модели папок).
- **VERDICT**: не конкурент (спит, GUI WIP), но дизайн-референс для MUST_FIX патч-слоя: «полные `<value>`-дефы из PatchOperationAdd с фильтром abstract» — та же семантика, что у TG/N89.

### 3.21 rimworld-rtl-translation-tools — mtimoustafa — NEW (discovery 2026-10-05)

- **IDENTITY**: Ruby, лицензии нет (файла LICENSE в дереве не видно; API license:null), создан 2019-01-26, последний push 2024-05-05, 3★. Скрипты `contextualize_arabic_letters.rb`, `reverse_rtl_text.rb`, `build_arabic.sh`.
- **SOURCE REVIEW**: ev. **1** (дерево + имена скриптов; тела не читались). Статус: **DOC_ONLY**.
- **PRACTICAL RUN**: NOT DONE (ниша узкая, без лицензии).
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: закрывает чужой пробел, не наш пробел extraction: пост-обработка RTL (контекстуализация арабских букв, реверс). У RimLoc Arabic/RTL = LEVEL 1 (locale metadata, не shaping) — при заходе в RTL нишу вернуться к этому классу задач.
- **VERDICT**: **non-goal** сейчас; заметка в RTL-роадмап.

### 3.22 RimWorldTranslationTool — lenhare — NEW (discovery 2026-10-05)

- **IDENTITY**: Python (`translate.py`, requirements), создан и запушен в один день 2024-07-02, 1★, лицензии нет (API). В репо **закоммичен `google_credentials.json`** — утечка секрета в публичной истории.
- **SOURCE REVIEW**: ev. **1** (дерево; тела не читались). Статус: **NOT_MATERIALLY_RELEVANT**.
- **PRACTICAL RUN**: NOT DONE (мёртв; Google Cloud Translate = BLOCKED_DEPENDENCY по ограничению №3).
- **SAME-CORPUS RUN**: NOT DONE.
- **GAPS FOUND**: анти-паттерн (секреты в репо) — противопоставление нашему Правилу 3.1 / Keychain-дисциплине.
- **VERDICT**: **non-goal**.

### 3.23 rwmt (RimWorld Multiplayer) — ложное срабатывание

- **IDENTITY**: rwmt/Multiplayer (Zetrith, 671★, MIT) + rwmt/Multiplayer-Locale (контент-переводы самого мода). К локализации модов отношения не имеет.
- **SOURCE REVIEW**: ev. **2** (wave 1, метаданные org). Статус: **NOT_MATERIALLY_RELEVANT**.
- **PRACTICAL RUN / SAME-CORPUS**: NOT DONE — не тул.
- **GAPS FOUND**: нет. **VERDICT**: исключён из суперсет-анализа; строка оставлена для неповторения ложного срабатывания discovery.

### 3.24 Ludeon official workflow (контекст)

- **IDENTITY**: `Ludeon/RimWorld-<Language>` (Dutch, Swedish, …) — DefInjected XML руками + PR; отдельного официального тулза нет. Вне GitHub: «Mod translation tool by Kkokoros» (Steam Workshop, in-game, закрыт).
- **SOURCE REVIEW**: ev. **1**. Статус: **NOT_MATERIALLY_RELEVANT** (как тул; как ниша — контекст).
- **PRACTICAL RUN / SAME-CORPUS**: NOT DONE — не тул.
- **GAPS FOUND**: ниша «официальный локализатор для перевод-команд» свободна — позиционирование для RimLoc (PO/XLIFF для CAT-профессионалов + сборка мода).
- **VERDICT**: не конкурент; рамка позиционирования.

---

## 4. Кросс-матричный синтез: что делать RimLoc

### MUST_FIX_BEFORE_BETA (подтверждено ≥2 независимыми прогонами)
1. **Патч-слой пуст.** На 3170653412 RimLoc извлекает из 25 патч-файлов **ноль** (learn-patches=0; scan --with-patches Δ=0 — w3-py, воспроизведено эталоном w3-cs), при том что NicoriciN89 достаёт 100 refs (54 игроку-видимых), Text Grabber — 163 полных дефа, RimTrans_PY демонстрирует ту же семантику. Брать: полные значения PatchOperationAdd/Replace/Insert + **обязательный полевой blacklist** (bodyType*, spawnCategories, requiredWorkTags, backstoryFilters* — урок 46% шума N89).
2. **Разметка в msgid.** `&lt;b&gt;{0}&lt;/b&gt;…` → RimLoc отдаёт `b{0}/b…` (4 ключа HugsLib). Найдено независимо тремя лейнами: tg (wave 2), RimTranslate (w3-py), RWAT+RimTrans-zh (w3-cs, где оба конкурента возвращают корректный текст). Ломает PO-раундтрип и доверие к экспорту.

### HIGH_VALUE_AFTER_BETA
3. **Defs вне version-папок и дыры словаря**: дефолтный промах KeyBinding-Defs HugsLib (чинится `--defs-dir`, проверено 61 записью — w3-cs); `all_fields`/DEFAULT_FIELDS построены, но не используются (`crates/rimloc-parsers-xml/src/lib.rs:1501-1505` — RimLangKit-лейн); словарь добить данными из TG-списка (~90 строк: WorkGiverDef gerund/verb, TrainableDef, MentalStateDef beginLetter/recoveryMessage, StatDef formatString…) и MTT-списка 38 полей.
4. **Мост learn-defs → export-po**: KeyBindingDef.label (+7 HugsLib), HediffDef.labelNoun (+2 VE) доходят до PO у конкурентов, у нас — только отчёт (w3-py).
5. **Индексированные стадии** `stages.0/1/2.label` вместо схлопнутого `stages.li.label` (tg: 19 строк на VE).
6. **Фильтр мусор-кандидатов**: `'-1'`, `'{0}: {1}'` и RGB-цвета в union/fuzzy-выдаче (6 + ~24 записей; RWAT и laskinss27 показывают фильтры — w3-cs/w3-py).
7. **Баги RimLoc**: `is_version_directory` считает числовой workshop-id версией (`version.rs:8-30`, probe-подтверждено); first-file-wins в union-режиме берёт текст старейшей версии вместо актуальной (1.4 против 1.6, tg).

### ROADMAP (не бета)
8. Workflow сопровождения языкового пакета — check/stale/import (Translation Forge).
9. TM-транзакционность: multi-target snapshot-журнал, ConcurrentPaths-детекция, recovery-сессия (RWAT `FileTransaction`/`FileSnapshotJournal`).
10. MT-роутер + model arena + чекпойнты проекта (Remis) — рамка суперсета; слот «оффлайн-провайдер» (Argos/Ollama) подтверждён N89.
11. Слоистый глоссарий Mod > RimWorld > General с запрещёнными вариантами (MTT).
12. Маркетинговый нарратив «полная локализация за копейки» — перехватить с честными цифрами (AutonomoAI: $8.86 / 125k слов).

### INTENTIONAL_NON_GOAL
CSV/Sheets-конвейер (laskinss27), спящий PO-скрипт как конвейер (RimTranslate), WPF/Avalonia-редакторы таблиц (TokcDK), JP-XLSX (etejasdgjjjj532), DeepL-скрипты (kelvinauta, lenhare), LLM-промпт-путевание (JalapenoLabs), маркетинг-репо (AutonomoAI), rwmt, корейская ниша RWAT/RMK, EN→PL-ниша MTT, морфология через внешний платный сервис (RimLangKit/Morpher).

### RimLoc уже сильнее (подтверждено прогонами)
CLI/JSON/schema_version-детерминизм · LoadFolders+IfModActive effective-view · все языки/версии флагами · существующий English-DefInjected как источник · ParentName-наследование · отсутствие фабрикаций и substring-FP · PO/XLIFF для CAT · Rust-кроссплатформенность · Runtime Bridge (единственный IN_GAME_PROVEN=7 в выборке).

---

## 5. Discovery 2026-10-05 (свежий поиск)

Команды: `gh search repos "rimworld translation" --sort updated --limit 25`, `gh search repos "rimworld translator"`, `gh search repos "rimworld мод перевод"`, `gh search repos "rimworld локализация модов"` (+ `gh api repos/...` по кандидатурам). 

- **Добавлены строками**: RimTrans_PY (3.20), rimworld-rtl-translation-tools (3.21), RimWorldTranslationTool (3.22).
- **Footnote (не тула)**: `BetterRimworlds/Rimworld-Urdu` — контент-репо «via Autonomo AI» (подтверждение активности их закрытого пайплайна); `RanMd/Rimworld-Translations`, `wigravy/Rimworld-translation-by-wigravy` — контент-репо; `miryeon/RimworldTranslator` (2024-07, спит), `cappie/rwt` (PHP, 2015), `Raschert0/Better-Rimworld-Translator` (C++, 2016), `Senpasi/rimxml2csv` (2024), `mtimoustafa/archotome` (QoL Ruby, 2024), `aiscy/RimworldTranslationHelper` (archived) — шум, материальной релевантности нет.
- **Расхождения метаданных разрешены**: API-детект лицензий GitHub сегодня отдаёт `null` для Grabber GUI / RimTransAI / JalapenoLabs / RimTrans_PY / TokcDK — файлы LICENSE прочитаны содержимо (MIT / GPL-3.0 / MIT / MIT; TokcDK — GPL-3.0 по wave 1), что и зафиксировано в матрице.

---

## 6. Ограничения реконсиляции (честные)

1. MT-стадии нигде не прогонялись: их нет в forge/RimTranslate/laskinss27 (ручной шаг пользователя); Argos (N89) — вне объёма ask; DeepL/Google/OpenAI — BLOCKED_DEPENDENCY по ограничению №3; LibreTranslate-сервер не поднимался.
2. Same-corpus NOT DONE для 7 feasible-тулов (Remis, Grabber GUI, RimTransAI, RimTrans_PY, etejasdgjjjj532, TokcDK, rtl-tools) — из-за границ wave 3, не из-за блокировок; это главный остаток очереди.
3. GUI-продукты (WinForms/Avalonia/WPF/PySide) не запускались штатно нигде — везде ядро через харнесс/драйвер или статический разбор; оценки покрытия относятся к ядрам.
4. Полнота 7 (IN_GAME_PROVEN) никому из конкурентов не присвоена и присвоена быть не может в этом стенде; Runtime Bridge RimLoc остаётся единственным уровнем 7 в сравнении.
5. Диффы считались на нормализованных (ключ, значение) с NFC+схлопыванием пробелов; RimLoc-сторона фильтровалась до English-подмножества в w3-cs и до export-po-знаменателя в w3-py — знаменатели в строках указаны.
6. Реконсиляция не запускала новых прогонов конкурентов — валидировала реквизит эталона (SHA, scan-идентичность, наличие бинаря) и свела уже выполненные прогоны; все числа цитируются из wave-отчётов с путями к JSON.

## 7. Артефакты

- Wave 1: `COMPETITOR_DEEP_DIVE_2026-10.md`, `competitors.json` (ev.0-2, source-инспекция).
- Wave 2: `TEXT_GRABBER_VS_RIMLOC.md` (ev.5), `RIMLANGKIT_VS_RIMLOC.md` (ev.6), `differential/tgdiff-*.json`, `differential/text-grabber-diff-summary.json`, `differential/rlk-diff-*.json`.
- Wave 3 (скопировано в `differential/`): `TIERA_PYTHON_PRACTICAL.md` + `tiera-python-diff.json`; `TIERA_CS_PRACTICAL.md` + `diff-rwat.json` + `diff-rimtrans-zh.json`.
- Мастер-лист: `COMPETITOR_MASTER_LIST_2026-10.md` (секция «Реконсиляция 2026-10-05»).
- Эталон: `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (da2fc77-линия; scan-путь идентичен HEAD `70fa342`).
