# Remis — практический прогон (wave 6, lane remis)

**Дата:** 2026-10-06 · **Статус прогона:** `PRACTICALLY_RUN` (headless-адаптер исполнен на корпусе; MT-стадии не затрагивались — для данной задачи не требовались)

## 1. Identity

| Поле | Значение |
|---|---|
| Продукт | Remis — https://github.com/Drlinglong/Remis |
| Лицензия | AGPL-3.0 |
| Клон | `/tmp/w6-remis/remis` (shallow, depth 50) |
| **Frozen HEAD SHA** | `5440918445b3006c026a64193c30a09c0ed7e0d6` (совпадает с ожидаемым `5440918`) |
| HEAD commit | 2026-10-05 23:54:36 +1100 — «fix(release): ship Remis v3.2.2 bugfix and workflow improvements» (v3.2.2) |
| Эталон для диффа | rimloc-cli 0.1.0-alpha.1, `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` (HEAD источника 72259e0b по заданию; RimLoc-worktree для сверки фич: `_rimloc-worktrees/ba-main` @ `646581311dbbc143`, 2026-10-06) |
| Корпус | 4 мода, рабочие копии `/tmp/w6-remis/corpus/<id>/` (исходник только читался) |

## 2. Что прогонялось — headless-точка входа

GUI Remis (PySide/Tauri) не нужен: RimWorld-адаптер — чистая библиотека.

- **Контракт** — `scripts/core/game_adapters/contracts.py:63-82`: `GameAdapter` = 7 методов (`discover`, `parse`, `parse_text`, `render`, `package_metadata`, `validate`, `language_folder`) + `id`/`game_id`. Подтверждено чтением.
- **Адаптер** — `scripts/core/game_adapters/rimworld.py:26` `RimWorldAdapter`, `FORMAT_RULES_VERSION = "rimworld-1.6-v1"` (`rimworld.py:13`), жёсткий каталог полей `_V16_DEF_FIELDS` (`rimworld.py:14-23`).
- **Headless-обвязка** — `scripts/core/game_adapters/workflow_bridge.py:60-74` `build_snapshot()` (discover→extract по всем ресурсам); для бенчмарка звал адаптер напрямую (`run_adapter.py`, команды ниже).
- **Реестр** — `registry.py:6-9`: структурированных адаптера два (`rimworld`, `project_zomboid`), остальное — `LegacyAdapter`.

Команды прогона (все исполнены в этой сессии):

```bash
python3 /tmp/w6-remis/run_adapter.py /private/tmp/w6-remis/corpus/<id> 1.6 results/remis_<id>_v1.6.json
python3 /tmp/w6-remis/run_adapter.py /private/tmp/w6-remis/corpus/<id> auto results/remis_<id>_auto.json
rimloc-cli scan --root /private/tmp/w6-remis/corpus/<id> --lang en --game-version 1.6 \
  --format json --out-json results/rimloc_<id>_v1.6.json --quiet
# варианты: без --game-version (auto), --game-version 1.3, --include-all-versions, --with-patches
python3 /tmp/w6-remis/diff_buckets.py <id>   # 4 корзины, нормализация {key, source}
```

## 3. Цифры по модам (источник = English, версия 1.6)

| Мод | Профиль | Remis: ресурсы/строки | Remis время* | RimLoc: строки | RimLoc время** |
|---|---|---|---|---|---|
| 3170653412 | LoadFolders + PatchOperations | 2 / 40 (только Defs) | 6 мс | 159 | 0.17 с |
| 818773962 | HugsLib (dll) | 3 / 82 (keyed 75 + defs 7) | 4.2 мс | 75 (только Keyed) | 0.07 с |
| 2023507013 | VE Framework 1.0–1.6 | 67 / 859 (keyed 593 + defs 266) | 37.8 мс | 764 | 0.26 с |
| 3242000764 | Anomaly Patch | 1 / 40 (keyed) | 2.2 мс | 40 | 0.07 с |

\* in-process, discover+parse (`time.perf_counter`). \*\* wall-clock процесса (старт+телеметрия в комплекте) — уровни измерения разные, сравнивать порядок величин, не числа.

