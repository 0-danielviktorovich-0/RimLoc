# RELEASE_PARITY_MATRIX — реконсиляция wave 3/5/6 competitive research против current main

**Дата:** 2026-10-07 · **Лейн:** parity (WF-PARITY-RECONCILE, full-release-convergence)
**База:** current main = `49a9eb8` (ba-main worktree, ветка main; к моменту сдачи HEAD дошёл до `897eedc` — только docs-state, вся кодовая база идентична). Предыдущая реконсиляция была на `28ae9c1` — с тех пор **37 коммитов**, включая merge PR #80 (release/wave-integration: security sweep, canonical no-PO translate, **edge-trim**, UI pass, Compare screen).
**Метод:** только reconcile документов с кодом. Конкуренты и бинарь RimLoc НЕ перезапускались (мандат); все числа конкурентов взяты из отчётов волн как есть. Проверки этого захода: `git log`/`git show`, чтение исходников RimLoc, дамп `defs_fields.json`, **исполнение пяти тестовых прогонов** (см. §0). GUI-поверхности (Compare screen, Source Inspector) — чтение кода/коммитов, не запуск приложения.

**Источники:** `docs/competitive/TIER_A_COMPLETION_MATRIX_2026-10.md` (финальные вердикты §4), wave-отчёты `docs/competitive/differential/` (TIERA_PYTHON, TIERA_CS, REMIS, TIERA_GRABBER_RTAI, TIERA_PYSMALL, TOKCDK), `roadmap/TIERB_ADAPTER_REQUIREMENTS_2026-10.md`, `roadmap/TIERCDE_REFERENCE_2026-10.md`, `docs/development/CHAT_BATCH_MANDATE.md`. Предыдущая версия этой матрицы: `/tmp/wf-comp/RELEASE_PARITY_MATRIX.md` (база 28ae9c1).

---

## 0. Фиксы на main после предыдущей реконсиляции — верифицировано исполнением

