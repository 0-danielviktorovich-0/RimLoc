# Каноническая модель проекта — аудит реального дата-флоу (гейт I, шаг 1)

**Дата**: 2026-09-23 · Аудит состояния рабочего дерева; все вызовы проверены grep'ом,
якоря `файл:строка`. Путь в скобках — короткое имя крейта. Примечание: в
`services/src/scan.rs` есть незакоммиченные +23 строки (интеграция patch-этапа в
`scan_units_with_defs_and_dict` / `scan_units_effective`); номера строк даны по
рабочему дереву.

## 1. Карта текущих путей данных

### 1.1 Скан (единственный производитель инвентаря)

| Этап | Что происходит | Якорь |
|---|---|---|
| Канонический вход | Keyed (`scan_keyed_xml` :375) + Defs по словарю (:377) + TKey (:383) + merge с переписыванием путей в DefInjected-target (:391) | `services/scan.rs:369` `scan_units_with_defs_and_dict` |
| Effective precedence | Keyed cross-file last-wins / in-file first, DefInjected SetOrAdd, Defs first-file-wins | `services/scan.rs:452` `apply_effective_precedence`; правила :424-441 |
| Patch-этап | apply/unsupported-подсчёт поверх Defs-инвентаря | `services/patches_effect.rs:131` `apply_patch_stage`; :43 `PatchReport`; вызовы `scan.rs:425` и `scan.rs:688` |
| Version/LoadFolders view | Keyed/DefInjected из effective-дир, Defs только из effective-корней | `services/modview.rs:143` `effective_view`; `services/scan.rs:632` `scan_units_effective` |
| Auto-вход | auto-словарь + Keyed + TKey + Defs (без precedence) | `services/scan.rs:223` `scan_units_auto` |
| Patch-кандидаты (opt-in) | текст-кандидаты патчей → юниты, путь = файл патча | `services/scan.rs:251` `scan_patches_as_units` |
| Комментарий-оверрайд | `EN:`-комментарии перезаписывают source Keyed | `services/scan.rs:294` `override_keyed_units_from_comments` |

Выход: `Vec<TransUnit>` (`core/lib.rs:54`: `key/source/path/line/tkey`), где `path`
для Defs-юнитов УЖЕ переписан в гипотетический target-пут (`Languages/English/DefInjected/…`,
`scan.rs:45`).

### 1.2 PO-путь (интерчейндж)

| Поток | Цепочка | Якорь |
|---|---|---|
| Export CLI | `run_export_po` (resolve версии :41, tm_roots из config :48-56) → `export_po_with_tm` :61 | `cli/commands/export_po.rs:24`; `services/export.rs:11` |
| Export: сборка | **Своя сборка**: keyed :19 + defs-meta :38 + TKey :77-104 + TM-карта :120-140 → `write_po_with_tm` :142 | `services/export.rs` |
| PO-запись | msgctxt = `"key|rel_path:line"` (:113-116); TM-попадание = `#, fuzzy` + prefill (:119-125); `X-RimLoc-Schema` (:85-88) | `export-po/lib.rs:55` |
| Import CLI | `read_po_entries` → группировка по reference-regex `Languages/<lang>/<rel>` :200/:395 → запись XML (fallback `Keyed/_Imported.xml` :131) | `services/import.rs:72,128,337`; `import-po/lib.rs:14,159,181` |
| Build из PO | plan/execute/progress → About.xml (:259-268) + файлы по той же reference-regex :244-245 | `services/build.rs:181,210,232`; `import-po/lib.rs:296-437` |
| Export GUI | **Дублирующая сборка в самом IPC-хендлере**: keyed :1693 + defs :1703 + TM :1759 → `write_po_with_tm` :1775; **TKey-блока нет** (0 совпадений `scan_defs_tkey`) | `gui/…/main.rs:1569` |
| CLI translate → PO | Ручная эмиссия msgctxt/msgid/msgstr строкой + `write_atomic` :181 | `cli/commands/translate.rs:158-181` |

Расхождение 1.2 — главный факт аудита: одна и та же «сборка инвентаря для PO»
существует в трёх копиях (`export.rs`, GUI :1693-1775, translate.rs), и GUI-копия
теряет TKey-юниты.

### 1.3 Валидация / coverage / compare

