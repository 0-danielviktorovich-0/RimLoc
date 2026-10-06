# TIER A — финальная матрица завершённости конкурентной очереди

Дата: 2026-10-05 · **Финальная реконсиляция: 2026-10-06 (wave 6, lane reconcile)** · Реконсиляция входов: wave 1 (source-инспекция, `COMPETITOR_DEEP_DIVE_2026-10.md` + `competitors.json`), wave 2 (same-corpus прогоны Text Grabber и RimLangKit), wave 3 (practical прогоны Python- и C#/PowerShell/Node-конкурентов), свежий discovery 2026-10-05, **wave 6 (practical прогоны остатка семёрки: Remis, Grabber GUI, RimTransAI, RimTrans_PY, etejasdgjjjj532, TokcDK, rtl-tools — `/tmp/w6-*/`, отчёты в `differential/`)**.

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
- **Добавлено реконсиляцией 2026-10-06 (wave 6).** Эталон жив: `ls /Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/` → `rimloc-cli` (14 282 640 байт, 2026-10-05 20:09), версия 0.1.0-alpha.1. **Все same-corpus диффы wave 2/3/6 считались от этого бинаря — внутренне согласованы.**
- **⚠️ «scan-поведение идентично текущему» больше НЕ действует для свежего HEAD.** Верифицировано реконсиляцией прогоном обоих бинарей на корпусе (эталон 72259e0b vs сборка HEAD-линии `64658131` из `/Volumes/Portable-SSD/caches/targets/w6-pysmall/release/rimloc-cli`, источник `646581311dbbc143`; текущий main `bb402cb` — docs-only поверх `6465813`, `crates/` не трогает, так что код идентичен): scan VE `2023507013` — **764 → 143** юнитов, причём сборка 6465813 приписывает записи путям `1.6/Languages/English/...`, из которых на диске существуют **3 из 143** (`1.6/` содержит только Assemblies/Defs/Patches), эталон — реальным `Languages/...` (**593 из 764** существуют); export-po VE — **765 → 141** msgid, HugsLib `818773962` — **76 → 1** msgid (сходимо с наблюдением py-small: 163→3 на 317, 41→40 на 324 — реконсилятором не перепрогонялось). Диапазон `72259e0b..6465813` содержит патч-слой (`31bed4f feat(parsers): extract игроку-видимых значений из PatchOperations`) и рефактор сепараторов путей (`db3c346 fix(parsers): пути со смешанными / и \ — `has_path_marker` заменил `contains("/Languages/")`-проверки в scan-коллекторах parsers-xml/cli). Классификация: **регрессия атрибуции путей/версий, не редизайн** → передано RimLoc-лейну (MUST_FIX §4.4); в ворктree уже идёт bisect диапазона (stash «wip-during-bisect», reflog 19:15). До фикса конкурентные диффы валидны относительно эталона, патч-слой валиден в 6465813.
- **Патч-слой HEAD подтверждён прогоном реконсиляции**: `learn-patches --mod-root /tmp/w6-pysmall/mods/3170653412 --game-version 1.5_1.6` (бинарь 6465813) → **54 записи** (tag_path: description 26, title 11, baseDesc 6, titleShort 5, titleShortFemale 4, titleFemale 2; 51 `PatchOperationReplace` + 3 `PatchOperationAdd`) — состав в точности равен игроку-видимому набору NicoriciN89 (wave 3), шума 0. JSON: `/tmp/w6-reconcile/lp-head.json`.

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
| 12 | Remis | Drlinglong | Python PySide + FastAPI-слой | AGPL-3.0 | 2026-10-05 (v3.2.2) | PRACTICALLY_RUN (ядро-адаптер headless; GUI не запускался) | 6 | DONE (4 мода) | **стратегический компаратор** (подтверждён) |
| 13 | RW Translator Grabber GUI | doktorravlik-svg | Python 3 + ttkbootstrap | MIT (файл LICENSE, проверен) | 2026-09-23 (активен) | PRACTICALLY_RUN (ядро headless; GUI не запускался; MT=BLOCKED_DEPENDENCY) | 6 | DONE (4 мода) | ParentName-резолв взять; морфология non-goal |
| 14 | RimTransAI | mmjio-xy | C# net9.0 Avalonia + Mono.Cecil | GPL-3.0 (файл LICENSE, проверен) | 2026-09-02 (v2.0.0) | PRACTICALLY_RUN (сборка + extraction-ядро; LLM=BLOCKED_DEPENDENCY; GUI-запуск не проверен) | 6 | DONE (4 мода) | reflection-анализ — уникален, ROADMAP |
| 15 | RimWorldModTranslator (JP) | etejasdgjjjj532 | Python | MIT | 2026-08-18 | PRACTICALLY_RUN (ядро) | 6 | DONE (Keyed/Defs-слой) | non-goal подтверждён (заглушки+баги) |
| 16 | RimworldModTranslator | TokcDK | C# WPF | GPL-3.0 | 2025-04-27 (спит) | PRACTICALLY_RUN (ядро reflection-харнессом; GUI=BLOCKED_PLATFORM) | 6 | DONE (4 мода + roundtrip записи) | non-goal как редактор; dict-gap донор |
| 17 | rimworld-mod-llm-auto-translator | JalapenoLabs | TypeScript | MIT | 2025-08-05 (заброшен) | BLOCKED_DEPENDENCY (OpenAI-ключ — единственная функция) | 2 | NOT DONE | non-goal |
| 18 | Rimworld-Mod-Translator | kelvinauta | JS/Node | **нет** | 2024-06-25 (1 день жизни) | NOT_MATERIALLY_RELEVANT | 2 | NOT DONE (не за чем: 97 строк DeepL-only) | non-goal |
| 19 | rimworld-autonomous-translator | AutonomoAI | — (кода нет) | — | 2026-01-14 | DOC_ONLY | 1 | NOT DONE (нечего запускать) | маркетинг-репо; нарратив |
| 20 | RimTrans_PY | masakitenchi (Manifold Paradox) | Python | MIT (файл LICENSE, прочитан) | 2024-08-22 | PRACTICALLY_RUN (ядро headless; GUI tkinter недоступен) | 6 | DONE (фокус-дифф патч-линии) | перекрыт новым патч-слоем RimLoc |
| 21 | rimworld-rtl-translation-tools | mtimoustafa | Ruby | **нет** | 2024-05-05 | PRACTICALLY_RUN (оба скрипта исполнены) | 4 | N/A (в корпусе 0 RTL-строк; прогон на родном тесте + LanguageData с ключами корпуса) | RTL-публикация — ROADMAP |
| 22 | RimWorldTranslationTool | lenhare | Python | нет (API) | 2024-07-02 (1 день) | NOT_MATERIALLY_RELEVANT | 1 | NOT DONE | анти-паттерн секретов — NEW |
| 23 | rwmt (Multiplayer) | rwmt org | C# | MIT | 2026-08-03 | NOT_MATERIALLY_RELEVANT | 2 | NOT DONE | ложное срабатывание |
| 24 | Ludeon official workflow | Ludeon | данные+PR | — | активны | NOT_MATERIALLY_RELEVANT (как тул) | 1 | NOT DONE | контекст-ниша RimLoc |

### 2. Сводка завершённости (после wave 6, 2026-10-06)

- **Строк в матрице: 24** (16 wave-1 записей − rwmt-омонимы и Ludeon-контекст оставлены строками, + RimLangKit/Text Grabber из wave 2, + 4 разбиения RimTrans-линии, + 3 новых discovery).
- **PRACTICALLY_RUN: 15** — Text Grabber, RimLangKit, RimTrans-zh, RimTranslate, Translation Forge, laskinss27, NicoriciN89, RimWorldAiTranslator (wave 2–3) + **Remis, Grabber GUI, RimTransAI, RimTrans_PY, etejasdgjjjj532, TokcDK, rtl-tools (wave 6)**.
- **SAME-CORPUS дифф получен для 14 из 24**; rtl-tools — прогон есть, same-corpus **N/A по природе** (в корпусе 0 RTL-строк, проверено сканом; инструмент — пост-процессор готовых переводов, не экстрактор).
- **NOT DONE: 9** — feasible-остаток семёрки исчерпан; всё оставшееся — блокировки или отсутствие материи:
  - **3 BLOCKED_PLATFORM**: Aironsoft (net461+WinForms + Assembly-CSharp.dll по Windows-пути), inkitter (net45 WinForms), MTT (pwsh+WPF отсутствуют на macOS);
  - **1 BLOCKED_DEPENDENCY**: JalapenoLabs (OpenAI-ключ — вся суть тулза; ограничение №3); сюда же MT-стадии wave 6: Grabber (deep-translator/googletrans) и RimTransAI (OpenAI/LLamaSharp);
  - **1 DOC_ONLY без кода**: AutonomoAI (маркетинг-репо, нечего запускать);
  - **4 не за чем / не тул**: kelvinauta, lenhare, rwmt, Ludeon workflow.
- **IDENTITY_UNRESOLVED: 0** — все линии идентифицированы, включая третий «RimTrans» (inkitter — независимый однофамилец) и Aironsoft (форк старой C#-линии duduluu, а не самостоятельный наследник).
- **Открытый пункт по самому RimLoc (не конкурентный)**: регрессия линии `6465813` относительно эталона `72259e0b` вне патч-слоя (scan/export-po, §0 и §4.4) — передана RimLoc-лейну, bisect уже идёт.

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

- **IDENTITY**: Drlinglong/Remis, Python (PySide-десктоп + SQLite + агент-слой), AGPL-3.0, 26★, frozen HEAD `5440918` (v3.2.2, 2026-10-05 — свежее, чем видела волна 1: тогда v3.2.1/2026-09-26), 2295 файлов. Мультиигровая платформа (Paradox — фокус; RimWorld в preview). Отчёт: `differential/REMIS_PRACTICAL_2026-10-06.md`.
- **SOURCE REVIEW**: ev. **6** SAME_CORPUS_DIFFERENTIAL — RimWorld-адаптер `scripts/core/game_adapters/rimworld.py` (439 строк; `FORMAT_RULES_VERSION = "rimworld-1.6-v1"`, жёсткий каталог `_V16_DEF_FIELDS`, `rimworld.py:13-23`), контракт `GameAdapter` из 7 методов (`contracts.py:63-82`), реестр — два структурных адаптера (`registry.py:6-9`). Статус: **PRACTICALLY_RUN** (ядро — чистая библиотека, GUI не нужен и не запускался).
- **PRACTICAL RUN**: DONE — headless-драйвер (`run_adapter.py`: discover→parse напрямую; SHA и команды в `/tmp/w6-remis/NOTES.md`) на всех 4 модах, версии 1.6 и auto (инференс версии, `rimworld.py:406-418`; результаты идентичны). Ресурсы/строки: 2/40 · 3/82 · 67/859 · 1/40; in-process 2.2–37.8 мс (RimLoc 0.07–0.26 с wall — уровни измерения разные, сравнивать порядок). MT-стадии для извлечения не требовались.
- **SAME-CORPUS RUN**: DONE (4 корзины, нормализация {key, source}). Суммарно: **BOTH 839 · SEMANTIC 13 · RIMLOC_ONLY 186 · REMIS_ONLY 160** (по модам: BOTH 40/71/688/40; SEM 0/4/9/0; R_ONLY 119/0/67/0; M_ONLY 0/7/153/0). JSON: `differential/remis-diff-<id>.json` ×4.
- **GAPS FOUND (RimLoc)**: (а) **слияние версионных папок дефолтом** — на plain versioned-модах (VE) RimLoc сливает 1.0–1.6: 81 юнит из файлов, которых в 1.6 нет; stale-значения вместо актуальных (`VEF_VerbRangeFactor.label`: RimLoc `verb range factor` из 1.3/1.4, в 1.6 — `weapon range factor`, по диску прав Remis); `--game-version 1.3`, `--game-version 1.6`, auto, `--include-all-versions` — побайтово одинаковые 764 юнита (флаг не влияет); (б) **IfModActive игнорируется** — Royalty-контент 3170653412 (36 строк) включён безусловно (Remis резолвит по `active_mods`, RimTransAI — по ActivePackageIds); (в) `--with-patches` на эталоне — молча 0 на моде с 53 переводимыми тегами патчей, включая однозначный `PatchOperationReplace` с xpath-литералом defName (`BackstoriesDef.xml:44-51`) — в линии 6465813 закрыто learn-patches=54 (§0); (г) **деградация источника в Keyed**: `<b>X</b>` → `bX/b`, standalone `>` выброшен, хвостовой пробел триммится (`Search: ` → `Search:` — значим для конкатенации; SEMANTIC 4 на HugsLib — span-сохраняющий парс Remis точнее); (д) REMIS_ONLY 153 на VE — gerund/verb WorkGiverDef (22), rulesStrings-индексы (~75), stages/gizmo (~56): добирается нашими `--defs-field/--defs-dict`, но дефолт уже; +7 KeyBindingDef.label HugsLib.
- **VERDICT**: **стратегический компаратор (threat HIGH), прогон подтвердил статус**: аккуратный span-сохраняющий движок с контрактом 7 методов, верифицируемой записью (обратный парс рендера + атомарная запись с rollback + manifest provenance, `workflow_bridge.py:104-203`), честной диагностикой неумеек (Patches: «offline extraction cannot resolve», `rimworld.py:243-244`; dll: `assembly_strings_unavailable`). Слабости конкурента: зашитый каталог полей (теряет `title*`×119 и `reportString`/`deathMessage`×67 — наш словарь шире), патчи 0, симлинк-баг LoadFolders на macOS (`.resolve()` в `_effective_roots` `rimworld.py:424` против нерезолвнутого root `:385-386` → ValueError, воспроизведён), `IfModActive` без списка активных модов валит scan целиком (`workflow_bridge.py:23-25`). Перенять: **пофайловый атомарный чекпойнт перевода** (schema v3: identity+config_fingerprint+source_snapshot_hash, resume с проверкой совместимости) — HIGH_VALUE; **glossary-health score + advisory AI** — ROADMAP; одномодальный auto-reuse переводов — не TM (наш TM шире). Классификации лейна: «жёсткий выбор одной версии» → наш MUST_FIX (§4.2); «статическая резолвка однозначных PatchOperationReplace/Add» → HIGH_VALUE, **уже закрыто в линии 6465813** (learn-patches); «зашитый каталог» → INTENTIONAL_NON_GOAL (наши словари гибче).

### 3.13 RW Translator Grabber GUI — doktorravlik-svg/RimWorld-Translator-Grabber-GUI

- **IDENTITY**: Python 3 + ttkbootstrap (Tk) + lxml + loguru + rapidfuzz + **pymorphy3** (морфология ru/uk) + deep-translator/googletrans (MT-fallback-цепочка подтверждена requirements.txt), **MIT** («Copyright (c) 2026 RimWorld Translator Team»; API-детект GitHub отдаёт null — разрешено в пользу файла), SHA `35b56bb`, push 2026-09-23, активен, русскоязычный. В репо закоммичены рабочие артефакты (debug.log 1.5 МБ, `translation_anchors.db`, Windows-venv `env/`). Отчёт: `differential/TIERA_GRABBER_RTAI_PRACTICAL.md` §1.
- **SOURCE REVIEW**: ev. **6** — ядро прочитано и исполнено: `collect_defs_full` — рекурсивный поиск всех элементов с `defName` + резолв наследования ParentName/Name (`utils/parent_resolver.py`) + применение патчей в индекс строк (`utils/patch_processor.py`: Add/Replace/Remove/Name/sequence); поле-модель — whitelist 60 тегов ∪ partial-матчи (Message/Label/Title/gerund…) ∪ **space-fallback** / blacklist 20 тегов + 14 паттернов (regex-подобные ищутся как подстроки и не срабатывают никогда — мёртвый код; `collectors.py:125-220`, `rimworld_xml.py:90-276`); `loadfolders_parser.py` — universal-скан глубины 2 + рекурсивный fallback, **IfModActive не разбирается**, `_detect_version` на `1.5_1.6` возвращает 1.4. Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: DONE — GUI-независимое ядро headless-драйвером `grabber_run.py` (venv: lxml/loguru/pymorphy3 + словари ru/uk) на всех 4 модах: Defs-поля ALL/NEWEST 213/5 · 7/7 · 306/264 · 0/0; Keyed EN 0/75/593/40. MT-стадии (deep-translator/googletrans) — **BLOCKED_DEPENDENCY**, не запускались. GUI не запускался — Tk кроссплатформенен, BLOCKED_PLATFORM неприменим; честное «не прогонялся».
- **SAME-CORPUS RUN**: DONE. Keyed HugsLib: **BOTH 75 / SEM 7** — полный паритет. Keyed VE: BOTH 593 / RIMLOC_ONLY 171 (существующие DefInjected-папки — конкурент читает только Keyed+Defs). Defs HugsLib: BOTH 7 — тройной паритет RimLoc learn-defs = Grabber = RimTransAI. Defs 3170653412: BOTH 126 / R_ONLY 36 (IfModActive-папка `Mods/Royalty` не активируется) / C_ONLY 87 (titleFemale/gerund… за счёт partial-матчей). Defs VE 1.6: BOTH 171 / **C_ONLY 62 — строгий суперсет словаря RimLoc на 1.6**. SEMANTIC 32, из них 25 — корректное PO-экранирование RimLoc, 1 — реальная порча разметки msgid (воспроизведена на tgbench-бинарнике), остальные — различия текстов 1.5/1.6 у Anomaly. JSON: `differential/tiera-grabber-rtai-diff.json`.
- **GAPS FOUND (RimLoc)**: независимое подтверждение дефекта msgid-разметки; PO-экранирование `\n`/`\"` — не дефект (корректный PO). От конкурента: **ParentName/Name-резолв при Defs-извлечении** (у learn-defs RimLoc нет; в основном извлечении RimLoc наследование есть — см. 3.16).
- **VERDICT**: жизнеспособный русскоязычный инструмент сопровождения переводов с MT-цепочкой; extraction-ядро — не competitor нашему. Перенять: ParentName-резолв в learn-defs (**HIGH_VALUE_AFTER_BETA**); расширение словаря из C_ONLY 62 (gerund/verb/inspectString — ROADMAP); pymorphy3 — **INTENTIONAL_NON_GOAL** (у RimLoc `morph`/pymorphy2); скан всех версий сразу — non-goal (`--include-all-versions` есть). Его критичные дыры: авто-версия ломается на `1.5_1.6` («newest» = 1.4 → 5 полей вместо 213 — молча устаревший контент), IfModActive не поддержан, дубль-политика без дедупликации текстов между версиями, DLL-извлечения нет, мёртвый blacklist-код. Anchors/TM-база — NOT_MATERIALLY_RELEVANT для extraction-лейна (TM — волна 3).

### 3.14 RimTransAI — mmjio-xy/RimTransAI

- **IDENTITY**: C# **net9.0-generic** (не `net9.0-windows`!) + Avalonia 12.1 (Semi.Avalonia) + Mono.Cecil 0.11.6 + OpenAI SDK + MiniExcel + Serilog; сателлит `RimTransAI.LocalTranslator` — LLamaSharp 0.27 (CPU), **GPL-3.0**, SHA `fb5d0d08` (v2.0.0, 2026-09-02), активен, китайский сегмент. Уточнение identity: канонический репо — `mmjio-xy/RimTransAI` (мастер-лист wave 1 ждал `RimTransAI/RimTransAI`); `global.json` пинит SDK 9.0.315 (на машине 10.0.401). Отчёт: `differential/TIERA_GRABBER_RTAI_PRACTICAL.md` §2.
- **SOURCE REVIEW**: ev. **6** — сборка `dotnet build -c Release` из внешнего cwd (обход global.json; `DOTNET_ROLL_FORWARD` не влияет) → **0 ошибок, 0 предупреждений**. Заявление «Avalonia → BLOCKED_PLATFORM» **не подтвердилось**: проект generic-net9.0 и компилируется на macOS; запуск GUI не проверялся. Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: DONE — extraction-ядро через **продуктовый путь** `ModParserService.ScanModFolder` (тот же, что `MainWindowViewModel.cs:82-88`) харнессом net10.0 с ProjectReference (`/tmp/w6-grabber-rtai/rtai-harness/`) на всех 4 модах. Items: 82 · **2209** · 53 · 40; Mono.Cecil-типы из модовых DLL: +1 (`HugsLib.UpdateFeatureDef`) / **+192** (`VFECore.*`, `KCSG.*`) / 0 / 0. Причины извлечения в данных (VE): Whitelist 678, **ReflectionField 712**, SmartSuffix 76, ListItem 150; LoadFolders=8. LLM/MT (OpenAI-совместимый, LLamaSharp) — **BLOCKED_DEPENDENCY**. Оговорка: жёсткий входной барьер `Assembly-CSharp.dll` (`ModParserService.cs:249-253` — без него return null, не сканируется ничего); LoadCore шёл по референсной копии `/Users/danielviktorovich/Developing/compare/RimTrans/Reflection/References/` (только чтение; DLL чужой сборки RimTrans, влияние на состав core-типов не оценивалось).
- **SAME-CORPUS RUN**: DONE. Keyed HugsLib: BOTH 75 / SEM 7. Keyed VE: BOTH 593 / R_ONLY 171 (DefInjected не читает: DefInj=0 во всех сканах). Keyed Anomaly: BOTH 40 / SEM 7 — дедуп версий first-wins: 32 ключа канонизировали **текст 1.5** при наличии 1.6 (приоритета «новейшей» нет). Defs HugsLib: BOTH 7. Defs 3170653412: BOTH 53 / R_ONLY 109 (наш словарь BackstoryDef шире: titleFemale/titleShort…). Defs VE 1.6: BOTH 166 / R_ONLY 5 (`stages.N.label` — не извлекает подписи стадий, только [TranslationCanChangeCount]-списки) / C_ONLY 52 (reflection-поля: gerund/verb/inspectString).
- **GAPS FOUND (RimLoc)**: дефект msgid-разметки подтверждён вторым инструментом лейна независимо; DefInjected-как-источник и дефолт «новейшая версия» — подтверждённые преимущества RimLoc (у RTAI first-wins 1.5).
- **VERDICT**: **Reflection-extraction из C#-сборок — единственный структурный дифференциатор волны: 712 ReflectionField на VE — поля, которых нет ни в одном статическом словаре. Классификация: ROADMAP** (тяжело: DLL игры, резолвер зависимостей, обфускация; ядро закрывают словарь + learn-defs + `--defs-dict`). **IfModActive-резолв в GameLoadOrderPlanner (IfModActive/IfModActiveAll/IfModNotActive по ActivePackageIds) — образцовый среди всех конкурентов**, референс для нашего IfModActive (HIGH_VALUE §4.9). Его дыры: без Assembly-CSharp.dll не сканирует вообще (RimLoc сканирует без игры); папки `v1.6/Assemblies` пропускает (regex `^\d+\.\d+$` не матчит `v`-префикс — у HugsLib спасает root `Assemblies/`); версии first-wins; DefInjected-источники не читает. GUI-запуск Avalonia на macOS — не проверен (сборка проверена).

### 3.15 RimWorldModTranslator — etejasdgjjjj532 (JP)

- **IDENTITY**: Python (`translator.py` 180 строк + `gui.py`), MIT, SHA `7432f47d` (2026-08-18), 1★. Отчёт: `differential/TIERA_PYSMALL_PRACTICAL.md` §2.
- **SOURCE REVIEW**: ev. **6** SAME_CORPUS_DIFFERENTIAL — ядро `extract_all()` исполнено на всех 4 модах (GUI-обёртка не запускалась, tqdm не влияет). Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: DONE — Defs/Keyed/Patches по модам: **0/0/0 · 7/75/0 · 0/593/0 · 0/0/0** (команда воспроизведения в отчёте лейна).
- **SAME-CORPUS RUN**: DONE (Keyed-слой HugsLib, знаменатель export-po 76): **BOTH 75 · COMPETITOR_ONLY 0 · RIMLOC_ONLY 0 · SEMANTIC 7** — все 7 семантических это дефекты экспорта **RimLoc**, не конкурента (он хранит сырой текст).
- **GAPS FOUND (RimLoc)**: очередное независимое подтверждение порчи msgid-разметки: `<b>The HugsLib mod</b>` → `bThe HugsLib mod/b`, ` > ` схлопнут (`Mod Options > All` → `Mod OptionsAll`), литеральный `\\n`; 7 ключей HugsLib.
- **VERDICT**: wave-3 deep-dive завышал возможности, прогон всё исправил: `extract_patches()` — **заглушка**, возвращает `[]` (`translator.py:138-140`, «simplified»), патч-линии нет вопреки README; `export_to_xlsx()` **падает на живом прогоне** (`to_sheet` не существует в pandas 3.0.6 → AttributeError, затем пустая книга → IndexError) — «RimWaldo format XLSX» не производит файла; читает только корневые `mod/Defs`, `Languages/English/Keyed`, `Languages/Japanese` — версионные папки не резолвит (на VE 172 строки RimLoc мимо него), DefInjected-источники не читает вовсе. **INTENTIONAL_NON_GOAL подтверждён прогоном**; ценность строки — независимый голос за MUST_FIX msgid (третий лейн волны 6).

### 3.16 RimworldModTranslator — TokcDK

- **IDENTITY**: C# WPF net8.0-windows (`UseWPF`, `csproj:3-10`), GPL-3.0, 51 .cs, SHA `7752a0d6` (2025-04-27, спит), README RU+EN. Отчёт: `differential/TOKCDK_PRACTICAL.md`.
- **SOURCE REVIEW**: ev. **6** — компиляция всего WPF-проекта на macOS удалась (`EnableWindowsTargeting=true`, `csproj:6`; 0 ошибок / 61 warning, SDK 10.0.401 arm64); ядро изолировано в `Helpers/EditorHelper.cs` (1722 строки), WPF-типы только в DataGrid-методах (`:1277-1339`), которые headless не вызывает. Статус: **PRACTICALLY_RUN** (ядро); GUI-рантайм — **BLOCKED_PLATFORM** (WPF на macOS не стартует; компиляция ≠ запуск).
- **PRACTICAL RUN**: DONE — reflection-харнесс (`/tmp/w6-tocdk/harness/`, net10.0, кастомный AssemblyLoadContext) поверх скомпилированной DLL: `GetTranslatableFolders` → `LoadDefKeyedStringsFromTheDir` → `ExtractStrings` → `CreateTranslationsTable`→`FillTranslationsData`→`WriteFiles` на всех 4 модах (обход GUI-инварианта «первая папка — заглушка `*`», `TranslationEditorViewModel.cs:368` + `EditorHelper.cs:735`). Команда: `DOTNET_ROLL_FORWARD=Major ./harness <корпус> <out> load|write`.
- **SAME-CORPUS RUN**: DONE. Готовые переводы (DefInjected/Keyed-XML + Strings/*.txt, фильтр `XmlReaderBase.cs:19-24`): HugsLib **815==815, обе корзины пусты**; Anomaly 40==40 (+32 дубля v1.5); VE 592==592 (rim_only_ready 1); 317 — 0 (Languages нет). **Roundtrip load→write** (HugsLib): 26 файлов, ключи круговые (Keyed 47/47, DefInjected 3/3), порядок сохранён; отличия косметические (BOM, отступы, комментарии); XML пишется плоскими dot-тегами (`EditorHelper.cs:1007`). Defs-слой: 317 — BOTH 106 / C_ONLY 2 / **R_ONLY 53** (всё семейство titleShort — его case-bug); VE — BOTH 162 / **C_ONLY 75** (JobDef/WorkGiverDef label/verb/labelNoun/baseInspectLine) / R_ONLY 9. Единственная потеря на готовых: многострочный Keyed `VEF.HiringDesc` — построчный regex (`EditorHelper.cs:586`) теряет молча, RimLoc собирает полностью. JSON: `differential/w6-tocdk-diff.json`.
- **GAPS FOUND (RimLoc)**: C_ONLY 75+2 — **dict-gap кандидаты** в `defs_fields.json` (49 типов): WorkGiverDef/JobDef `label`/`verb`/`labelNoun`/`baseInspectLine`, `structureLabel`, `stuffAdjective`, `adjective`, `summary`, `ideoName`, `pawnsPlural`. Словарное сравнение: THEIR-ONLY 26 плоских имён / OUR-ONLY 19 (наш typed-словарь структурно шире). Уроки-гейты (его дефекты, у нас покрыто прогоном): словарь в точном регистре RimWorld-XML (его `titleshort`/`titleshortFemale`, `EditorHelper.cs:97-99` → 53 потери), многострочные значения, ParentName/Abstract-резолв (его пропуск `Mote_*.label`, `Motes.xml:33-35`).
- **VERDICT**: **non-goal как редактор** (таблица переводов с автосейвом дублирует наш React-workspace), но строка обогатилась прогоном: единственный конкурент, читающий **tar-архивы языков** (SharpCompress, `TarXmlReader`/`TarTxtReader`) — ROADMAP-источник готовых переводов (на корпусе tar нет — ветка не исполнена, честно); читает и **пишет** ModsConfig.xml, генерирует перевод-мод (`<mod>_Translated` + About.xml + LoadFolders.xml, `EditorHelper.cs:1086-1275`). Его дыры: IfModActive игнорирует (`1.5_1.6/Mods/Royalty` ушёл в папки безусловным), Patches не читает вовсе, дубли по версиям в одной таблице (VE ~1300 ключей против наших 764 уникальных), нотация `stages.0.label` против нашего `stages.li.label` (SEMANTIC схемы, содержимое одинаково).

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

### 3.20 RimTrans_PY — masakitenchi

- **IDENTITY**: masakitenchi/RimTrans_PY, Python (lxml + regex), **MIT** («Copyright (c) 2023-24 Manifold Paradox»), SHA `81cca257` (2024-08-22, спит ~2 года), 3★, китайский сегмент. Отчёт: `differential/TIERA_PYSMALL_PRACTICAL.md` §1.
- **SOURCE REVIEW**: ev. **6** SAME_CORPUS_DIFFERENTIAL — ядро исполнено headless-драйвером `rtp_driver.py`, воспроизводящим GUI-метод `Patch_Extract_Tab.do_extract` (`main.py:275-355`) в дефолтном split-режиме (GUI tkinter недоступен: в python@3.14 нет `_tkinter`; прецедент Text Grabber-лейна). Статус: **PRACTICALLY_RUN**.
- **PRACTICAL RUN**: DONE — Defs/патчи по модам: **51/26 · 7/0 · 196/0 (540 сырых по 7 версиям) · 0/0**. Live-проверка `ModLoadFolder.py` на живом LoadFolders.xml: v1.4/v1.5/v1.6 парсятся, `IfModActive="Ludeon.RimWorld.Royalty"` попадает в `Loadfolders.IfActive` (`ModLoadFolder.py:94-101`).
- **SAME-CORPUS RUN**: DONE — фокус-дифф патч-линии на 3170653412 против нового learn-patches (линия 6465813): **1.5_1.6: RimLoc 54 vs RTP 26 — BOTH 26 / COMPETITOR_ONLY 0 / RIMLOC_ONLY 28 / SEMANTIC 0** (все 26 — `BackstoryDef.description`, значения идентичны посимвольно после NFC+пробелов); **1.4: RIMLOC_ONLY 66**. Его 26 ⊂ наших 54. JSON: `differential/TIERA_PYSMALL_DIFF.json`.
- **GAPS FOUND (RimLoc)**: новых нет — **новый патч-слой RimLoc строго перекрывает конкурента** (wave-3 MUST_FIX «патч-слой пуст» закрыт; IfModActive в модели `modview.rs:20,70`, learn-patches берёт файлы условных папок — 4 из 66 на 1.4). Остаток — расхождение HEAD/эталон вне патч-слоя (§0, §4.4).
- **VERDICT**: подтверждён и закрыт — wave-3 дизайн-референс «полные `<value>`-дефы из PatchOperationAdd с фильтром abstract» реализован в линии 6465813 с более широким словарём полей. Дефекты оригинала, найденные прогоном (не ревью): non-split `extract()` теряет всё, кроме последнего файла (`TranslationExtractor.py:272,275` — живое доказательство: 364 файла VE → 1 запись; GUI дефолтно split, поэтому пользователи не бьют); конкатенация дублей ключей `+=` (`:232-244`, латентная порча данных, на корпусе дублей не было); xpath-словарь только `label|description` (`:18-20`) и нет dotall у regex — `PatchOperationAdd` с xpath, разорванным переносами, не берётся (поэтому 0 на 1.4-патчах). **Классификация патч-слоя RimLoc: MUST_FIX_BEFORE_BETA → ВЫПОЛНЕНО.**

### 3.21 rimworld-rtl-translation-tools — mtimoustafa

- **IDENTITY**: Ruby (nokogiri ~> 1.16), лицензии нет (файла LICENSE нет; API license:null), SHA `962a1053` (2024-05-05), 3★. Скрипты `reverse_rtl_text.rb`, `contextualize_arabic_letters.rb`, `build_arabic.sh`. Отчёт: `differential/TIERA_PYSMALL_PRACTICAL.md` §3.
- **SOURCE REVIEW**: ev. **4** INTEGRATION_TESTED — оба скрипта **исполнены** (ruby 2.6.10 системный + nokogiri 1.13.8; `.ruby-version`=3.2.4 не потребовался). Wave-3 DOC_ONLY снят. Статус: **PRACTICALLY_RUN** (CLI-скрипты — и есть их интерфейс).
- **PRACTICAL RUN**: DONE — на родном TestFile.xml репо (15 узлов изменено) и на сконструированном LanguageData с реальными ключами RimLoc-патчей (`VengefulNomad67.title` арабский, baseDesc с `[PAWN_nameDef]` и `{0}`, ивритский title): реверс слов корректен, плейсхолдеры `{0}`/`[PAWN_possessive]` сохранены на местах (офсетный scan/sub/insert, `reverse_rtl_text.rb:29-44`), арабские буквы переведены в контекстные формы презентации, иврит реверсируется. Скрипт перезаписывает вход на месте — только в копиях лейна.
- **SAME-CORPUS RUN**: **N/A по природе** — python-скан всех XML корпуса нашёл **0 файлов** с арабскими/ивритскими символами (U+0600–06FF, U+0590–05FF); инструмент к тому же не экстрактор, а **пост-процессор готовых переводов** (слот «публикация», не «извлечение») — корзины BOTH/RIMLOC_ONLY неприменимы.
- **GAPS FOUND**: нет — с extraction-конвейером не пересекается.
- **VERDICT**: **ROADMAP** — единственный в выборке инструмент с обработкой RTL: реверс + контекстуализация арабского/иврита как опциональный шаг экспорта (после build); защита плейсхолдеров по офсетам — референс реализации для будущего RTL-шага. Без лицензии код копировать нельзя — только дизайн-референс.

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

## 4. Кросс-матричный синтез: что делать RimLoc (после wave 6, 2026-10-06)

### ФИНАЛЬНЫЕ ВЕРДИКТЫ по MUST_FIX (закрыты в main, 2026-10-06, wave 7)

| § | MUST_FIX | Статус | Фикс |
|---|---|---|---|
| 4.1 | msgid-разметка + тримминг | **FIXED** | entity-фикс Keyed-ридера (829273d, PR #63-линия): GeneralRef декодируется, trim на значение; w6-лейны гоняли до-фиксные бинари (tgbench 72259e0b / stale-сборки) — на текущем main HugsLib msgid `<b>…</b>` верифицирован export-po Gate H-агентом и рераном спек |
| 4.2 | Слияние версионных папок дефолтом (stale-значения) | **FIXED** | PR #78: scan classic-модов через effective view (новейшая версия побеждает per-key, --game-version приоритетен); дефолт более не теряет корневой Keyed (VE 143→736 записей, 0 stale, `weapon range factor` из 1.6 верифицирован) |
| 4.3 | IfModActive при извлечении | **HIGH_VALUE остаётся** (over-включение, не потеря; референс RimTransAI GameLoadOrderPlanner) |
| 4.4 | «Регрессия 6465813 vs 72259e0b» | **ПЕРЕКЛАССИФИЦИРОВАНО**: это не регрессия мержей — латентный дефект export-po (gv-сужение без Gate H), вскрытый осознанным фиксом резолвера f3bf643. **FIXED** PR #77 (modview effective_view, регресс-тест export_po_game_version_loadfolders_keeps_root_keyed; A/B 1→76 msgid). Бисект-«зелёные» f3bf643/ca2cedc — артефакт stale-fingerprint таргета (урок §91) |
| 4.5 | Патч-слой | **CLOSED** (54/54, RimTrans_PY 26 ⊂ 54 1:1 — подтверждено волной 5) |

### Урок верификации (§91)
Бисект-прогоны в один CARGO_TARGET_DIR при последовательных чекаутах дают stale-бинарники
(cargo не всегда инвалидирует по mtime) — «зелёный» родитель красного коммита мог просто
не пересобраться. Свежий таргет на коммит; контроль self-report версии бинаря.

### MUST_FIX_BEFORE_BETA
1. **Разметка и тримминг в msgid — единственный живой MUST_FIX extraction-ядра.** `<b>X</b>` → `bX/b`; standalone `>` выброшен (`Mod Options > All` → `Mod OptionsAll`); хвостовой пробел триммится (`Search: ` → `Search:` — в RimWorld значим для конкатенации). Подтверждено теперь **шестью лейнами / восемью инструментами**: tg (w2), RimTranslate (w3-py), RWAT+RimTrans-zh (w3-cs), Remis + Grabber + RimTransAI (w6-grabber-rtai), rimwt (w6-pysmall) — во всех случаях конкурент хранит сырой текст. Ломает PO-раундтрип и доверие к экспорту.
2. **Слияние версионных папок дефолтом (w6-remis).** На plain versioned-модах (VE) эталон 72259e0b сливает 1.0–1.6: 81 юнит из файлов, которых в 1.6 нет; stale-значения вместо актуальных (`verb range factor` из 1.3/1.4 против `weapon range factor` 1.6 — по диску прав конкурент); `--game-version 1.3`, `--game-version 1.6`, auto и `--include-all-versions` дают побайтово одинаковый вывод. Родственно tg-находке w2 (union first-file-wins берёт старейшую версию). Классификация лейна: MUST_FIX.
3. **IfModActive при извлечении игнорируется (w6-remis; референс решения — RimTransAI).** DLC-контент включается безусловно (Royalty-папка 3170653412: +36 строк); RimTransAI корректно не активирует условные папки без ActivePackageIds. Классификация лейна: HIGH_VALUE (over-включение, не потеря), с п.2 — одна версия-семантика.
4. **Регрессия линии `6465813` vs эталона `72259e0b` вне патч-слоя — верифицировано реконсиляцией 2026-10-06.** scan VE 764→143 (сборка 6465813 приписывает записи `1.6/Languages/...`, на диске существуют 3 из 143; эталон — реальные `Languages/...`, 593 из 764); export-po VE 765→141, HugsLib 76→1 msgid (прогон обоих бинарей реконсиляцией; py-small дополнительно: 163→3 на 317, 41→40 на 324 — не перепрогонялось). Кандидаты в диапазоне: `31bed4f` (патч-слой), `db3c346` (`has_path_marker` заменил сепараторные проверки в scan-коллекторах). До фикса «scan идентичен» не действует; → RimLoc-лейн (bisect уже идёт в ворктree).
5. **Патч-слой — ЗАКРЫТ в линии `6465813` (был MUST_FIX w2/w3).** learn-patches даёт **54/54 игроку-видимых** на 3170653412@1.5_1.6 (состав 26/11/6/5/4/2 == набор N89, 51 Replace + 3 Add, шум 0 — верифицировано реконсиляцией), +66 на 1.4, RimTrans_PY 26 ⊂ 54 значения 1:1, SEMANTIC 0. Остатки: выкатить семантику патчей в основной scan-конвейер (на эталоне `--with-patches` = 0 — w6-remis) и починить п.4.

### HIGH_VALUE_AFTER_BETA
6. **Defs-словарь в основном экспортном потоке / мост learn-defs→export-po**: KeyBindingDef.label ×7 HugsLib берут в основной конвейер четверо конкурентов w6 (RimTrans_PY, rimwt, RimTransAI, Grabber), у RimLoc — только learn-defs; REMIS_ONLY 153 на VE (gerund/verb WorkGiverDef 22, rulesStrings-индексы ~75, stages/gizmo ~56) — дефолт уже, добор флагами `--defs-field/--defs-dict`.
7. **Dict-gap словаря по данным корпуса**: TokcDK C_ONLY 75 на VE (JobDef/WorkGiverDef `label`/`verb`/`labelNoun`/`baseInspectLine`), Grabber C_ONLY 62 на VE 1.6 (gerund/verb/inspectString), THEIR-ONLY 26 имён TokcDK — сверить с `defs_fields.json` (49 типов) и дозаполнить обоснованно.
8. **Индексированные стадии/списки**: `stages.0/1/2.label` вместо схлопнутого `stages.li.label` (tg w2: 19 строк на VE; RimTransAI не извлекает подписи стадий вовсе — наш learn-defs отдаёт), rulesStrings-индексы (remis w6: ~75).
9. **IfModActive-резолвка по списку активных модов** — референс RimTransAI GameLoadOrderPlanner (образцовый среди конкурентов) и Remis `active_mods`.
10. **ParentName/Name-резолв в learn-defs** (Grabber имеет; в основном извлечении RimLoc наследование есть — TokcDK Mote_* подтвердил).
11. **Пофайловый атомарный чекпойнт перевода** (Remis: schema v3, identity+config_fingerprint+source_snapshot_hash, resume с проверкой совместимости) — практически подтверждён w6; нашей ProjectSession-модели в таком виде нет.
12. **Фильтр мусор-кандидатов**: `'-1'`, `'{0}: {1}'`, RGB-цвета в union/fuzzy-выдаче (w3; остаётся).

### ROADMAP (не бета)
13. **Reflection-extraction из C#-сборок** (RimTransAI: ReflectionField 712 на VE, +192 типа из модовых DLL — единственный структурный дифференциатор волны; закрывает класс «полей, которых нет в словаре»).
14. **Glossary-health score + advisory AI** (Remis: детерминированный score/100 + suggestions-only AI, никогда не мутирует данные).
15. **RTL-публикация** (rtl-tools): реверс + контекстуализация арабского/иврита шагом экспорта; защита плейсхолдеров по офсетам — референс.
16. **Tar-архивы языков как источник готовых переводов** (TokcDK, единственный; на корпусе ветка не исполнена).
17. Workflow сопровождения языкового пакета — check/stale/import (Translation Forge, w3).
18. TM-транзакционность: multi-target snapshot-журнал, ConcurrentPaths-детекция, recovery-сессия (RWAT w3).
19. MT-роутер + model arena + оффлайн-провайдер (Remis w3/w6, N89 w3) — рамка суперсета PROVIDER-слоя.
20. Слоистый глоссарий Mod > RimWorld > General с запрещёнными вариантами (MTT w3).
21. Маркетинговый нарратив «полная локализация за копейки» — перехватить с честными цифрами (AutonomoAI w3: $8.86 / 125k слов).

### INTENTIONAL_NON_GOAL
CSV/Sheets-конвейер (laskinss27), спящий PO-скрипт как конвейер (RimTranslate), WPF/Avalonia-редакторы таблиц (TokcDK), JP-XLSX (etejasdgjjjj532 — подтверждено прогоном: заглушка патчей, сломанный XLSX), DeepL-скрипты (kelvinauta, lenhare), LLM-промпт-путевание (JalapenoLabs), маркетинг-репо (AutonomoAI), rwmt, корейская ниша RWAT/RMK, EN→PL-ниша MTT, морфология через внешний платный сервис (RimLangKit/Morpher) и через pymorphy3 (Grabber — у RimLoc `morph`/pymorphy2), скан всех версий в дефолте (Grabber/RimTransAI/Remis тащат все версии; `--include-all-versions` есть флагом), зашитый каталог полей без расширения пользователем (Remis — наши словари гибче).

### RimLoc уже сильнее (после волны 6)
CLI/JSON/schema_version-детерминизм · словарь полей шире зашитых каталогов (Remis теряет `title*`×119 + `reportString`/`deathMessage`×67 на двух модах) · существующий English-DefInjected как источник (+171 на VE мимо Grabber и RimTransAI) · ParentName-наследование в извлечении · scan без установленной игры (RimTransAI без Assembly-CSharp.dll не сканирует ничего) · дефолт «новейшая версия» на LoadFolders-ветках (но НЕ на plain versioned-модах — MUST_FIX 2) · отсутствие фабрикаций и substring-FP · PO/XLIFF для CAT · персистентная TM с fuzzy (против одномодального auto-reuse Remis) · Rust-кроссплатформенность · Runtime Bridge (единственный IN_GAME_PROVEN=7 в выборке). Патч-слой в линии 6465813 закрыт (MUST_FIX 5); актуальный баланс блокирует регрессия 6465813/эталон (MUST_FIX 4).

---

## 5. Discovery 2026-10-05 (свежий поиск)

Команды: `gh search repos "rimworld translation" --sort updated --limit 25`, `gh search repos "rimworld translator"`, `gh search repos "rimworld мод перевод"`, `gh search repos "rimworld локализация модов"` (+ `gh api repos/...` по кандидатурам). 

- **Добавлены строками**: RimTrans_PY (3.20), rimworld-rtl-translation-tools (3.21), RimWorldTranslationTool (3.22).
- **Footnote (не тула)**: `BetterRimworlds/Rimworld-Urdu` — контент-репо «via Autonomo AI» (подтверждение активности их закрытого пайплайна); `RanMd/Rimworld-Translations`, `wigravy/Rimworld-translation-by-wigravy` — контент-репо; `miryeon/RimworldTranslator` (2024-07, спит), `cappie/rwt` (PHP, 2015), `Raschert0/Better-Rimworld-Translator` (C++, 2016), `Senpasi/rimxml2csv` (2024), `mtimoustafa/archotome` (QoL Ruby, 2024), `aiscy/RimworldTranslationHelper` (archived) — шум, материальной релевантности нет.
- **Расхождения метаданных разрешены**: API-детект лицензий GitHub сегодня отдаёт `null` для Grabber GUI / RimTransAI / JalapenoLabs / RimTrans_PY / TokcDK — файлы LICENSE прочитаны содержимо (MIT / GPL-3.0 / MIT / MIT; TokcDK — GPL-3.0 по wave 1), что и зафиксировано в матрице.

---

## 6. Ограничения реконсиляции (честные, после wave 6)

1. MT-стадии нигде не прогонялись: их нет в forge/RimTranslate/laskinss27 (ручной шаг пользователя); Argos (N89) — вне объёма ask; DeepL/Google/OpenAI — BLOCKED_DEPENDENCY (ограничение №3); в wave 6 — deep-translator/googletrans Grabber и OpenAI/LLamaSharp RimTransAI; LibreTranslate-сервер не поднимался.
2. **Same-corpus NOT DONE сжался с 7 feasible до нуля**: все семь остатка прогнаны wave 6 (шесть — полные same-corpus диффы; rtl-tools — прогон скриптов, same-corpus N/A по природе: 0 RTL-строк в корпусе, проверено сканом; инструмент — пост-процессор). Остаток NOT DONE = 9 строк по причинам платформы/зависимостей/отсутствия материи, не по границам волн.
3. GUI-продукты штатно не запускались нигде: TokcDK WPF — BLOCKED_PLATFORM (компиляция OK, запуск нет); RimTransAI Avalonia — сборка OK (0 ошибок), запуск GUI не проверен; Grabber GUI (Tk) и Remis (PySide) — не запускались без платформенной причины (честное «не прогонялся»); RimTrans_PY — tkinter отсутствует в python@3.14. Все оценки покрытия — по ядрам.
4. Полнота 7 (IN_GAME_PROVEN) никому из конкурентов не присвоена и присвоена быть не может в этом стенде; Runtime Bridge RimLoc остаётся единственным уровнем 7 в сравнении.
5. Диффы считались на нормализованных (ключ, значение) с NFC+схлопыванием пробелов; знаменатели (scan / export-po / learn-defs / learn-patches) указаны в строках. Замечание w6-grabber-rtai: описание корпуса волны 3 («Languages/English/DefInjected на корне» у 3170653412) расходилось с фактическим состоянием (Languages нет, Defs виртуализируются в DefInjected-пути) — числа совпали, исправлено описание, не корпус.
6. Реконсиляция wave 6 не перезапускала конкурентные прогоны лейнов — свела их отчёты, скопировала артефакты и **сама исполнила только сверку эталон/HEAD** (§0): scan/export-po на VE и HugsLib + learn-patches на 317 (бинари tgbench и w6-pysmall). Числа py-small по 317 (163→3) и 324 (41→40) реконсилятором не перепрогонялись.
7. Влияние референсной Assembly-CSharp.dll чужой сборки (RimTrans-Reflection) на состав core-типов RimTransAI не оценивалось; версия DLL не сверялась.
8. Дефект `+=`-конкатенации RimTrans_PY заявлен по коду (`TranslationExtractor.py:232-244`) — на корпусе дублей внутри файла не было; non-split overwrite-баг воспроизведён живым вызовом.
9. Во время реконсиляции в ворктtree шёл чужой bisect (stash «wip-during-bisect», reflog 19:15): первый заход правок матрицы был стёрт внешним `git checkout` main и восстановлен реконсилятором; в репо ничего не закоммичено.

## 7. Артефакты

- Wave 1: `COMPETITOR_DEEP_DIVE_2026-10.md`, `competitors.json` (ev.0-2, source-инспекция).
- Wave 2: `TEXT_GRABBER_VS_RIMLOC.md` (ev.5), `RIMLANGKIT_VS_RIMLOC.md` (ev.6), `differential/tgdiff-*.json`, `differential/text-grabber-diff-summary.json`, `differential/rlk-diff-*.json`.
- Wave 3 (скопировано в `differential/`): `TIERA_PYTHON_PRACTICAL.md` + `tiera-python-diff.json`; `TIERA_CS_PRACTICAL.md` + `diff-rwat.json` + `diff-rimtrans-zh.json`.
- **Wave 6 (скопировано в `differential/`, 2026-10-06)**: `REMIS_PRACTICAL_2026-10-06.md` + `remis-diff-<id>.json` ×4; `TIERA_GRABBER_RTAI_PRACTICAL.md` + `tiera-grabber-rtai-diff.json`; `TIERA_PYSMALL_PRACTICAL.md` + `TIERA_PYSMALL_DIFF.json`; `TOKCDK_PRACTICAL.md` + `w6-tocdk-diff.json`. Рабочие данные лейнов: `/tmp/w6-{remis,grabber-rtai,pysmall,tocdk}/`.
- **Реконсиляция wave 6**: сверка эталон/HEAD исполнена в `/tmp/w6-reconcile/` (`scan-ve-ref.json`/`scan-ve-head.json`, `ve-ref.po`/`ve-head.po`, `hugs-ref.po`/`hugs-head.po`, `lp-head.json`) — команды и вывод в §0.
- Мастер-лист: `COMPETITOR_MASTER_LIST_2026-10.md` (секция «Реконсиляция 2026-10-05»).
- Эталон: `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (линия 72259e0b; **для линии `6465813` scan/export-po НЕ идентичны — см. §0/§4.4**); сборка 6465813: `/Volumes/Portable-SSD/caches/targets/w6-pysmall/release/rimloc-cli` (текущий main `bb402cb` — docs-only поверх, crates идентичны).
