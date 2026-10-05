# Text Grabber vs RimLoc — бенчмарк обнаружения строк на одном корпусе

Дата: 2026-10-05 · Lane: text-grabber · Корпус: `/Users/danielviktorovich/Developing/rimloc-test-corpus/steamcmd-root/steamapps/workshop/content/294100/` (4 мода, только чтение)

## 1. Идентичность конкурента

| | Text Grabber | RimLoc (эталон) |
|---|---|---|
| Репозиторий | https://github.com/kamikadza13/Text-grabber | worktree `/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main` |
| Версия | SHA `48697bcaba864737061ea1e7b4d983ae89d3850a`, ветка `main`, коммит «Update 1.7.2 Bugfixes» от 2026-03-15 (совпадает с remote, `git ls-remote`) | HEAD `da2fc777267dfec5c936d06a525b3a6f36d63c2b`, ветка `feature/ui-r1-convergence`, версия CLI 0.1.0-alpha.1 |
| Язык / формат | Python 3, GUI-приложение (tkinter + ttkbootstrap + win32) | Rust CLI (clap), JSON/CSV/PO/XLIFF |
| Лицензия | **отсутствует** (файла LICENSE нет; GitHub API `license: null`) — клон только для локального сравнения | — |
| Блокировка запуска | **BLOCKED_GUI + BLOCKED_WINDOWS**: `Text_Grabber.py:6-13` импортирует `windll`, `win32con`, `win32gui`, `tkinter`; `main()` пишет в tkinter-виджеты (`Text_Grabber.py:397`), завершается `os.startfile` (`:1346`) — CLI-режима в продукте нет | нет |

## 2. Окружение

- macOS (darwin 27.0.0, arm64), Python 3.14.7 (`.venv` в клоне).
- Зависимости Text Grabber ставятся без несовместимостей: `lxml 6.1.3, printy 3.0.1, appdirs 1.4.4, pathvalidate 3.3.1, pillow 12.3.0, pyperclip 1.11.0` (+ `python-Levenshtein`, `requests`). `requirements.txt`/`pyproject` в репо нет — версии сняты с `pip list` рабочего venv. Непереносимое (win32/tkinter) не ставится и не нужно для headless-ядра.
- rimloc-cli собран из worktree `ba-main` (crates/ чист, грязь только в `gui/tauri-app`, `docs`, `testlab`, `Cargo.lock`); сборка: 7 мин 33 c в свежем `CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/rimloc-tgbench` (общий кеш `rimloc` содержал битые rlib-артефакты rustls/webpki: «crate `rustls` required to be available in rlib format» — чужой кеш не трогал, собрал в новый каталог).

**Как запускался конкурент.** Продукт требует Windows+GUI, поэтому ядро запущено headless: `/tmp/rimloc-diff/tgbench/tg_headless_driver.py` импортирует родные модули клона (`Get_searching_folders`, `Finish_string_module`, `Patch_processing→Patch_grabber`, `Get_database_by_list_of_pathes_of_mods`) и дословно вендорит три маленькие функции из непереносимого `Text_Grabber.py:147-237`. Мод копируется в `/tmp/tg-scratch/<id>` (продукт **пишет внутрь папки мода**: `os.chdir` `Text_Grabber.py:252`, `os.makedirs('_Translation')` `Get_searching_folders.py:53`, `rmtree/_Translation` `Text_Grabber.py:399-401`) — корпус остался нетронут. Настройки: `Delete_old_versions_translation=False` (все версии; дефолт продукта `True` = только максимальная, `Settings_module.py:153`), `Copy_original_patches=True` (дефолт `False`, `:158`). Не воспроизведено из GUI-пайплайна: merge абстрактных родителей (`adding_elems_into_root_by_Parent_elem`, `Text_Grabber.py:581`) — учтено в выводах.

## 3. Команды воспроизведения

```bash
# Text Grabber (ядро, по моду; 0.03–0.31 c на мод)
/Users/.../text-grabber/.venv/bin/python /tmp/rimloc-diff/tgbench/tg_headless_driver.py <modid> /tmp/rimloc-diff/tgbench/tg-<modid>.json

# RimLoc (эталон; 0.28–1.87 c на мод)
CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/rimloc-tgbench \
  cargo build --release -p rimloc-cli   # из worktree ba-main
rimloc-cli scan --root <мод> --format json --out-json rimloc-<modid>.json \
  --include-all-versions --with-patches --quiet

# Дифф
python3 /tmp/rimloc-diff/tgbench/tg_vs_rimloc_diff.py
```

Нормализация: identity = (kind, DefType, `defname.path`) для Defs-строк; (keyed, тег) для Keyed; текст сравнивается после strip+схлопывания пробелов+html.unescape. RimLoc фильтруется до английского источника (зеркало выбора TG — `first_founded_lang`, `Text_Grabber.py:180-189`). Пост-проход: нотация `stages.li.label` (RimLoc) ≡ `stages.0.label` (TG) — 5 пар на VE Framework.

## 4. Таблица по 4 модам (числа — идентичности строк)