| Поток | Цепочка | Якорь |
|---|---|---|
| Validate | `scan_canonical` → канон → `rimloc_validate::validate` | `services/validate.rs:5,17,35,53,72`; `validate/lib.rs:22` |
| Placeholders cross-lang | канон → src/tgt-карты → `SourceMatcher` → сравнение наборов | `services/validate.rs:96` |
| Lists / orphans cross-lang | аналогично; TKey-алиасы не считаются сиротами | `services/validate.rs:208,300`; тесты :506-537 |
| Coverage | канон → `SourceMatcher`+`canonical_match_key` → TODO-паритет (игра 1.6) | `services/validate.rs:388`; :462-464 `INVENTORY_SEMANTICS` |
| Сопоставление | `TKeyRegistry`/`SourceMatcher::resolve_target` → `Resolution::{Matched,Ambiguous,Unmatched}` | `services/matching.rs:39,59,155,190,107`; `services/util.rs:23` |
| Compare CLI | канон ×2 (свой root на сет) → отчёт | `cli/commands/compare.rs:25,74`; `services/extras/diff.rs:154,296,439` |

### 1.4 Build (сгенерированный мод)

| Поток | Цепочка | Якорь |
|---|---|---|
| from-root | `scan_keyed_xml` (Keyed-дерево, осознанно) → regex-группировка → XML | `services/build.rs:6,87` (scan :19,:102) |
| from-PO | через `rimloc_import_po::build_translation_mod_*` | `services/build.rs:181,210,232`; `cli/commands/build_mod.rs:59,91,120` |

### 1.5 LLM-путь (`rimloc-llm`)

| Этап | Что происходит | Якорь |
|---|---|---|
| Вход | канон-скан → фильтр source-lang → дедуп по key → `TranslateUnit{id=key, source, context=file_name}` | `cli/commands/translate.rs:32,42,52-63`; `llm/provider.rs:10` |
| Движок | батчи по бюджету, retry, checkpoint/resume, strict-плейсхолдеры | `llm/engine.rs:57,74,153` (`check_pair`); `llm/checkpoint.rs:16` |
| Провайдеры | `Provider`-трейт; anthropic / openai-compat / mock; ключи keychain/env | `llm/provider.rs:56`; `llm/mock.rs:13`; `llm/lib.rs:52` KeySource, :96 env_key |
| Выход | `UnitOutcome` → **PO-строки руками в CLI** (см. 1.2) | `llm/engine.rs:40`; `translate.rs:158-181` |

### 1.6 TM, glossary, word-info, morph

- **TM**: две родословных — (а) tm_roots: `scan_keyed_xml(tm_root)` → map `key→val`, last-wins (`services/export.rs:120-140`; fallback из config `cli/commands/export_po.rs:48-56`); (б) GUI `load_tm` читает .po по msgctxt (gui main.rs:2789). TM-ключи — логические ключи Keyed; TKey/TM по target-пути фиксируется fuzzy-меткой PO.
- **Glossary**: builtin EN→RU + JSON-оверрайды (`llm/glossary.rs:11,31`); подмешивается в каждый `TranslateRequest` (`engine.rs:118`).
- **word-info**: `scan_units_auto` → диагностика склонений; capability жёстко по языку (ru/de/uk) (`services/extras/wordinfo.rs:27,55`).
- **morph**: генерация словоформ (`services/extras/morph.rs`, `MorphProvider`).

### 1.7 Version-diff, sourceChanged, существующие переводы

| Поток | Состояние | Якорь |
|---|---|---|
| version-diff | два `resolve_game_version_root` + канон-скан на версию → diff-отчёт | `cli/commands/version_diff.rs:14-23`; `services/extras/version_diff.rs:59` |
| sourceChanged | **НЕ реализовано** — только placeholder-комментарий «диагностика попадёт сюда» | `services/matching.rs:188` |
| Scan чужого языкового пака | тот же канон-скан, фильтр по целевой языковой папке (GUI/CLI scan, XLIFF-экспорт) | `gui main.rs:1310-1313`; `cli/main.rs:1279-1284` |
| lang_update | скачать zip-перевод игры → применить в `Data/Core/Languages` | `services/extras/lang_update.rs:185`; `cli/commands/lang_update.rs:16` |
| merge_keyed | слияние Keyed-наборов (GUI `merge_keyed_gui`) | `services/keyed_merge.rs`; `gui main.rs:411` |
| Strings/RulePack | GUI `scan_strings_gui`: WalkDir `Languages/*/Strings/*.txt` — вне TransUnit-модели | `gui main.rs:725` |

### 1.8 GUI IPC (сводка, `gui/tauri-app/src-tauri/src/main.rs`)

`scan_mod` :1123 (канон :1284 + плагины :1292 + патчи :1302) · `export_po` :1569
(дубль-сборка, см. 1.2) · `validate_mod` :1854 (:1919 + cross-lang :2000,:2030) ·
`import_po` :2188 (`import_po_to_file` :2229) · `build_mod` :2309 (:2342,:2353,:2376) ·
`coverage_gui` :467 · `export_xliff_gui` :502 / `import_xliff_gui` :536 ·
`validate_po_gui` :2558 · `load_tm` :2789 · `learn_defs` :1413 / `learn_patches_cmd` :2894 ·
`xml_health` :2092 · `diff_xml_cmd` :3239 (:3297,:3307). CLI-XLIFF отдельно:
`scan_units` + hardcode `"en"` (`cli/main.rs:1279-1284`, :349 импорт).

