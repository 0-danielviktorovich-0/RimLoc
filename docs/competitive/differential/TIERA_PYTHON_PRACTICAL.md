# Tier A — практические прогоны ЧЕТЫРЁХ Python-конкурентов RimLoc

Дата: 2026-10-05 · Лейн: tierA-practical-1 · Рабочая папка: `/tmp/w3-py/`
Дифф-JSON: `/tmp/w3-py/tiera-python-diff.json`

## 0. Методика и эталон

**Корпус** (копии для записи в `/tmp/w3-py/mods/`, оригиналы только на чтение):
`3170653412` (PatchOperations+LoadFolders, Defs/Patches в версиях 1.4 и 1.5_1.6, Languages/English/DefInjected на корне),
`818773962` (HugsLib, root Defs + Languages/English/Keyed + 11 языков),
`2023507013` (VE Framework, Defs+Patches в 1.0–1.6, Languages/English на корне),
`3242000764` (Anomaly Patch, только Patches+Languages/English/Keyed в 1.5/1.6, Defs нет вообще).

**Эталон RimLoc**: бинарь `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (версия 0.1.0-alpha.1).
По задаче сборка da2fc77, «scan-поведение идентично текущему». HEAD worktree `/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main` на момент прогона: `72259e0b98c78ca0f34057d86a5ad32f7deb24dd` (2026-10-05).

**База RimLoc (мои прогоны, команды ниже):**

| Мод | scan (все языки) | export-po (English, msgid) | learn-patches | scan --with-patches |
|---|---|---|---|---|
| 3170653412 | 159 | 163 | **0** | Δ=0 (файлы идентичны) |
| 818773962 | 815 | 76 | — | Δ=0 |
| 2023507013 | 764 | 765 | — | Δ=0 |
| 3242000764 | 40 | 41 | — | Δ=0 |

`scan` читает все `Languages/*` (у HugsLib 11 языков → 815), `export-po` — только English: правильный знаменатель покрытия исходных строк — export-po. `learn-defs` на VE Framework: 985 «кандидатов» сырья (со дублями по 7 версиям) = 228 уникальных, из них 196 в 1.6. Важный побочный факт: `learn-defs`/`learn-patches` пишут out-dir **относительно корня мода**, а не cwd — мой первый прогон оставил файлы внутри копии мода (убраны из копий, лежат в `rimloc-baseline/*-OUT`).

Команды эталона:

```bash
RIMLOC=/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli
$RIMLOC scan --root /tmp/w3-py/mods/<id> --format json --quiet > scan-<id>.json
$RIMLOC scan --root /tmp/w3-py/mods/<id> --format json --with-patches --quiet
$RIMLOC export-po --root /tmp/w3-py/mods/<id> --out-po po-<id>.po --pot --quiet
$RIMLOC learn-patches --mod-root /tmp/w3-py/mods/3170653412 --out-json lp.json --quiet
$RIMLOC learn-defs --mod-root /tmp/w3-py/mods/2023507013 --out-dir <out>
```

---

## 1. RimTranslate (winterheart) — GPL-3.0, СПИТ

**Identity**: SHA `ad1a8f71997f988681bbac188dc1c37b16dff0ab`, последний коммит **2024-04-27** (≈17.5 мес тишины), GPL-3.0, v0.6.7. Один файл `RimTranslate.py` (372 строки), lxml+polib.

**Статус прогона: PRACTICALLY_RUN** — полный циклextract→PO→XML на HugsLib.

**Workflow**: `--source-dir` → PO (`DefInjected/` из Defs по whitelist 24 полей, RimTranslate.py:61-85; `Keyed/` из Languages/English/Keyed), `--output-dir` → DefInjected XML из PO, `--compendium` — TM из готовых переводов.

**Команды воспроизведения:**

```bash
cd /tmp/w3-py && .venv/bin/python RimTranslate/RimTranslate.py \
  --source-dir mods/818773962 --po-dir rt-out/818773962
.venv/bin/python RimTranslate/RimTranslate.py \
  --output-dir rt-build/818773962 --po-dir rt-out/818773962
```

**Результаты по корпусу:** 0 / 85 / 323 / 0 PO-записей (3170653412 / 818773962 / 2023507013 / 3242000764).

**Дифф с RimLoc (HugsLib, лучший случай конкурента):**

| Корзина | N | Что это |
|---|---|---|
| BOTH | 71 | одинаковые ключи+текст (Keyed) |
| COMPETITOR_ONLY | **7** | KeyBindingDef.label из Defs (`Publish log file (hold Ctrl)`…) — реальный игроку-видимый текст настроек; RimLoc export-po Defs не читает (есть только learn-defs отдельной стадией) |
| SEMANTIC_DIFFERENCE | 4 | **RimLoc портит разметку в msgid**: `<b>The HugsLib mod</b>` → `bThe HugsLib mod/b` (вырезаны угловые скобки, имена тегов остались). RimTranslate хранит сырой текст. Для CAT/PO-раундтрипа это дефект RimLoc |
| RIMLOC_ONLY (3170653412) | 163 | Languages/English/DefInjected конкурентом не сканируется вообще (ход только по Keyed) |

**Находки:**
- Жёсткий дефект потока: нет root `Defs/` → `logging.error` + `quit()` **до** стадии Keyed (RimTranslate.py:282-284), причём exit code 0. Мод 3242000764 с 41 строкой в Keyed обработать невозможно в принципе. 3 из 4 модов корпуса недоступны без ручного запуска в версионную подпапку.
- Нет поддержки LoadFolders/версий: 3170653412 и 2023507013 требуют ручного `--source-dir mods/X/1.5_1.6`.
- Round-trip PO→XML проверен: тестовый перевод записан в `<HugsLib_settings_btn>ТЕСТ-ПЕРЕВОД</HugsLib_settings_btn>`, statistics `81/0/1/82`.

**Gaps RimLoc, вскрытые конкурентом**: (а) порча разметки в msgid — см. SEMANTIC; (б) KeyBindingDef.label из Defs конкурент берёт в PO-конвейер, RimLoc — только через learn-defs-отчёт.

**Verdict: MUST_FIX_BEFORE_BETA** — не ради конкурента (он спит и мал), а из-за найденного: msgid с вырезанной разметкой (`bThe HugsLib mod/b`) — это дефект экспорта RimLoc, ломающий доверие переводчика к PO.

---

## 2. Translation Forge (Momaomao8787) — MIT, активный

**Identity**: SHA `5a21f9d12debbb952f739af71b2ec4bf4bca923e`, коммит **2026-10-05 09:10 (+0800) — сегодня**, MIT. Пакет `core/` + tkinter-UI, CLI `forge`, UI на 15 языках.

**Статус прогона: PRACTICALLY_RUN** (scaffold + check + export на 3 модах). MT-стадий в инструменте **нет вообще** — это human-in-the-loop чекер/экспортёр, BLOCKED_DEPENDENCY не возникает.

**Workflow**: `scaffold` (скелет языкового пакета) → `check` (недостающие переводы: source-Defs vs target-пакет) → `export` CSV/XML → ручной перевод → `import` записи в DefInjected. Внутри: LoadFolders-резолв, ParentName-наследование, whitelist DEFAULT_FIELDS.

**Команды воспроизведения:**

```bash
cd /tmp/w3-py && .venv/bin/pip install -e ./Translation-Forge
.venv/bin/forge scaffold --source-mod mods/<id> --target-mod tf-packs/<id>-ru --lang Russian
.venv/bin/forge check   --source-mod mods/<id> --target-mod tf-packs/<id>-ru --lang Russian
.venv/bin/forge export  --source-mod mods/<id> --target-mod tf-packs/<id>-ru --lang Russian --format csv --out tf-packs/pending-<id>.csv
```

**Результаты:** pending 156 / 7 / 202 / **err.defs_not_found (exit 1)** — на безDefs-моде отказ изящный, но мод не обрабатывается.

**Дифф Defs-слоя с RimLoc (VE Framework, версия 1.6):**

| Корзина | N | Примеры |
|---|---|---|
| BOTH | 196 | ядро label/description/reportString/stages.N.label — совпадает с `learn-defs` 1:1 |
| COMPETITOR_ONLY | **6** | 4× Mote_*.label (огневые следы — спорная ценность: motes почти не видны), 2× HediffDef.labelNoun (реальная находка) |
| RIMLOC_ONLY | 0 | — |

LoadFolders-резолв корректен: 3170653412 → `1.5_1.6/Defs` + условный `1.5_1.6/Mods/Royalty/Defs`.

**Gaps RimLoc**: learn-defs whitelist уже почти дотягивается до forge; реальные потери — единичные поля типа `labelNoun`. Keyed/Patches вне скоупа forge — автор честно документирует 13 классов пропусков (README «已知限制»), включая «仅扫静态 Defs/».

**Verdict: ROADMAP** — workflow «сопровождение готового языкового пакета» (check/stale/import) — отдельный продуктовый срез; для RimLoc-беты критичного ничего.

---

## 3. RimWorldModTranslator (NicoriciN89) — Apache-2.0 — ГЛАВНЫЙ ДИФФЕРЕНЦИАТОР

**Identity**: SHA `ccb1df508cc93dce4f7e3fd35e78b74990f6d1f5`, коммит 2026-07-22, Apache-2.0. Оффлайн-переводчик: Argos Translate (bundled en→ru/uk/de/fr) + опциональный LLM-polish через Ollama. Scanner (354 стр.) + patches.py (132 стр.).

**Статус прогона: PRACTICALLY_RUN** (scanner+patches на всём корпусе). MT-стадия (Argos) **не прогонялась** — вне объёма задачи (просили scanner/patches).

**Команды воспроизведения:**

```bash
cd /tmp/w3-py/RimWorldModTranslator && /tmp/w3-py/.venv/bin/python -c "
import sys; sys.path.insert(0,'.')
from pathlib import Path
from src.scanner import scan_mod
r=scan_mod(Path('/tmp/w3-py/mods/3170653412'))
print(sum(len(t.data.keyed_items()) for t in r.keyed),
      sum(len(t.data.keyed_items()) for t in r.def_injected))"
```

**Результаты scan по корпусу:** 313 / 82 / 1113 / 40 (Keyed+DefInjected-задачи).

Состав 313 на 3170653412: 213 — Defs-fallback (GTF-бэкстории; ровно равно 213 кандидатам rimloc learn-defs), 100 — патч-производные.

### ФОКУС-ДИФФ патч-извлечения: 3170653412, версия 1.5_1.6

Экстракция `src/patches.py`: PatchOperationAdd/Replace/Insert на любой глубине вложенности (Sequence/FindMod/Conditional), разбор типового xpath `Defs/Type[defName="X"]/путь/li[N]` (1-базные → 0-базные индексы), обход `<value>` той же `_walk_def`, что и для обычных Defs; неоднозначные формы (голый `li`, предикаты по label) пропускаются (patches.py:39-70).

| Метрика | RimWorldModTranslator | RimLoc |
|---|---|---|
| Патч-файлов в 1.5_1.6 | 25 | те же 25 |
| Извлечено refs | **100** | **0** (learn-patches: 0 записей; scan --with-patches: Δ=0) |
| — игроку-видимые | **54** (description 26, title 11, baseDesc 6, titleShort 5, titleShortFemale 4, titleFemale 2) | 0 |
| — НЕ игроку-видимые (шум) | **46** (bodyTypeMale/Female 30, requiredWorkTags.* 11, spawnCategories.0 3, backstoryFiltersOverride… 2) | — |
| Пересечение ключей с rimloc export-po | 0 (патчи бьют по чужим/ванильным def-ам: MafiaBoss17, Gigolo30, VBE_*, AssassinsBro_*) | — |

Примеры реальных находок конкурента: `Gigolo30.title='courtesean'`, `SpaceTactician28.title='war tactician'`, `MafiaBoss17.description='[PAWN_nameDef] was a high-ranking member of a crime syndicate…'`, `VBE_GalleyChef.baseDesc=…`. Это ровно тот класс «полные дефы против xpath-ключей», о котором задача говорит «семантическая разница, не баг»: игроку эти строки **видны** (заголовки и описания бэксторий), и RimLoc их отдаёт только как… ничего — даже xpath-ключей на этом моде не выдаёт.

**Качество, а не только количество**: из 46 шумных 30 — `bodyType*` (defName-ссылки на BodyTypeDef), 11 — `requiredWorkTags.*` и 3 — `spawnCategories.0`: их перевод через DefInjected **ломает матчинг идентификаторов** (спавн-категории бэксторий, work tags). Чёрный список `NEVER_TRANSLATABLE_TAGS` (rimworld_rules.py:32-58) кураторский с prov-комментариями «Найдено на: …», но этих полей не знает. Урок для RimLoc: патч-экстракция обязана идти с жёстким полевым фильтром, иначе 46% шума с коррупционным риском.

На 3242000764 патчи текста не содержат — конкурент честно выдал 0.

**Gaps RimLoc**: патч-слой — крупнейшая подтверждённая дыра (0 против 54 игроку-видимых на одном моде; learn-patches и scan --with-patches молчат).

**Verdict: MUST_FIX_BEFORE_BETA** — патч-текст (полные значения PatchOperationAdd/Replace/Insert с фильтром полей) в бете RimLoc отсутствует, конкурент это умеет уже сегодня.

---

## 4. rimworld-mod-translator (laskinss27-cmyk) — MIT

**Identity**: SHA `8dc978d982aab2d3373ee8bb0f9b8810d0df37a5`, коммит 2026-08-13, MIT. Один файл 1796 строк: tkinter-GUI (Windows-first) + CSV→Google Sheets flow (формула `=GOOGLETRANSLATE(D2;"auto";"ru")` — шаг пользователя, не API-вызов инструмента).

**Статус прогона: PRACTICALLY_RUN для локальных стадий** (extract+build прогнаны на всём корпусе). **Sheets-перевод: BLOCKED_DEPENDENCY** по условию задачи (не заводить; фактически ключи не нужны — MT делается пользователем вручную в Google Sheets, сам инструмент оффлайновый).

**Команды воспроизведения** (GUI-методы вызваны напрямую, tkinter заглушён):

```bash
PYTHONPATH=/tmp/w3-py/stub /tmp/w3-py/.venv/bin/python - <<'PY'
import sys; sys.path.insert(0,'/tmp/w3-py/rimworld-mod-translator')
import rimworld_translator as R
# find_xml_files -> collect_identifiers -> extract_from_file (см. export_text, rimworld_translator.py:1419)
# build: R.build_translation_package(mod_root, csv, out_root, 'Russian', '1.6')
PY
```

**Результаты extract:** 181 / 83 / 1701 / 72 строк CSV. Build-стадия: 3 тестовых перевода → `Languages/Russian/Keyed/AnomalyPatch_Keys.xml` + `About/About.xml` + Generation report; структура мода-перевода корректна.

**Критические архитектурные ограничения (из кода):**
- `find_xml_files` (rimworld_translator.py:1375) **пропускает папки `patches` целиком** — патч-текст не извлекается (−54 игроку-видимых на 3170653412 относительно NicoriciN89).
- `in_languages and not in_english_keyed → continue` (rimworld_translator.py:1383-1385): Languages/English/**DefInjected** не читается как источник — 163 записи RimLoc на 3170653412 мимо.
- **Нет резолва версий**: 2023507013 даёт 321+248+205+181+145+4+4 строк по всем семи версиям сразу (последняя — RimLoc 765: только Languages 1.6); 3242000764 — 32 (1.5) + 40 (1.6) в одном CSV; 3170653412 — смесь 1.4 и 1.5_1.6.
- Мусор из About: `About/ModSync.xml` дал 1 строку (в skip-листе только about.xml).

Плюс: трёхслойный фильтр `is_translatable` + `collect_identifiers` (кросс-проверка defName, «Fire/Wood не переводить») — идея умная, на Keyed работает (75/75 на HugsLib).

**Gaps RimLoc**: ничего критичного; модель «экспорт CSV → ручной Sheets» — упрощённая альтернатива PO-конвейеру RimLoc.

**Verdict: INTENTIONAL_NON_GOAL** — копировать нечего: пропуск DefInjected-источников, отсутствие версионности и патчей — регресс против RimLoc; единственная идея на заметку — кросс-проверка идентификаторов как доп. фильтр эвристики.

---

## 5. Сводная таблица

| | RimTranslate | Translation Forge | RimWorldModTranslator | laskinss27 |
|---|---|---|---|---|
| Статус | PRACTICALLY_RUN | PRACTICALLY_RUN | PRACTICALLY_RUN (scanner/patches) | PRACTICALLY_RUN (+Sheets: BLOCKED_DEPENDENCY) |
| 3170653412 | 0 | 156 pending | **313** | 181 |
| 818773962 | 85 | 7 | 82 | 83 |
| 2023507013 | 323 | 202 | **1113** | 1701 (7 версий) |
| 3242000764 | 0 | err.defs_not_found | 40 | 72 (2 версии) |
| RimLoc export-po | 163 | (Defs-слой: learn-defs 196 @1.6) | — | — |
| Патчи | нет | нет (автор-документировано) | **54 игроку-видимых + 46 шума** | нет (skip 'patches') |
| Версии/LoadFolders | нет (quit-баг) | да | да | нет (всё в кучу) |

## 6. Вердикты (что делать RimLoc)

1. **MUST_FIX_BEFORE_BETA — патч-слой**: RimLoc на 3170653412 извлекает из 25 патч-файлов **ноль** (learn-patches=0, scan --with-patches Δ=0) при 54 игроку-видимых строках у конкурента. Брать семантику patches.py (полные значения с фильтром полей), а не xpath-ключи. Обязателен полевой blacklist (bodyType*, spawnCategories, requiredWorkTags, backstoryFilters*) — иначе 46% шума с риском порчи данных, как у конкурента.
2. **MUST_FIX_BEFORE_BETA — разметка в msgid**: `export-po` вырезает `<b>`/`</b>` в `bThe HugsLib mod/b` (4 ключа HugsLib). Переводчик в CAT видит мусор; RimTranslate доказывает, что сырой текст сохранить можно.
3. **HIGH_VALUE_AFTER_BETA — Defs в PO-конвейере**: KeyBindingDef.label (HugsLib, +7) и `labelNoun` (VE, +2) попадают в PO у конкурентов, у RimLoc — только в learn-defs-отчёт. Мостить learn-defs → export-po.
4. **ROADMAP** — workflow сопровождения языкового пакета (Translation Forge: check/stale/import) — отдельный срез, не для беты.
5. **INTENTIONAL_NON_GOAL** — CSV/Sheets-конвейер laskinss27 и спящий PO-конвейер RimTranslate как таковые; ничего уникального, кроме найденных выше дефектов RimLoc, не дают.

## 7. Честные ограничения прогона

- MT-стадии нигде не прогонялись: у RimTranslate/forge их нет (RimTranslate — TM-компендиум, forge — ручной шаг), Argos у NicoriciN89 не запускался (вне объёма ask), Sheets у laskinss27 — BLOCKED_DEPENDENCY по условию (ключи не заводились; фактически MT выполняется пользователем вручную).
- GUI-инструменты (forge-ui, NicoriciN89 GUI, laskinss27 tkinter) не запускались — прогон ядер через CLI/программные вызовы; у laskinss27 экспорт воспроизведён вызовом методов `export_text` без GUI (логика 1:1, rimworld_translator.py:1419-1560).
- Первый прогон laskinss27 (2660 строк на VE) был загрязнён моим же артефактом rimloc learn-defs (suggested.xml внутри копии мода) — числа в отчёте из чистого прогона (1701). Артефакты перемещены из копий модов (mv, без удаления).
- GUI-tkinter в системе нет (python@3.14 без _tkinter) — потому стаб; на результаты ядер не влияет.
- Побочное наблюдение по RimLoc: `learn-defs`/`learn-patches` пишут относительный out-dir от корня мода, а не от cwd (замечено на прогоне, файлы сначала легли в mods/<id>/rimloc-baseline/…).