| Мод | Тип | BOTH | COMPETITOR_ONLY (TG) | RIMLOC_ONLY | SEMANTIC | TG всего | RimLoc всего |
|---|---|---|---|---|---|---|---|
| **3170653412** (PatchOperations+LoadFolders, 1.4/1.5_1.6, IfModActive) | Backstories-патчи | 161 | **230** (123 из патчей, 88 из Defs, 19 из IfModActive-Royalty) | 0 | 1 | 392 | 162 |
| **818773962** (HugsLib, dll, 13 языков, v1.1–v1.6) | Keyed+KeyBinding Defs | 71 | 7 (KeyBindingDef/Category — нет в словаре RimLoc) | 0 | **4** (разметка) | 82 | 75 |
| **2023507013** (VE Framework, версии 1.0–1.6) | Defs 214 файлов + Keyed | 759 (+5 li-fold) | **127** (HediffDef 45, WorkGiverDef 30, TrainableDef 16, PawnRenderTreeDef 14, StatDef 7, …) | 4 («Mote»-заглушки) | 1 (устаревшая версия) | 887 | 764 |
| **3242000764** (Anomaly Patch, 1.5/1.6) | Keyed | 40 | 0 | 0 | 0 | 40 | 40 |
| **Итого** | | **1031** | **364** | **4** | **6** | 1401 | 1041 |

Время: TG-ядро 0.03–0.31 c/мод; rimloc scan 0.28–1.87 c/мод; сборка RimLoc 7m33s однократно.

**False positives.** TG: `PawnRenderTreeDef.…debugLabel` ×14 (подстрока `label` срабатывает внутри `debuglabel` — `Part_of_tag_to_extraction`, `Settings_module.py:91-100`), `workDisables` ×2; синтез `titleFemale/titleShortFemale` ×125 (см. §5.1). RimLoc: наследованные лейблы-заглушки `Mote_*.label="Mote"` ×4 (TG отсекает их списком `Forbidden_text`, `Settings_module.py:131-139`).

**Missed.** RimLoc не увидел 364 строки TG: полные дефы из PatchOperationAdd (163), поля типов вне словаря (WorkGiverDef, TrainableDef, KeyBindingDef, HediffDef label/description/labelNoun, formatString, beginLetter/recoveryMessage…), ст_stage>1 у hediff-стадий (RimLoc отдаёт только первый `.li.`), 125 синтетических форм TG. TG не увидел только 4 «Mote» (намеренный фильтр).

## 5. SEMANTIC_DIFFERENCE (6 случаев, разобраны вручную)