`auto`-режим Remis (без указания версии) дал идентичные результаты на всех 4 модах — инферсил 1.6 (`game_version_inferred`, `rimworld.py:406-418`).

## 4. Same-corpus дифф (4 корзины, нормализация {key→без префикса DefType::, source})

| Мод | BOTH | SEMANTIC | RIMLOC_ONLY | REMIS_ONLY |
|---|---|---|---|---|
| 3170653412 | 40 | 0 | **119** | 0 |
| 818773962 | 71 | 4 | 0 | 7 |
| 2023507013 | 688 | 9 | 67 | **153** |
| 3242000764 | 40 | 0 | 0 | 0 |

JSON-корзины: `/tmp/w6-remis/results/diff_<mod>.json` (плюс сырые `remis_*.json`, `rimloc_*.json`).

### 4.1 SEMANTIC (13 случаев, разобраны вручную)

- **818773962, 4 случая — деградация источника у RimLoc.** Remis сохраняет точный текст (`<b>The HugsLib mod</b>`, `Options > All`), RimLoc возвращает `bThe HugsLib mod/b`, `Mod OptionsAll` — угловые скобки (включая легитимный standalone `>`) выброшены. Remis здесь объективно точнее (span-сохраняющий парс, `rimworld_xml.py:16-60`).
- **2023507013, 9 случаев:** 8 — тримминг краёв у RimLoc (`Search: ` → `Search:`; хвостовой пробел в RimWorld часто значим для конкатенации), 1 — реальный конфликт значения `VEF_VerbRangeFactor.label`: Remis `weapon range factor` (= диск 1.6/Defs/Stats_Pawns.xml), RimLoc `verb range factor` (= текст из 1.3/1.4, см. 4.3). **По диску прав Remis.**

### 4.2 RIMLOC_ONLY — что Remis пропускает

- **3170653412 (119):** `title`×52, `titleShort`×52, `titleFemale`×2, `description`×11, `baseDesc`×1, `titleShortFemale`×1. У Remis каталог `rimworld-1.6-v1` не знает семейства `title*` у `BackstoryDef` → в игре останутся непереведённые титулы персонажей. Реальная потеря контента, не шум.
- **2023507013 (67):** `reportString`×32 (`RecipeDef`), `label`×18, `description`×10, `stages`×5, `deathMessage`×2 — поля, которых нет в каталоге Remis, но есть в словаре RimLoc.

### 4.3 REMIS_ONLY — что пропускает RimLoc (по умолчанию)

- **2023507013 (153):** `gerund`/`verb` (`WorkGiverDef`, 22), rulesStrings-индексы (~75), `stages.*.label`/gizmo-поля (~56) — покрываются каталогом Remis; RimLoc добирается флагами `--defs-field/--defs-dict/--defs-type-schema`, но дефолт уже.
- **818773962 (7):** `KeyBindingDef.label` ×7 (`PublishLogs`, `OpenLogFile`…) — обычные label, но RimLoc их не взял (словарь по DefType; KeyBindingDef в нём отсутствует).

### 4.4 Versioned-папки и LoadFolders — главное расхождение

- **Remis** (`rimworld.py:275-340`): парсит `LoadFolders.xml`, выбирает ветку `v1.6` (в 3170653412 папка `1.4` корректно исключена); `IfModActive/IfModNotActive/IfModActiveAll` резолвятся только при явном списке `active_mods` в `source_lang`; без него — ошибка `loadfolders_condition_unknown` (severity=error, `workflow_bridge.py:23-25` уронит весь scan). С `active_mods=["Ludeon.RimWorld.Royalty"]` — корректно: 3 ресурса вместо 2. Plain version-папки (2023507013): выбирает ровно одну — max ≤ запрошенной версии (`rimworld.py:331-336`).
- **RimLoc:** на моде с LoadFolders ветку `v1.6` уважает (1.4 исключён), **но `IfModActive` игнорирует** — Royalty-контент (`1.5_1.6/Mods/Royalty/Defs/GTF_Backstories.xml`, 36 строк) включён безусловно. На моде с plain version-папками (2023507013) **сливает ВСЕ версии 1.0–1.6**: в выводе 81 юнит из файлов, которых в 1.6 нет (`Stats.xml` — 59, `Jobs_ItemProcessor.xml`, `Jobs_Machine.xml`, `Needs_Machine.xml`, `Dummy.xml`, `WorkGivers_ItemProcessor.xml`), и устаревшие значения вместо актуальных (`verb range factor` из 1.3/1.4 против `weapon range factor` в 1.6). **`--game-version 1.3`, `--game-version 1.6`, auto и `--include-all-versions` дали побайтово одинаковые 764 юнита** — флаг на этом моде не влияет.
- **Follow-up баг Remis (найден при прогоне):** `_effective_roots` резолвит симлинки (`rimworld.py:424` `.resolve()`), а `_package_relative` сравнивает с нерезолвнутым `root` (`rimworld.py:385-386`) → на macOS `/tmp/...` (симлинк на `/private/tmp`) любой LoadFolders-мод падает `ValueError: ... is not in the subpath of`. Воспроизведено: `discover(Path("/tmp/w6-remis/corpus/3170653412"), …)` → ValueError; тот же мод по `/private/tmp/...` — ОК. Затронута только LoadFolders-ветка.