**Проектного состояния (Project) в коде нет**: `grep project.json/.rimloc/ProjectState`
— только `~/.rimloc/plugins-allow.json` (plugins) и дефолтный packageId
(`build_mod.rs:32`). Конфиг — разовый TOML (`rimloc-config`), не состояние перевода.

## 2. Классификация зависимостей

Классы: DOMAIN REQUIREMENT (семантика RimWorld/перевода), ADAPTER CONCERN (знание
файл-лейаута игры — место адаптера), TRANSPORT CONCERN (формат интерчейнджа),
LEGACY COUPLING (копия/историческое сцепление).

| Место | Зависимость | Класс | Решение |
|---|---|---|---|
| `core/lib.rs:81` TKeyMeta (strategy/suffix/def_type/contexts) | сериализационные метаданные TKey | DOMAIN | KEEP — ядро SourceEntry-метаданных |
| `services/scan.rs:452` `apply_effective_precedence` + `patches_effect.rs:131` | эффективная семантика игры | DOMAIN | KEEP — eligibility/resolver-слой |
| `services/matching.rs:107` `Resolution`, :96 `MatchOrigin` | доказанное сопоставление target→source | DOMAIN | KEEP — evidence в Translation |
| `services/validate.rs:462-464` semantics-константы | методология ≠ транспорт | DOMAIN | KEEP |
| `core/lib.rs:54` TransUnit.key | ключ = serialized DefInjected-путь; identity target-зависим через переписанный `path` (`scan.rs:45`) | DOMAIN/ADAPTER | MIGRATE — stable identity отдельно от сериализации |
| `services/scan.rs:9` `DEFAULT_SOURCE_LANG_DIR="English"`; `export.rs:22-28`; `cli/main.rs:1284` hardcode `"en"` | EN-допущение | ADAPTER | MIGRATE — в InventoryContext/языковые пары |
| `import-po/lib.rs:194` `rimworld_lang_dir`; `services/util.rs` `is_*_lang_dir`; содержательные `contains("/Keyed/")`-проверки (`scan.rs:457-463`, `build.rs:22-27`, GUI) | файл-лейаут Languages/ | ADAPTER | MIGRATE — в RimWorld-адаптер |
| `export-po/lib.rs:113-125` msgctxt `"key|path:line"`, fuzzy, `X-RimLoc-Schema` :85-88 | PO-кодирование identity | TRANSPORT | KEEP — но в адаптере, не в домене |
| `import.rs:200,395` + `build.rs:17,98,244` | пять копий reference-regex «Languages/&lt;lang&gt;/&lt;rel&gt;» | TRANSPORT/LEGACY | MIGRATE — один PO-адаптер |
| `import-po/lib.rs:116,131` fallback `Keyed/_Imported.xml` | тихая потеря структуры | LEGACY | REMOVE (гейт I: отказ или явный warning) |
| `core/lib.rs:93` PoEntry, :110 `parse_simple_po` | PO-типы/парсер в core | LEGACY | REMOVE — в import-po/export-po |
| `core/lib.rs:102` RimLocError «for crates that still import it» | мёртвый шов | LEGACY | REMOVE |
| `services/export.rs:11` сборка PO (GATE A-копия) + `gui main.rs:1693-1775` дубль **без TKey** + `translate.rs:158-181` ручной PO | три сборки одного экспорта | LEGACY | MIGRATE — единый PO-адаптер над инвентарём |
| `scan.rs:399` env `RIMLOC_FUZZY` | скрытый флаг в сервис-слое | LEGACY | REMOVE (опция в конфиг/CLI) |
| `llm/glossary.rs:11` builtin EN→RU; `llm/engine.rs:26-36` дефолт en→ru | EN-RU-допущение | ADAPTER | MIGRATE — языковые пары Project |
| `extras/wordinfo.rs:27` capability по ru/de/uk | знание языков игры в сервисе | ADAPTER | MIGRATE |
| `modview.rs:143` + `scan.rs:632` | version-view без активных модов/load order | DOMAIN (неполный) | KEEP + расширить до InventoryContext |

## 3. Целевая архитектура (из мандата)

