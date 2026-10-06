# Tier A py-small — практические прогоны ТРЁХ мелких конкурентов RimLoc

Дата: 2026-10-06 · Лейн: py-small (wave 6) · Рабочая папка: `/tmp/w6-pysmall/`
Дифф-JSON: `/tmp/w6-pysmall/TIERA_PYSMALL_DIFF.json`
Методика — образец wave-3: `_rimloc-worktrees/ba-main/docs/competitive/differential/TIERA_PYTHON_PRACTICAL.md`

## 0. Методика и эталон

**Корпус** (рабочие копии в `/tmp/w6-pysmall/mods/`, оригиналы только на чтение):
`3170653412` (PatchOperations+LoadFolders), `818773962` (HugsLib), `2023507013` (VE Framework 1.0–1.6), `3242000764` (Anomaly Patch, только Patches).

**Эталон RimLoc**: бинарь `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (HEAD 72259e0b, wave-3). Мой прогон сегодня подтверждает идентичность wave-3 числам:

| Мод | scan | export-po | learn-patches |
|---|---|---|---|
| 3170653412 | 159 | 163 | **0** |
| 818773962 | 815 | 76 | — |
| 2023507013 | 764 | 765 | — |
| 3242000764 | 40 | 41 | — |

**Новая сборка RimLoc** (по рецепту задачи): `mkdir -p /Volumes/Portable-SSD/caches/targets/w6-pysmall && cd /Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main && CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/w6-pysmall CARGO_INCREMENTAL=0 cargo build --release -p rimloc-cli` — SHA источника **646581311dbbc143d99e77d26dbda365c2d60c28** (2026-10-06), сборка 3m59s.

**Заявленная «54/54» нового патч-слоя — подтверждена прогоном:**
- `learn-patches --mod-root mods/3170653412 --game-version 1.5_1.6` → **54 записи**, состав в точности совпадает с игроку-видимым набором wave-3 (description 26, title 11, baseDesc 6, titleShort 5, titleShortFemale 4, titleFemale 2), операций: 51 Replace + 3 Add. Шумовых полей (bodyType\*/spawnCategories/requiredWorkTags — урок N89) **ноль**.
- `learn-patches` без `--game-version` → 66 записей (все из 1.4: baseDesc 38, title 16, titleShort 6, titleShortFemale 4, titleFemale 2 — тоже все игроку-видимые), из них 4 — из IfModActive-условной папки `1.4/Patches/Core_Royalty`.
- Известный побочный факт wave-3 воспроизвёлся: `learn-patches` пишет `learn_out/` относительно корня версии мода, не cwd — артефакты вынесены из копий модов в `/tmp/w6-pysmall/out/rimloc-baseline/` (mv, без удаления).

**Побочное наблюдение — расходождение HEAD vs эталон вне патч-слоя (не мой лейн, передать reconcile/RimLoc-лейну):** у сборки 64658131 `scan`/`export-po` дают другие числа, чем эталон 72259e0b, на 3 из 4 модов: scan VE 764→143, export-po 163→3 / 76→0 / 765→140 / 41→40. Часть записей HEAD-scan приписана путям, которых нет на диске (`2023507013/1.6/Languages/English/DefInjected/AbilityDef/Abilities_Trainables.xml` — `stat: No such file or directory`, при том что `1.6/` содержит только Assemblies/Defs/Patches). Команды и числа — в `TIERA_PYSMALL_DIFF.json` (`rimloc_baseline.head_64658131`). Между SHA один коммит трогал `rimloc-core/src/lib.rs`. Регрессия это или редизайн — вне объёма py-small.

---

## 1. RimTrans_PY (masakitenchi) — MIT, спит с 2024-08-22

**Identity**: SHA `81cca257053cc22147ce74e726c44459fc5f9131` (2024-08-22 15:51 +0900, «Fix nesting extraction»), MIT (LICENSE в дереве). Python + lxml + regex. Wave-3: SOURCE_CONFIRMED_ONLY, «третье независимое подтверждение патч-линии» — этот прогон закрывает практическую часть.

**Статус прогона: PRACTICALLY_RUN** — ядро headless (GUI на tkinter, в python@3.14 нет _tkinter; прецедент Text Grabber-лейна). Драйвер `/tmp/w6-pysmall/rtp_driver.py` воспроизводит GUI-метод `Patch_Extract_Tab.do_extract` (main.py:275-355) в его дефолтном split-режиме: BFS → фильтр root Defs/Patch → `TranslationExtractor.extract([file], targets)`.

**Команда воспроизведения:**
```bash
cd /tmp/w6-pysmall && .venv/bin/python rtp_driver.py /tmp/w6-pysmall/mods/3170653412 out/rtp-3170653412.json all
```

**Результаты по корпусу** (Defs-записи / патч-записи): 51/26 · 7/0 · 196\*/0 · 0/0. (\* VE: 540 сырых по 7 версионным папкам → 196 уникальных ключей.)

### ФОКУС-ДИФФ патч-линии: 3170653412 vs новый learn-patches RimLoc — ключевое сравнение волны

| Версия | RimLoc new | RimTrans_PY | BOTH | COMPETITOR_ONLY | RIMLOC_ONLY | SEMANTIC |
|---|---|---|---|---|---|---|
| 1.5_1.6 | 54 | 26 | **26** | **0** | **28** | **0** |
| 1.4 | 66 | 0 | 0 | 0 | **66** | 0 |

- Все 26 совпавших ключей — `BackstoryDef.description`; значения идентичны посимвольно (нормализация NFC+пробелы), SEMANTIC=0.
- **RIMLOC_ONLY=28** на 1.5_1.6: title/titleShort/titleFemale/titleShortFemale/baseDesc — поля, которых нет в словаре RimTrans_PY.
- Почему у RimTrans_PY 0 на 1.4 (19 патч-файлов): (а) `xpath_regex` матчит только поля `label|description` (TranslationExtractor.py:18-20), а 1.4-патчи бьют по baseDesc/title; (б) `PatchOperationAdd` с xpath, разорванным переносами строк (`Defs/BackstoryDef[\n\tdefName="X"\n]`), regex `.*?` без флага dotall не берёт; (в) `anomaly_xpath` (TranslationExtractor.py:42) требует `text()="Defs"` литерально и только класс `PatchOperationAdd`.
- На 3242000764 — 0/0: мод патчит кастомными классами `AnomalyPatch.PatchOperationReplaceIf` с ведущим `/Defs/` и полями bodyType\* — всё вне словаря.
- `ModLoadFolder.py` проверен на живом LoadFolders.xml мода: v1.4/v1.5/v1.6 парсятся, `IfModActive="Ludeon.RimWorld.Royalty"` попадает в `Loadfolders.IfActive` (ModLoadFolder.py:94-101), default-фоллбек для 1.3 отдаёт 1.5_1.6. Wave-3 дизайн-референс подтверждён рантаймом; **в RimLoc HEAD IfModActive уже есть в модели** (`crates/rimloc-services/src/modview.rs:20,70` — «conditional entries returned separately»), и learn-patches берёт файлы из IfModActive-папок (4 из 66).

**Дефекты оригинального кода (найдены прогоном, не ревью):**
1. **`extract()` в non-split режиме теряет всё, кроме последнего файла** — `pairs = extract_tree(tree)` перезаписывает словарь на каждой итерации (TranslationExtractor.py:272,275). Живое доказательство: `PE.extract(все 364 файла VE)` → **1 запись** (последний Defs-файл `1.5/Defs/ApparelLayerDefs/ApparelLayerDefs.xml`). GUI дефолтно работает в split-режиме (split=True, main.py:190), поэтому пользователи не бьют — но API мёртв.
2. **Конкатенация дублей ключей внутри файла**: `keys[res][key] += value` (TranslationExtractor.py:232-234, 242-244) — строки склеиваются вместо замены/предупреждения. На корпусе не сработал (0 семантических расхождений в BOTH-наборе), но это латентная порча данных.

**Gaps RimLoc**: новых нет. Наоборот: новый патч-слой RimLoc **строго перекрывает** RimTrans_PY на этом моде (его 26 ⊂ наших 54, значения 1:1) — wave-3 MUST_FIX закрыт.

**Verdict: подтверждено и закрыто** — «третье подтверждение патч-линии» состоялось как валидация дизайна (полные value-дефы + abstract-фильтр + IfModActive в модели папок), но после патч-слоя 64658131 RimTrans_PY не даёт ни одной строки, которой нет у RimLoc. Классификация фичи патч-слоя: MUST_FIX_BEFORE_BETA → **выполнено** (остаток: сверить HEAD-расхождения scan/export-po, §0).

---

## 2. RimWorldModTranslator (etejasdgjjjj532) — MIT, микро, «живой»

**Identity**: SHA `7432f47ddf43a8a69b9318006e3506519a5446bc` (2026-08-18 06:15 +0900, «Update README.md»), MIT. `translator.py` 180 строк + `gui.py`.

**Статус прогона: PRACTICALLY_RUN** (ядра `extract_all()` на всех 4 модах; GUI-обёртка не запускалась — tqdm не влияет, ядро чистое).

**Команда воспроизведения:**
```bash
cd /tmp/w6-pysmall && .venv/bin/python -c "
import sys; sys.path.insert(0,'/tmp/w6-pysmall/rimwt')
from translator import RimWorldModTranslator
print(RimWorldModTranslator('/tmp/w6-pysmall/mods/818773962').extract_all())"
```

**Результаты по корпусу** (Defs/Keyed/Patches): 0/0/0 · 7/75/0 · 0/593/0 · 0/0/0.

| Корзина (HugsLib, Keyed-слой, знаменатель export-po ref 76) | N |
|---|---|
| BOTH | **75** |
| COMPETITOR_ONLY | 0 |
| RIMLOC_ONLY | 0 |
| SEMANTIC_DIFFERENCE | **7** |

Все 7 семантических — **дефекты экспорта RimLoc, не конкурента**: rimwt хранит сырой текст, RimLoc в msgid вырезает угловые скобки (`<b>The HugsLib mod</b>` → `bThe HugsLib mod/b`) и схлопывает ` > ` (`Mod Options > All mod update news` → `Mod OptionsAll`); `\\n` остаётся литералом (последнее — вероятно, экранирование, но скобки и `>` — реальная порча). Ключи: HugsLib_features_confirmIgnore, _description, _linkDesc, HugsLib_loadOrderWarning_text, _github_token_tip, _shareConfirmMessage, HugsLib_updateRequired_text. Wave-3 находка №2 (msgid-разметка) **воспроизведена независимо и жива в эталоне 72259e0b**.

**Что вскрылось в самом конкуренте (wave-3 deep-dive завышал возможности):**
- `extract_patches()` — **заглушка**, возвращает `[]` (translator.py:138-140, «Implementation for patches (simplified)»), хотя docstring модуля и README обещают «Patches extraction». Патч-линии у rimwt НЕТ.
- `export_to_xlsx()` **падает на живом прогоне**: `pd.DataFrame(...).to_sheet(writer, ...)` — несуществующий метод (translator.py:151; pandas 3.0.6, `hasattr → False`) → AttributeError; при выходе из `with pd.ExcelWriter` сохраняется пустая книга → `IndexError: At least one sheet must be visible`. Рекламный «RimWaldo format XLSX» не производит файла.
- Читает только корень: `mod/Defs`, `Languages/English/Keyed`, `Languages/Japanese` (merge). Версионные папки не резолвит → 3170653412 и 3242000764 дают 0, VE даёт Keyed 593 из корня, но Defs 1.0–1.6 мимо, и DefInjected-источники не читаются вовсе (VE: 765−593=172 строки RimLoc мимо rimwt).

**Verdict: INTENTIONAL_NON_GOAL** как конкурент (заглушка патчей + сломанный экспорт + только корневые папки — регресс против RimLoc по всем осям), но прогон снова **независимо подтвердил MUST_FIX msgid-разметки** (7-й случай, третий лейн). Классификация: MUST_FIX_BEFORE_BETA (msgid-разметка — уже в списке wave-3; новый голос за приоритет).

---

## 3. rimworld-rtl-translation-tools (mtimoustafa) — Ruby, без лицензии

**Identity**: SHA `962a1053a7e8f1c3306ccb5bafe601565d5ada0f` (2024-05-05), лицензии нет (файла LICENSE нет; wave-3: license:null). Скрипты: `reverse_rtl_text.rb`, `contextualize_arabic_letters.rb`, `build_arabic.sh` (+ nokogiri ~> 1.16).

**Статус прогона: PRACTICALLY_RUN** — оба скрипта исполнены (ruby 2.6.10 системный + nokogiri 1.13.8 уже в системе; .ruby-version=3.2.4 не потребовался). Wave-3 DOC_ONLY снят.

**Same-corpus: N/A** — python-скан всех xml корпуса нашёл **0 файлов** с арабскими/ивритскими символами (U+0600–U+06FF, U+0590–U+05FF). Инструмент к тому же не экстрактор: это пост-процессор ГОТОВЫХ переводов (реверс строк для Unity-рендера), т.е. слот «публикация», а не «извлечение» — корзины BOTH/RIMLOC_ONLY неприменимы по природе.

**Команды воспроизведения** (всё в моих копиях; скрипт перезаписывает вход на месте):
```bash
cd /tmp/w6-pysmall/rtl-tools
ruby ./reverse_rtl_text.rb /tmp/w6-pysmall/out/rtl-run/TestFile.xml          # 15 узлов изменено
ruby ./reverse_rtl_text.rb /tmp/w6-pysmall/out/rtl-run/langdata/Backstories.xml
ruby ./contextualize_arabic_letters.rb /tmp/w6-pysmall/out/rtl-run/langdata/Backstories.xml
```

**Проверено на LanguageData-тесте с реальными ключами RimLoc-патчей** (`VengefulNomad67.title` арабский, baseDesc с `[PAWN_nameDef]` и `{0}`, ивритский title): реверс слов корректен, плейсхолдеры `{0}` и `[PAWN_nameDef]`/`[PAWN_possessive]` сохранены на местах (reverse_rtl_text.rb:29-44 — scan/sub/insert вокруг офсетов), арабские буквы переведены в контекстные формы презентации (ﻢﻘﺘﻨﻣ ﻱﻭﺪﺑ), иврит реверсируется.

**Verdict: ROADMAP** — единственный в выборке инструмент с обработкой RTL; для RimLoc это фича публикационного слоя (арабский/иврит: реверс + контекстуализация после build), не конкурент. Идея защиты плейсхолдеров по офсетам — референс для будущего RTL-шага. Без лицензии код нельзя копировать — только дизайн-референс.

---

## 4. Сводная таблица

| | RimTrans_PY | rimwt (etejasdgjjjj532) | rtl-tools |
|---|---|---|---|
| Статус | PRACTICALLY_RUN | PRACTICALLY_RUN (ядро) | PRACTICALLY_RUN (скрипты) |
| 3170653412 | 51 defs + 26 patch | 0 | N/A (RTL в корпусе нет) |
| 818773962 | 7 defs | 7 defs + 75 keyed | N/A |
| 2023507013 | 196 defs (540 raw) | 593 keyed | N/A |
| 3242000764 | 0 | 0 | N/A |
| Патч-линия vs новый RimLoc | 26 ⊂ 54 (BOTH 26 / R_ONLY 28 / SEM 0) | заглушка `[]` | пост-процессор, не экстрактор |
| Уникальное против RimLoc | ничего (перекрыт) | 7 голосов за msgid-MUST_FIX | RTL-пост-обработка (ROADMAP) |
| Дефекты конкурента | non-split extract теряет всё кроме последнего файла; `+=`-конкатенация дублей | XLSX-экспорт падает (to_sheet); только root | — |

## 5. Вердикты (что делать RimLoc)

1. **MUST_FIX_BEFORE_BETA — msgid-разметка (подтверждено третьим независимым прогоном)**: 7 ключей HugsLib, `<b>`/`>` вырезаны в msgid (rimwt хранит сырой текст; RimTranslate доказывал то же в wave-3). Это единственный MUST_FIX, оставшийся живым после появления патч-слоя.
2. **Патч-слой — закрыт**: learn-patches 64658131 даёт 54/54 игроку-видимых на 3170653412@1.5_1.6 (состав 26/11/6/5/4/2 == wave-3 N89), плюс 66 на 1.4, без шума. RimTrans_PY перекрыт полностью (26/26 значений 1:1).
3. **HIGH_VALUE_AFTER_BETA — KeyBindingDef.label и Defs в PO-конвейере**: RimTrans_PY берёт 7 KeyBinding\*-записей HugsLib из Defs (те же, что RimTranslate в wave-3), rimwt — те же 7; RimLoc отдаёт их только через learn-defs. Мост learn-defs→export-po остаётся в силе.
4. **ROADMAP — RTL-публикация** (rtl-tools): реверс+контекстуализация как опциональный шаг экспорта для арабского/иврита.
5. **Передать RimLoc-лейну (вне py-small)**: расхождение scan/export-po сборки 64658131 с эталоном 72259e0b на 3/4 модов корпуса + синтетические пути в scan-JSON, которых нет на диске (§0). Команды воспроизводимы из JSON.

## 6. Честные ограничения прогона

- GUI всех трёх инструментов не запускались: RimTrans_PY (tkinter отсутствует в python@3.14) — ядро через драйвер, повторяющий `do_extract` 1:1 в split-режиме; rimwt — `extract_all()` напрямую; rtl-tools — CLI-скрипты как есть (это и есть их интерфейс).
- MT-стадий нет ни у одного из трёх: RimTrans_PY — только извлечение; rimwt — извлечение+сломанный XLSX; rtl-tools — трансформации. BLOCKED_DEPENDENCY не возникает.
- Дефект `+=`-конкатенации RimTrans_PY не воспроизведён на данных корпуса (нет дублей внутри одного файла) — заявлен по коду (TranslationExtractor.py:232-244) с указанием строк; overwrite-баг non-split воспроизведён живым вызовом (364 файла → 1 запись).
- rtl-tools same-corpus-прогон невозможен по природе (в корпусе 0 RTL-строк, проверено python-сканом) — вместо этого прогон на родном тест-файле репо и на сконструированном LanguageData с ключами из корпуса.
- Диффы считались на нормализованных (ключ, значение) — NFC + схлопывание пробелов, знаменатели указаны в каждом диффе.
- В репозиторий ничего не коммитилось; оригиналы корпуса только читались; `/tmp/rimloc-diff` не трогался.