### 4.5 Patches

- **Remis:** extraction не делает вообще; даёт честную диагностику `patch_runtime_unknown` — «PatchOperations can alter translatable Defs; offline extraction cannot resolve their runtime results» (`rimworld.py:243-244`) + `assembly_strings_unavailable` для dll (`rimworld.py:245-246`). Цитата подтверждена в коде.
- **RimLoc:** `--with-patches` («infer DefInjected keys from xpath») на всех 4 модах добавил **0 юнитов** — включая 3170653412, где в патчах 53 переводимых тега, среди них статистически однозначные кейсы (`PatchOperationReplace` c xpath `Defs/BackstoryDef[defName="AssassinsBro_Medieval_MadQueen"]/title` → `mad ruler`, файл `1.5_1.6/Patches/Medieval_Backstories/BackstoriesDef.xml:44-51`). Молча, без предупреждений. (Для контекста: 2023507013 и 3242000764 патчи действительно структурные — там 0 переводимых тегов, потому оба инструмента «правы».)

## 5. Re-использование / чекпойнты / глоссарий — против RimLoc (TM/глоссарий/session-persist)

RimLoc-сторона сверена по его worktree (`_rimloc-worktrees/ba-main` @ 64658131): `crates/rimloc-domain/src/tm.rs` (TranslationMemoryEntry + bounded_levenshtein/fuzzy_threshold), `crates/rimloc-domain/src/glossary.rs` (GlossaryTerm), `crates/rimloc-services/src/session.rs` (ProjectSessionManager: create/open/snapshot/list).

| Фича Remis | Где в коде | Аналог в RimLoc | Вердикт |
|---|---|---|---|
| Re-использование существующих переводов **одного мода** (`existing_translations`: ищет готовые файлы целевого языка, конфликт значений → ValueError) | `game_adapters/translation_reuse.py:8-44` | **TM есть, и шире**: персистентная база + fuzzy-match (Levenshtein) | У RimLoc уже есть в лучшем виде; одномодальный авто-reuse Remis — не TM |
| Чекпойнты перевода: атомарный `.remis_checkpoint.json` (schema v3, identity task/run/project + config_fingerprint + source_snapshot_hash), resume с проверкой совместимости, feature-flag | `core/checkpoint_manager.py:1-60, 251-277`, `core/feature_policy.py:20`, `routers/translation_recovery.py:39-100` | session-persist есть (`ProjectSessionManager`), но это проектные сессии, а не пофайловый resume перевода | **Чего в RimLoc нет в таком виде** — пофайловый атомарный resume с контрактом совместимости |
| Глоссарий: CRUD, дерево, поиск по game_id | `routers/glossary.py:261-344` | глоссарий есть (`GlossaryTerm`) | Паритет по базе |
| **Glossary health**: детерминированный отчёт score/100 + опциональный AI-advice (structured output, suggestions-only, никогда не мутирует данные) | `core/glossary_health_reviewer.py:24-40`, `routers/glossary.py:83-199` | не обнаружено в worktree (grep health по glossary-модулям — пусто) | **Дифференциатор Remis** |
| Верифицируемая запись: `_verify_rendered` (обратный парс рендера + сверка ключей/значений), атомарная запись с rollback, manifest с provenance (source_hash) | `workflow_bridge.py:104-203` | частично (validate/import-пайплайн RimLoc; эквивалент rollback-записи пакета не проверялся в этой задаче) | Сильная инженерная практика Remis; переносить идею |
| Токен-валидация `{0}`/`{x}_label`/теги/`->`-грамматика | `rimworld_text.py:85-99` | есть в validate-крейте | Паритет (глубокое сравнение не входило в задачу) |