```
RimWorld-адаптеры (скан XML, сериализация, лейаут, lang-дир)
        ↓
effective content resolver (версия/LoadFolders/патчи — modview + patches_effect)
        ↓
eligibility engine (словари/learn → типизированный компонент, гейт J)
        ↓
канонический SourceEntry inventory  ← InventoryContext (версия/DLC/моды/load order)
        ↓
Project / Translation state (языковые пары, статусы, provenance, sourceChanged)
        ↓
TM · glossary · LLM · validation   (все на канонических юнитах)
        ↓
сервис-слой (сегодняшние services/*) → GUI · CLI · MCP
        ↓
выходные адаптеры (RimWorld-мод · PO · JSON · XLIFF · CSV)
```

PO = интероп-формат на границе, не доменное состояние. Существующие типы уже
закрывают часть стека: `modview::EffectiveModView` (modview.rs:15) — зародыш
InventoryContext; `PatchReport` (patches_effect.rs:43) — patch-provenance;
`CoverageMethodology` (validate.rs:376) — методологическая идентичность.

## 4. Требования к модели (спека для шага 2)

**SourceEntry** — стабильная запись источника:
- `stable identity`: target-независимый id (не serialized путь); serialized пути —
  производные, через `TKeyMeta.suffix` (`core/lib.rs:81`); `source hash` для
  sourceChanged.
- `EntryKind`: Keyed · DefInjected · TKey · Strings/RulePack · Backstories ·
  Patch-derived (сегодня все сводятся к key+path, кроме Strings — `gui main.rs:725`).
- `source locale`, `source contexts` (мульти-контекст: несколько узлов на identity
  — уже считается в `TKeyMeta.contexts`), `canonical location` (path/line),
  `version context`.
- `provenance` enum: `raw | version-selected | conditional-branch |
  patch-transformed | overridden` — закрывает «известный хвост» Gate H
  (CANONICAL_INVENTORY.md §Provenance и §Known tail).
- `serialization metadata` = `TKeyMeta`; `eligibility evidence` = основания
  включения (сейчас — словари, `scan.rs:180`).

**Translation**: `source_entry_id` · target locale · text · статус как размерности
(completeness / review / validation / lifecycle — не один флаг; сегодн. proxy —
TODO-паритет `validate.rs:438`, fuzzy PO) · `provenance` (human | TM | LLM |
imported — сегодня выводится из пути/флага) · notes · `sourceChanged`
(detected via source hash; точка подвеса уже оставлена в `matching.rs:188`).

**Project**: языковые пары · mod-корпус · `InventoryContext { target version,
active DLC, active mods, load order, view label Exact|Potential }` · персистентное
состояние (reopen) — сегодня отсутствует (см. 1.8).

**Существующие типы, которые переиспользуются**: `TransUnit` (`core/lib.rs:54`),
`TKeyMeta` (:81), `Resolution`/`MatchOrigin` (`matching.rs:107,96`),
`PatchReport`/`UnsupportedOp` (`patches_effect.rs:43`), `EffectiveModView`
(`modview.rs:15`), `CoverageReport`/`CoverageMethodology` (`validate.rs:361,376`),
`ImportSummary` (`rimloc-domain`), `TranslateUnit`/`UnitOutcome`
(`llm/provider.rs:10`, `llm/engine.rs:40`).

## 5. Acceptance-гейт Gate I

Три workflow на ОДНОЙ модели — бизнес-логика одинакова, различаются только
адаптеры входа/выхода:

- **A. native no-PO**: source (RimWorld-адаптер) → Project → MockProvider
  (`llm/mock.rs:13`) → validate → build. Ни одного PO-объекта в цепочке.
- **B. PO interop**: Project → PO-адаптер → внешняя правка .po → PO-import →
  Project → build. Import обязан распознавать записи, созданные export'ом (msgctxt
  round-trip), и размечать их `provenance=imported`.
- **C. existing translation**: source + существующий языковой пак (scan целевой
  папки, `gui main.rs:1310-1313`) → Project: уже-переведённое сохраняется
  (preserve), расхождения — review, пустое/TODO — missing (`validate.rs:438`) →
  build.

Гейт закрыт, если: (1) все три идут через один SourceEntry-инвентарь и один
Project-state — разница только в адаптерах; (2) TM (`export.rs:120-140` +
GUI `load_tm` :2789), glossary (`llm/glossary.rs:11`), LLM (`translate.rs:32`) и
валидация (`validate.rs:5`) работают на канонических юнитах без сборки
промежуточных файлов; (3) project reopen восстанавливает состояние; (4) дубли из
§1.2 (export.rs / GUI :1693 / translate.rs:158) сведены к одному PO-адаптеру —
включая утерянный в GUI TKey-блок.

**Fidelity-матрица адаптеров** (RimWorld/PO/JSON/XLIFF/CSV; колонки:
identity · source text · TKey meta · provenance · статусы) — заполняется
фактами после реализации шага 2; скелет фиксируется здесь, чтобы гейт был
проверяемым.