1. **TG синтезирует женские формы и перезаписывает реальные значения (BackstoryDef).** У 3170653412 TG добавил 125 записей `titleFemale/titleShortFemale`, которых нет в XML (сгенерированы из мужского титула: `GTF_AbandonedOrphan61.titleFemale="abandoned orphan"`), а в файле, где поле ЕСТЬ, выдал синтез вместо факта: `GTF_ShipBoy89.titleShortFemale` — TG «ship boy», в XML и у RimLoc «ship girl» (`1.5_1.6/Mods/Royalty/Defs/BackstoryDefs/GTF_Backstories.xml`). Причина: `add_titleFemale/add_titleShortFemale=True` (`Settings_module.py:183-184`) + обработчик `BackstoryDef_add_title_short` (`Finish_string_module.py:573`). RimLoc читает узлы как есть → **скелет TG содержит фабрикованные и одну перезаписанную строку**.
2. **Патч-контент: глубина TG против точечности RimLoc.** TG через `Patch_grabber` извлекает полные дефы, добавленные `PatchOperationAdd` (163 строки у 3170653412: `Backstories_Core1.xml`, `Backstories_VBE2.xml`…), RimLoc с `--with-patches` вывел лишь 36 ключей по xpath (виртуальный `GTF_Backstories.xml`). Но у TG нет фильтра установки мода: путь `IfModActive="Ludeon.RimWorld.Royalty"` отсканирован при отсутствующем DLC (+19 строк вне области релевантности); RimLoc уважает IfModActive в effective-view (`scan.rs:180-193`, Gate H).
3. **Фиделити текста против свежести версии.** HugsLib: значения Keyed с entity-разметкой `&lt;b&gt;The HugsLib mod&lt;/b&gt;…` (XML `English.xml:79`) TG сохраняет точно; RimLoc выдаёт «bThe HugsLib mod/bshould always…» — угловые скобки вырезаны, разметка для переводчика потеряна (4 из 75 ключей). Обратная сторона: в union-режиме (`--include-all-versions`) RimLoc применяет first-file-wins по (DefType,key) (`scan.rs:31-70`), лексикографически «1.4» < «1.6», и текст берётся из **старейшей** версии: `StatDef.VEF_VerbRangeFactor.label` — RimLoc «verb range factor» (1.4), TG «weapon range factor» (1.6, актуально для 1.6-игрока).
4. **Ключи стадий.** RimLoc сворачивает `stages.li.label` в один ключ с текстом первой стадии (потеряны стадии 1..N: 19 строк у VE); TG разворачивает индексированные `stages.0/1/2.label` (5 пар совпали после фолда, остальные — TG-only).
5. **Языки.** RimLoc по умолчанию сканирует ВСЕ Languages/* (815 юнитов у HugsLib против 75 английских); TG берёт строго первый язык из приоритет-списка. Для сравнения RimLoc отфильтрован до English — но дефолтное поведение инструментов различается принципиально.
6. **Существующие DefInjected.** RimLoc читает английские DefInjected-сайдкары как источник (например, `StatDef/Stats.xml` ×59 у VE); TG пропускает DefInjected всегда (`Text_Grabber.py:212-213`, выборка только Keyed/Strings `:196-202`) — для мода с готовым English-DefInjected перевод-скелет TG эти строки потеряет.

**Попутная баг-находка в RimLoc (не конкурент):** `is_version_directory` считает папку `<workshop-id>` (например `3242000764`) версией (чистые цифры, `version.rs:8-30,104-110`), поэтому `resolve_game_version_root` (`version.rs:130-136`) молча возвращает корень и `--game-version 1.5` игнорируется — проверено: probe на 3242000764 вернул контент 1.6 при запросе 1.5.

## 6. Сложность workflow (шагов до результата)

- **Text Grabber: 7 шагов и заблокированная автоматизация.** Клон → venv+6 зависимостей → понять, что CLI нет (Windows-GUI) → написать headless-драйвер (~170 строк) поверх чужих модулей → скопировать мод в scratch (продукт пишет в папку мода) → запуск/мод → ручная нормализация вывода (сырые FDPT-структуры, а не формат обмена). Для обычного пользователя: поставить Python, скачать exe, кликать в GUI — автоматизации и CI нет вообще; воспроизводимость нулевая.
- **RimLoc: 2 шага.** `cargo build --release -p rimloc-cli` (однократно) → `rimloc-cli scan --root … --format json --out-json …`. Детерминированный JSON со schema_version, 0.3–1.9 c/мод.

## 7. Evidence level: **5 из 7**

Шкала: 0 — догадка · 3 — статический разбор кода · 5 — запуск реального кода конкурента на реальном корпусе с изъятиями · 7 — оба продукта прогнаны end-to-end штатным способом.

Ставлю 5: ядро TG (поиск папок, извлечение Defs/Keyed, Patch_grabber) — родной код из зафиксированного SHA, запускался на всех 4 модах; НО штатный продукт не запущен нигде (BLOCKED_GUI+BLOCKED_WINDOWS), GUI-этапы (merge абстрактных родителей, сборка выходной папки, переименования) не воспроизведены, настройки отличаются от дефолта (все версии, патчи вкл.). RimLoc прогнан штатным CLI полностью (7/7 для эталонной стороны).

## 8. Выводы: перенять / уже сильнее

**Перенять у Text Grabber:**
1. **Словарь транслируемых полей шире.** Добавить во встроенный `defs_fields.json` (49 типов): WorkGiverDef (gerund/verb), TrainableDef (label/description), KeyBindingDef/KeyBindingCategoryDef (label/description), HediffDef (label/description/labelNoun/injuryProps…), MentalStateDef (baseInspectLine/beginLetter/recoveryMessage), ScenPartDef/IncidentDef/ApparelLayerDef/StatCategoryDef (label), StatDef formatString — это ~90 из 364 TG-only строк, закрывается чистыми данными, без эвристик.
2. **Индексированные стадии**: `stages.0.label`, `stages.1.label`… вместо одного `stages.li.label` — иначе теряются стадии 1..N.
3. **Извлечение дефов из PatchOperationAdd** (полные добавленные `<def>`-деревья, а не только xpath-вывод) — самый крупный кусок TG-only (163 строки на патч-моде).
4. **Сохранять entity-разметку** (`&lt;b&gt;`) в Keyed-значениях — сейчас RimLoc портит их при извлечении.

**RimLoc уже сильнее:**
1. CLI/JSON/schema_version — полная автоматизация и воспроизводимость; TG физически не имеет headless-режима.
2. Точность: dict-подход без substring-ложных срабатываний (TG ловит `debugLabel` по подстроке `label`), без синтеза фантомных полей и перезаписи реальных значений.
3. Семантика загрузки игры: IfModActive/effective-view, cross-file last-wins для Keyed, уважение отсутствующих DLC; TG тащит контент неактивных модов.
4. Родительское наследование (ParentName) «из коробки» в union-режиме; существующие English-DefInjected читаются как источник (TG пропускает).
5. Все языки/версии настраиваются флагами; у TG версии — только «макс» или «все», язык — только первый из зашитого списка.

## 9. Артефакты

- Клон+NOTES: `/Users/danielviktorovich/Developing/_competitive/text-grabber/` (NOTES.md: SHA, ветка, дата, отсутствие лицензии)
- Headless-драйвер: `/tmp/rimloc-diff/tgbench/tg_headless_driver.py`
- Выгрузки TG: `/tmp/rimloc-diff/tgbench/tg-<modid>.json` · RimLoc: `tgbench/rimloc-<modid>.json`
- Дифф: `tgbench/tgdiff-<modid>.json` + сводка `tgbench/text-grabber-diff-summary.json`
- Скрипт диффа: `tgbench/tg_vs_rimloc_diff.py`