Классификация рекомендаций для нашей платформы:

| Фича | Классификация |
|---|---|
| Семейство `BackstoryDef::title/titleShort/titleFemale` (+ `RecipeDef::reportString`, `deathMessage`) в извлечение | **MUST_FIX_BEFORE_BETA** (119+67 строк реального контента теряются даже дефолтом RimLoc — но у Remis это главный gap; у нас проверить покрытие словарём) |
| Версионные папки без LoadFolders: жёсткий выбор одной версии, никаких слияний | **MUST_FIX_BEFORE_BETA** (антипример RimLoc: stale-значения из 1.3/1.4 в выдаче) |
| Статическая резолвка однозначных PatchOperationReplace/Add (xpath с literal defName) | **HIGH_VALUE_AFTER_BETA** (RimLoc заявляет — на корпусе 0; Remis не пытается; кто сделает честно — преимущество) |
| `IfModActive`-резолвка по списку активных модов | **HIGH_VALUE_AFTER_BETA** (RimLoc сейчас over-включает DLC-контент) |
| Пофайловый атомарный чекпойнт перевода с контрактом совместимости | **HIGH_VALUE_AFTER_BETA** |
| Glossary-health score + advisory AI | **ROADMAP** (приятно, не барьер) |
| Сверхжёсткий каталог полей без расширения пользователем (взамен — словари RimLoc `--defs-dict`) | **INTENTIONAL_NON_GOAL для нас** — гибкие словари лучше зашитого каталога; Remis сам в этом проигрывает |

## 6. Gaps / риски Remis (для полноты)

1. Симлинк-баг LoadFolders-ветки (4.4, `rimworld.py:424` vs `:385-386`) — воспроизведён.
2. Каталог `rimworld-1.6-v1` зашит и не расширяется; для не-1.6 — только предупреждение `game_version_rules_unverified` (`rimworld.py:46-47`).
3. `IfModActive` без списка активных модов — severity=error, что валит весь `build_snapshot` (`workflow_bridge.py:23-25`): мод 3170653412 непрогоняем пайплайном Remis без ручного списка.
4. Patches не извлекаются вовсе (честно, но 53 переводимых тега на одном моде корпуса остаются вне конвейера).

## 7. Verdict

Remis — не «GUI-обёртка без ядра»: его RimWorld-адаптер — аккуратный, span-сохраняющий, source-preserving движок с контрактом из 7 методов, верифицируемой записью и честной диагностикой того, чего он не умеет (Patches, dll). На корпусе он быстрее на порядках (миллисекунды vs десятки-сотни мс на процесс) и точнее в деталях источника (markup, пробелы, актуальные версии файлов). Его слабые места — зашитый каталог полей Defs (теряет `title*`/`reportString`/`deathMessage` — суммарно 186 строк на двух модах), нерешённые PatchOperations и баг симлинков в LoadFolders. Главные дыры RimLoc, подсвеченные диффом: слияние версионных папок с устаревшими значениями при неэффективном `--game-version`, игнор `IfModActive`, молчаливый ноль от `--with-patches`, потеря угловых скобок и тримминг в Keyed. Для битвы за переводчиков модов решают покрытие полей и корректность версионных веток — здесь ни одна из платформ пока не закрывает корпус целиком.

## 8. Артефакты

- Отчёт: `/tmp/w6-remis/REMIS_PRACTICAL_2026-10-06.md` (этот файл) · `NOTES.md` (SHA, команды)
- Раннер: `run_adapter.py` · Дифф: `diff_buckets.py`
- JSON: `results/remis_<id>_{v1.6,auto}.json`, `results/rimloc_<id>_{v1.6,auto,allver,v1.3,patches}.json`, `results/diff_<id>.json`, `results/*.time`