| # | Фикс | Evidence в main (49a9eb8) | Проверка этого захода |
|---|---|---|---|
| 1 | **edge-trim (бывший I1, единственный IMPLEMENT_NOW)** | коммит 9dc9a7c «fix(parsers): Keyed-значение = точный текст минус отступы (I1)»; `ws_run_is_formatting` → `crates/rimloc-parsers-xml/src/lib.rs:81`, `commit_text_chunk` `:92`, `commit_ref_chunk` `:129`, `close_value_buffer` `:151` на всех emit-сайтах (`:374`, `:406`, `:460`); inline-элементы сериализуются обратно с атрибутами; CDATA без трима | `cargo test -p rimloc-parsers-xml edge_trim` → **7 passed** (`edge_trim_tests`, `lib.rs:3279`: однострочный край, многострочный indent, HugsLib A/B, inline round-trip, attrs, self-closing, CDATA) |
| 2 | **Gate H (PR #77)** | `export-po --game-version` через modview effective_view → `crates/rimloc-cli/src/commands/export_po.rs:44-60` (root-first view order); регресс-тест `cli_integration.rs:253` | `cargo test -p rimloc-cli --test cli_integration export_po_game_version_loadfolders_keeps_root_keyed` → **1 passed** |
| 3 | **newest-wins (PR #78)** | `effective_view` → `crates/rimloc-services/src/modview.rs:209`; тесты `cli_integration.rs:320`, `:485` | оба теста исполнены → **2 passed** (`scan_plain_version_dirs_pick_newest_version_per_key`, `build_mod_from_root_newest_version_wins_per_key`) |
| 4 | **canonical no-PO translate (PR #80)** | `crates/rimloc-cli/src/commands/translate.rs:1-9` («LLM translation into CANONICAL project state… PO is no longer the internal seam»), `--out-json`/`--managed-root`/`--emit-po` `:118-124`; ApplyOrigin `crates/rimloc-services/src/contract.rs:221-230` (Human/Llm/Tm/Import), `ApplyIntentsRequest` `:234` | `cargo test -p rimloc-cli --test canonical_no_po` → **3 passed** (`no_po_1_scan_project_apply_validate_build`, `no_po_2_mock_translate_review_build_without_po`, `po_interop_canonical_export_import_keeps_identity`) |
| 5 | **Compare screen (PR #80)** | version_diff_scan → `crates/rimloc-services/src/extras/version_diff.rs`; `contract_version_diff` (dc2e181, 3906044, 2c6555b, fdaf83e, 8f09bcf); CHANGELOG: live version-compare, exact counts, honest 200-cap | чтение коммитов + CHANGELOG; GUI runtime не запускался |
| 6 | **UI-проход (PR #80)** | i18n ключей палитры, LM-ошибок кодами, локали по targetLocale, возврат фокуса, снос мёртвого version-select (15aba01…74052ca) | CHANGELOG Fixed; перечитано, не исполнялось |
| 7 | **security sweep (PR #80)** | отказ hostile PO `#:`-references (a9b2537), lru 0.18.5 (f826df7), vendor cleanup (03d6921); нулевой open-security гейт верифицирован security-лейном на 3477a37 | чтение; security-лейн закрыл отдельно |

Итого: **единственный IMPLEMENT_NOW предыдущей матрицы (I1 edge-trim) закрыт и подтверждён тестами**. Все MUST_FIX_BEFORE_BETA матрицы Tier A (§4.1-4.5) теперь закрыты или переклассифицированы: 4.1 разметка+тримминг → FIXED (§0.1); 4.2 версии → FIXED (§0.3); 4.4 «регрессия 6465813» → переклассифицировано и FIXED (§0.2); 4.5 патч-слой → CLOSED; 4.3 IfModActive → HIGH_VALUE over-включение (не MUST_FIX).

## 1. Выводы волн, которые УСТАРЕЛИ на 49a9eb8

1. **«Хвостовой пробел триммится (`Search: ` → `Search:`)» (w5-remis SEMANTIC 8/13, w6-pysmall 7 ключей, MUST_FIX №1 матрицы)** — устарело ПОСЛЕДНИМ: 9dc9a7c сохраняет краевые пруны однострочника, режет только перенос+indent, и вдобавок чинит потерю контента реальных inline-элементов внутри значения (раньше `<b>…</b>` внутри значения давал пустое значение). 7 тестов зелёные этим заходом.
2. Все прочие устаревания из предыдущей реконсиляции (§1 там) остаются устаревшими: патч-слой 54/54, msgid-разметка, слияние версий, «регрессия 6465813», числовой workshop-id, out-dir у learn-стадий.
3. Валидное из волн: IfModActive over-включение (PARITY P4), Defs/KeyBindingDef вне дефолтного export-po (P5/R11), dict-gap словаря (R11 — подтверждено дампом словаря), stages.N-индексы (R7), ParentName в learn-defs отсутствует (R8 — подтверждено grep), TM-транзакционность (R6), reflection (R1 — подтверждено: кода нет).

## 2. Проверка 12 ключевых областей против кода (по заданию)

| Область | Что нашлось в коде (49a9eb8) | Вердикт-следствие |
|---|---|---|
| 1. Defs gaps (gerund/verb/labelNoun/baseInspectLine/structureLabel/stuffAdjective/adjective/summary/ideoName/pawnsPlural/KeyBindingDef.label) | `crates/rimloc-parsers-xml/assets/defs_fields.json` — **49 def-типов**; дамп этого захода: **ни одного** из перечисленных полей нет (ThingDef имеет `gerundLabel`/`verbs.li.label` — но не «verb»); KeyBindingDef и IdeoDef **отсутствуют как типы**; FactionDef имеет `pawnSingular` без `pawnsPlural`; WorkTypeDef только label/description. Словарь embedded `lib.rs:952`. Расширение пользователем: `--defs-dir`/`--defs-field`/`--defs-dict` → `scan.rs:74-76,149-190`, эвристика `--fuzzy` | R11 (REJECT post-release): гэп реален, мост флагами есть |
| 2. IfModActive/IfModNotActive/conditional dirs | `modview.rs:161-165` — парсятся `IfModActive`+`IfModActiveAll`+`IfModNotActive`; `conditional_dirs` `:22`; `effective_view` `:209`; provenance `conditional_branch` → `canonical.rs:105` | P4 PARITY |
| 3. ParentName/inheritance | Основной scan: same-file цепочка `lib.rs:824-875`, cross-file helper «shallow-field inheritance across files by ParentName» `:1393-1448`, ещё сайты `:1703`, `:1812-2016`. **learn-defs: grep ParentName в `crates/rimloc-services/src/learn/` → пусто** | Наследование в ядре есть (R8 остаётся только про learn-defs) |
| 4. Reflection (Mono.Cecil) | `grep -rni "mono.cecil\|cecil" --include="*.rs" --include="*.toml" crates/` → **0** (упоминания только в competitive-доках); `reflection` по crates → 0 | R1 REJECT: только research |
| 5. TM: sourceChanged exclusion, ambiguity refusal, ranking, batch apply | Модель `crates/rimloc-domain/src/tm.rs:26-46` (ключ `(source_text, target_locale)`, изоляция локалей, auto/import/manual политики), rank `:59`, `bounded_levenshtein` `:181`, `fuzzy_threshold` `:219`; ranking `tm_ranked_matches` → `session.rs:2693-2738` (tier exact→normalized→fuzzy, потом status.rank); CRUD+import CSV/JSON `session.rs:939-1187`; `tm_lookup` `:1193`. sourceChanged: поле `canonical.rs:188`, пайплайн `mark_source_changed` `:365`, сессия `session.rs:95,319-336`. Ambiguity refusal: `ResolutionStatus::Ambiguous` `canonical.rs:222-223`, «ambiguous lines are never overwritten» `session.rs:2290`, existing-анализ conflicts/obsolete/ambiguous/invalid `:2190-2437`. Batch apply: `ApplyIntentsRequest` `contract.rs:234`, `Capability::ProjectApplyIntents` `:394`, батч-применение с origin `session.rs:595-607`, cancel-next тест `:4603` | Входит в S6 (TM) и P7 |
| 6. Glossary: validation against target, import/export, consistency | Канонический глоссарий проекта: `canonical.rs:307-310` (wave 13), CRUD `session.rs:727/751/819`; LLM-глоссарий builtin 15 терминов + user JSON override → `crates/rimloc-llm/src/glossary.rs:13-48`; **prompt-enforced** (`prompt.rs:16` «apply the provided glossary strictly», инжекция `:101,117`). Пост-валидация вывода против терминов — **нет**: `crates/rimloc-llm/src/validator.rs` проверяет только плейсхолдеры (printf/braces/[VARS]/lookup-редукция `:13-50`). Отдельного import/export-командного шва у глоссария нет (состояние в каноническом проекте, JSON-load на LLM-стороне) | R20 REJECT post-release (consistency-check); сам глоссарий — часть S6 |
| 7. Validation catalog (§6.5) | 8+ видов в коде: `empty` (Error) `crates/rimloc-validate/src/lib.rs:119`; `placeholder-check` (Error: bad %, несбалансированные/пустые/невалидные braces; Info-хинт) `:141-232`; `invisible-char` (Warning: ZWSP/bi-di, в тексте И в ключах) `:47-108`; `duplicate` (Error, same-file) `:262-275`; `duplicate-global` (Warning, cross-file в одном scope) `:279-313`; `placeholder_set_mismatch` (строгий сет {name}-токенов источник≠перевод) `:320+`; services-слой: `non-translatable-flagged` (Error) `services/validate.rs:44`; cross-language `placeholder-check` (Error) `:229` + `ambiguous-key` (Warning) `:242`; `list-mismatch` (Error) `:329`; `orphan` (Warning) `:410`. Плюс typed severity, support-bundle, SARIF-совместимый JSON | P7 PARITY |
| 8. WordInfo/Morphology | `crates/rimloc-services/src/extras/wordinfo.rs:19-35` — capability Full/None (Gender/Case/Plural только для языков с WordInfo), `diagnose` `:111` (переведённые label без WordInfo-покрытия), `scaffold_case` `:150` (`word;?;?;?;?;?` — 6 русских падежей). Morph: провайдеры Dummy/MorpherApi/Pymorphy2 → `commands/morph.rs:6-18`; `generate` `extras/morph.rs:166`, pymorphy-declension `:104`, выходы `_Case.xml`/`_Plural.xml`/`_Gender.xml` `:252-266` (Case/Plural/Gender) | R18 NON_GOAL-часть; русская ветка работает |
| 9. Chat batch: canonical→selection→export→import→apply | Каноническая половина живая и E2E-протестирована (см. §0.4). **Батч-менеджер по мандату `docs/development/CHAT_BATCH_MANDATE.md` (665 строк, G5-W3): grep `ChatBatch\|chat_batch\|AI_CHAT` по crates/gui → пусто.** Нет: per-batch статусы (NOT STARTED/…/DONE §4), стабильная batch-идентичность (§5), AI_CHAT-provenance (§16 — ApplyOrigin не имеет такого класса `contract.rs:221-230`), sourceChanged→STALE_BATCH гейт на импорте (§13), [Copy next batch] | **I1 IMPLEMENT_NOW** |
| 10. AI provider: batching, retry, cancellation, checkpoint/resume | Batching: `batch_char_budget` 6000 → `crates/rimloc-llm/src/engine.rs:19,31`, `batch_by_budget` `:220-237`, цикл `:112`. Retry: RateLimited → sleep(retry-after, cap 5s) `engine.rs:125-131`; retry-after парсится в `anthropic.rs:49` и `openai_compat.rs:82`. Checkpoint/resume: append-only JSONL, «one record per accepted unit», replay `load_completed` → `crates/rimloc-llm/src/checkpoint.rs:1-40`. Cancellation: batch-уровень в сессии (JobCancel capability `contract.rs:396`, cancel-next `session.rs:4603`); **mid-batch отмены провайдера нет** (синхронный engine) | P6 PARITY; mid-run cancel — оговорка |
| 11. Source Inspector: provenance, selected_by, version, contexts | `crates/rimloc-core/src/lib.rs:74` `SourceRef{file,line}` («No fabricated line numbers»), `:83` `selected_by`, словарь winner_reason `:118+` (first-file-wins / keyed-last-wins / keyed-first-in-file / tkey-last-assignment / definjected-setoradd / patch-applied); `SourceProvenance{version_selected, conditional_branch, …}` → `canonical.rs:99-119`; мульти-контексты `contexts: Vec<SourceContext>` `:158` (Primary + Other usages, TKey-узлы) | S-часть provenance-стека; GUI-вью чтением кода (W7 CHANGELOG) |
| 12. Existing translation: versioned source, obsolete, TKey, sourceChanged | Versioned source: `version_selected` `canonical.rs:102` + effective_view. Obsolete: `ExistingPackAnalysis` conflicts/obsolete → `crates/rimloc-services/src/project.rs:327-465` (`:362-370` поля, `:438/:449` наполнение), `Lifecycle::Obsolete` `:550,1434` («vanished def type only obsoletes its own work» тест `:1205-1257`). TKey: `TKeyRegistry` (identities + proven aliases, suffix-стрип только для известных TKey) → `crates/rimloc-services/src/matching.rs:33-60`; winner `tkey-last-assignment`. sourceChanged: §5-строка выше | Всё живое, часть S6/P7 |

## 3. Матрица релизного паритета — материально релевантные capability Tier A/B

Вердикты: SUPERIOR · PARITY · IMPLEMENT_NOW · REJECT_NOT_PRODUCT_RELEVANT · EXTERNAL_BLOCKER.

### SUPERIOR (9)

| # | Competitor | Capability | Competitor evidence | RimLoc current (файл:строка) | RimLoc evidence |
|---|---|---|---|---|---|
| S1 | Remis | Покрытие полей Defs | w5-remis §4.2: зашитый каталог `rimworld-1.6-v1` теряет `title*`×119 и `reportString`/`deathMessage`×67 на двух модах | `crates/rimloc-parsers-xml/assets/defs_fields.json` — 49 typed-типов (title*-семейство, reportString, deathMessage, pawnSingular, leaderTitle…); embedded `crates/rimloc-parsers-xml/src/lib.rs:952`; расширение `--defs-field/--defs-dict` → `crates/rimloc-cli/src/commands/scan.rs:74-76` | дамп словаря этим заходом: 49 типов, 35+ полей |
| S2 | Grabber GUI, RimTransAI, RimTrans-zh | Чтение существующего English-DefInjected как источника | w5: RIMLOC_ONLY 171 на VE у обоих (DefInj=0); w3-cs: RIMLOC_ONLY 370 у RimTrans-zh | export-po/scan включают существующие DefInjected-папки дефолтом (тест `scan_keyed_nested_definj_drops_def_type` в parsers) | прогон parsers-тестов этим заходом (28 ok) |
| S3 | RimTransAI | Скан без установленной игры | w5 §2: `ModParserService.cs:249-253` — без Assembly-CSharp.dll return null | RimLoc сканирует XML без игры; grep Assembly-CSharp/Mono.Cecil по crates → 0 | grep этого захода |
| S4 | N89, RimTrans_PY, Remis, Grabber | Патч-слой | w3 §3: N89 100 refs = 54 видимых + 46 шума; w5: RimTrans_PY 26 ⊂ RimLoc 54; Remis 0; Grabber version-баг | learn-patches whitelist+blacklist (`patches_extract.rs:82-89` по пред. реконе; юнит-тесты `tests/patch_extract.rs` — **2 passed этим заходом**); `scan --with-patches` → юниты: `crates/rimloc-cli/src/commands/scan.rs:85`, `:294-307` (`scan_patches_as_units`) | чтение scan.rs + прогоны тестов |
| S5 | Grabber, RimTransAI, laskinss27, TokcDK | Версионная семантика (newest-wins + приоритет --game-version) | w5: Grabber `1.5_1.6` молча 1.4 (5 vs 213 полей); RTAI first-wins канонизирует 1.5; laskinss27/TokcDK сваливают все версии (~1300 vs 764) | per-key newest-wins + `--game-version` + Gate H: `crates/rimloc-services/src/modview.rs:209`, export-путь `export_po.rs:44-60` | **тесты исполнены этим заходом: 3 passed** (cli_integration.rs:253, :320, :485) |
| S6 | Весь прямой сет | CAT-стек (PO/XLIFF round-trip + TM + глоссарий + validate) | w3/w5: PO round-trip только у спящего RimTranslate; Remis — глоссарий без TM; полного аналога нет | `crates/rimloc-export-po`, `rimloc-import-po`, `rimloc-export-xliff`, `rimloc-import-xliff`; TM: `crates/rimloc-domain/src/tm.rs:26-46,59,181,219` + `session.rs:939-1187,1193,2693-2738` (ранжированный lookup: exact→normalized→fuzzy, rank статуса); глоссарий: `canonical.rs:307-310`, `session.rs:727-819`; validate: каталог §2.7 | все файлы прочитаны этим заходом |
| S7 | Весь прямой сет | In-game верификация | матрица §6.4: level 7 IN_GAME_PROVEN не присвоен никому | Runtime Bridge — level 6.5 proven; level 7 = owner gate | `.rimloc-release-state.json` gates.real_rimworld (прочитан) |
| S8 | Remis, Text Grabber | Безопасность записи | w5-remis §6.1: symlink-баг LoadFolders → ValueError воспроизведён; TG пишет внутрь папки мода | канонический write-guard (e9d3680), symlink-safe atomic writes, adversarial-тесты `write_guard_adversarial.rs` | carried из пред. реконе (тесты не перезапускались) |
| S9 | lenhare, Remis | Хранение секретов | w6 матрица 3.22: lenhare закоммитил google_credentials.json; Remis — ключи в своём конфиге | keyring v3 apple/windows-native за feature `keychain`: `crates/rimloc-llm/Cargo.toml:27`, `secrets.rs:36-83`. Оговорка: CLI-дефолт-сборка без фичи | grep+Cargo.toml этим заходом |

### PARITY (7)

| # | Competitor | Capability | Competitor evidence | RimLoc current (файл:строка) | RimLoc evidence |
|---|---|---|---|---|---|
| P1 | Все лейны | Keyed-извлечение | BOTH 75/593/40 (HugsLib/VE/Anomaly) у всех инструментов | полное покрытие ядра Keyed; после PR #77/#78 ни одна корзина Keyed-слоя не теряется | Gate H + newest-wins тесты — passed этим заходом |
| P2 | Remis | Резолв LoadFolders v1.x-веток | w5 §4.4: Remis корректно исключает 1.4 на ветке v1.6 | та же семантика (view order root-first, `export_po.rs:51-56`) + Gate H | тест Gate H — passed |
| P3 | Remis, RimTranslate, RWAT, rimwt | Сохранность сырого текста (разметка, entities, краевые пробелы) | все хранят raw; w5-remis SEMANTIC 8/13 + w6-pysmall 7 ключей — кра-трим | 9dc9a7c: значение = точный текст минус newline-indent (`lib.rs:81` `ws_run_is_formatting`, `:151` `close_value_buffer`); `Search: ` сохраняет пробел, inline-элементы round-trip с атрибутами, CDATA без трима | **7 edge_trim-тестов passed этим заходом** |
| P4 | RimTransAI (эталон), Remis | IfModActive/условные папки | w5: RTAI резолвит по ActivePackageIds; Remis без active_mods валит scan целиком | RimLoc включает условное безусловно и помечает: `modview.rs:161-165` (все три атрибута), `conditional_dirs` `:22`, `conditional_branch` `canonical.rs:105`; классификация кампании HIGH_VALUE over-включение | чтение modview.rs/canonical.rs |
| P5 | Grabber, RTAI, RimTrans_PY, rimwt, Remis | Defs-слой в конвейере | w6: четверо конкурентов берут KeyBindingDef.label ×7 в основной конвейер | capability доступна сегодня: learn-defs + `--defs-dir/--defs-field/--defs-dict/--fuzzy` (`scan.rs:74-76,149-190`); дефолт export-po держит Defs на корне мода (`export_po.rs:44`); классификация HIGH_VALUE_AFTER_BETA | флаги верифицированы в scan.rs |
| P6 | Remis (12+ MT-роутер), N89 (Argos offline) | MT/provider-слой | w5/w6: MT-стадии конкурентов BLOCKED_DEPENDENCY, не прогонялись | providers anthropic/openai-compat/mock + batching/retry/checkpoint (`engine.rs:112-131`, `checkpoint.rs`), keychain, prompt-injection defense; роутера и оффлайн-провайдера нет — ROADMAP 19 | чтение engine.rs/checkpoint.rs |
| P7 | RWAT, Remis | QA-валидация перевода | RWAT Alias/структурные чеки; Remis token-валидация (w5 §5) | каталог из 8+ видов проверок (§2.7): empty, placeholder-check, invisible-char, duplicate, duplicate-global, placeholder_set_mismatch, non-translatable-flagged, ambiguous-key, list-mismatch, orphan + severity + support-bundle | validate-crate и services/validate.rs прочитаны; поведенческого сравнения с конкурентами не делали — паритет консервативно |

### IMPLEMENT_NOW (1)

| # | Competitor | Capability | Competitor evidence | RimLoc current (файл:строка) | Почему реализовывать сейчас |
|---|---|---|---|---|---|
| I1 | — (прямого аналога в Tier A/B нет; это owner-мандат G5 RimLoc) | Chat-batch manager: batch identity, per-batch статусы (NOT STARTED→DONE), [Copy next batch], provenance AI_CHAT, sourceChanged→STALE_BATCH гейт импорта | мандат `docs/development/CHAT_BATCH_MANDATE.md` (665 строк, G5-W3 от 2026-09-24): «supported product workflow, not a hidden JSON export trick» | Половина пайплайна живая и E2E-протестирована: canonical entries→selection→apply (`translate.rs:1-60`; `ApplyIntentsRequest` `contract.rs:234`; origin-классы `:221-230`; 3 теста `canonical_no_po.rs` — passed). Экспорт/импорт — через PO interop (`--emit-po`/import-po). **Отсутствует по коду**: grep `ChatBatch\|chat_batch\|AI_CHAT\|STALE_BATCH` по crates/gui → пусто; per-batch статусов нет, batch-идентичности нет, AI_CHAT-класса в ApplyOrigin нет (§2.9) | (1) Владельческий мандат в активной конвергенции (release-state: provider-лейн «chat-batch» — running). (2) Ничто внешнее не блокирует: примитивы все есть (ApplyIntents-батчи, sourceChanged-детекция `canonical.rs:188,365`, severity-валидатор, PO/XLIFF-interop). (3) Не competitor-parity gap — потому гейт `parity_superior_parity_only` это не задерживает, но релизная полнота продукта — да |

### EXTERNAL_BLOCKER (0)

Нет. MT-ключи — BYO-модель; платформенные ограничения конкурентов — их.

### REJECT_NOT_PRODUCT_RELEVANT для этого релиза (20)

| # | Competitor | Capability | Причина (с проверкой кода, где применимо) |
|---|---|---|---|
| R1 | RimTransAI | Reflection-extraction из C#-сборок (712 ReflectionField + 192 типа DLL на VE) | ROADMAP; **grep Mono.Cecil/cecil/reflection по crates → 0** — только research, прототипа нет; требует DLL игры — противоречит S3 |
| R2 | Remis | Glossary-health score + advisory AI | ROADMAP («приятно, не барьер») |
| R3 | Remis | Пофайловый атомарный чекпойнт перевода (schema v3, resume с fingerprint) | HIGH_VALUE пост-релиз; у RimLoc свой срез: append-only JSONL `checkpoint.rs:1-40` + атомарный project envelope — перевод одного мода не блокируется |
| R4 | rtl-tools | RTL-публикация | ROADMAP; 0 RTL-строк в корпусе |
| R5 | TokcDK | tar-архивы языков как источник | ROADMAP; ветка на корпусе даже не исполнялась |
| R6 | RWAT | TM-транзакционность (multi-target snapshot, ConcurrentPaths, recovery) | ROADMAP 18; атомарность конверта + symlink-safe writes закрывают класс порчи |
| R7 | Remis, TG, MTT | Индексированные `stages.N.label`, rulesStrings-индексы | HIGH_VALUE пост-релиз; контент доступен через learn-defs (`stages.li.label` в словаре: HediffDef/ThoughtDef — дамп этого захода) |
| R8 | Grabber | ParentName-резолв в learn-defs | HIGH_VALUE пост-релиз; **в основном извлечении наследование есть** (`lib.rs:824-875`, `:1393-1448` — grep этого захода), нет только в learn-defs (`crates/rimloc-services/src/learn/` → пусто) |
| R9 | RWAT | Фильтр мусор-кандидатов (`'-1'`, `'{0}: {1}'`, RGB) | HIGH_VALUE пост-релиз; ~6 записей на мод, не порча |
| R10 | Translation Forge | Workflow сопровождения пакета check/stale/import | ROADMAP 17; срезы есть (diff-xml, annotate, xml-health, init, lang_update) |
| R11 | TokcDK (75/26), Grabber (62), MTT (~38) | Дозаполнение dict-gap: verb/labelNoun/baseInspectLine/structureLabel/stuffAdjective/adjective/summary/ideoName/pawnsPlural/KeyBindingDef.label/WorkTypeDef.gerundLabel | HIGH_VALUE_AFTER_BETA; **дамп defs_fields.json этого захода: все перечисленные отсутствуют** (KeyBindingDef/IdeoDef как типы отсутствуют, pawnSingular есть — pawnsPlural нет); сегодня достигается `--defs-field/--defs-dict/--fuzzy` (`scan.rs:74-76`); расширение — обоснованным списком после релиза |
| R12 | TokcDK | WPF-редактор таблиц, ModsConfig-менеджмент, RMT.DB кеш | NON_GOAL (React-workspace; дублирование) |
| R13 | laskinss27 | CSV→Google Sheets | NON_GOAL (пропускает patches/DefInjected/версии) |
| R14 | etejasdgjjjj532 | JP-XLSX экспорт | NON_GOAL — сломан в конкуренте (w6: to_sheet AttributeError) |
| R15 | kelvinauta, lenhare | DeepL-only скрипты | NON_GOAL; lenhare — анти-паттерн секретов (S9) |
| R16 | JalapenoLabs | LLM-промпт-путевание путей | NON_GOAL (анти-паттерн; у RimLoc структурированный промпт `prompt.rs:16,101,117`) |
| R17 | AutonomoAI | «Полная локализация за копейки» | NON_GOAL как тул (кода нет); нарратив — roadmap 21 |
| R18 | RimLangKit, Grabber, MTT, RWAT-RMK | Morpher/pymorphy3, RMK-корейская ниша, EN→PL ниша, скан всех версий дефолтом | NON_GOAL: у RimLoc morph (pymorphy2/MorpherApi, Case/Plural/Gender — `extras/morph.rs:166,252-266`) и capability-aware WordInfo (`extras/wordinfo.rs:19-35,150`); `--include-all-versions` есть |
| R19 | Remis (Tier B) | Полный stateless read/render GameAdapter-контракт + render round-trip gate + манифесты (R1-R6) | ROADMAP Tier B — нужен при втором игровом адаптере; ядро уже строже (AdapterIdentity/SourceEntryId/провенанс, Tier B §5.1); plugin-api пока scan-only (`rimloc-plugin-api/src/lib.rs:5-19`) |
| R20 | MTT (слоистый глоссарий), Remis | Пост-валидация терминологической консистентности вывода против глоссария | ROADMAP; сегодня глоссарий prompt-enforced (`prompt.rs:16`), валидатор вывода проверяет плейсхолдеры (`validator.rs:13-50`), не термины; CRUD глоссария живой (`session.rs:727-819`) |

## 4. Счёт вердиктов

| Вердикт | Строк |
|---|---|
| SUPERIOR | 9 |
| PARITY | 7 |
| IMPLEMENT_NOW | 1 |
| REJECT_NOT_PRODUCT_RELEVANT | 20 |
| EXTERNAL_BLOCKER | 0 |
| **Всего** | **37** |

**Гейт `parity_superior_parity_only`** (`.rimloc-release-state.json`, сейчас PENDING): по данным этой матрицы — **все материально релевантные capability прямых конкурентов SUPERIOR/PARITY**. Единственный предыдущий блокер (I1 edge-trim) закрыт 9dc9a7c и подтверждён тестами. Оставшийся IMPLEMENT_NOW (I1 chat-batch) — это owner-мандат без конкурентного аналога, конкурентный паритет он не задерживает.

## 5. Честные ограничения этой реконсиляции

1. Конкуренты НЕ перезапускались — все числа конкурентов из отчётов волн; «54/54», «143→736», «1→76» подтверждены волнами/предыдущей реконсиляцией, не этим заходом. **Этим заходом исполнено**: 7 edge_trim-тестов, 3 newest-wins/Gate-H теста, 3 canonical_no_po теста, полный parsers-suite (28+2) — команды в §0.
2. Edge-trim: игровая семантика (хранит ли RimWorld InnerText без trim) прогоном игры не проверялась — evidence: парсер-тесты + 2 лейна + 4 конкурента с сырым хранением.
3. GUI-поверхности (Compare screen, Source Inspector, UI-проход) — чтение кода/коммитов/CHANGELOG, не запуск приложения.
4. Keychain: live за feature `keychain` (`Cargo.toml:27`); CLI-дефолт-сборка без фичи — решение о релизной сборке вне этого лейна.
5. Поведенческого сравнения валидаторов RimLoc vs RWAT/Remis прогоны не делали — P7 паритет по составу чеков, консервативно.
6. Chat-batch: implemented-половина подтверждена тестами, отсутствующая — grep-отсутствием; промежуточные формы (например, частичный batch state в GUI-коde вне grep-шаблонов) не исключены, но не найдены.
