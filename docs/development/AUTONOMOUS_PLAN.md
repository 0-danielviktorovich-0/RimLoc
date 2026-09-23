# AUTONOMOUS PLAN — RimLoc RC campaign

Start: 2026-09-23 · Approved by Daniel · **Never push/publish — final push only after explicit approval.**

## Goal
Converge RimLoc to the best locally achievable release-candidate: loop
ANALYZE → PLAN → IMPLEMENT → TEST → REVIEW → FIND GAPS → FIX → REGRESSION → REVIEW
until independent passes stop finding meaningful actionable defects.

## Priority order
P0 security/data-loss/broken-core → P1 major correctness/real-mod/severe UX → P2 missing functionality/architecture/competitor gaps → P3 perf/ergonomics → P4 cosmetic. No P3/P4 while P0/P1 actionable.

## Hard constraints
- Original RimWorld install, Workshop mods, saves, production profile: **read-only** (hash-verified in testlab).
- No paid API calls (key presence ≠ permission). Tests use MockProvider/fixtures/local providers only.
- No history rewrite, force-push, branch deletion, stash delete/apply without analysis (stash analyzed 2026-09-23, kept intact).
- Small scoped English Conventional commits with `- ` bullets (commit-msg hook); agent scripts `scripts/agent-*.sh` with session `zcode-rc-campaign`; changelog `Unreleased` for user-facing changes; crates MSRV 1.70; architecture invariants per AGENTS.md (services = orchestration, CLI thin).
- Blocked workstream → record in AUTONOMOUS_STATUS.md, keep moving, revisit.

## Phases (roadmap, not checklist; stop criterion = convergence)
- **F0 baseline** — git inspection, stash analysis, bundle+tag `f0-baseline-20260923`, fetch, rebase 43/2 (done clean), baseline build/test/fmt/clippy, status files.
- **F1 testlab** — `testlab/` (manifests/synthetic/adversarial/expected/minimized-regressions/scripts), real-mod manifests (1814383360 VWE partial human RU, 2927850179 VFE pack, 2126925929 VE Weapons, vanilla tar), hash-verify script, `rimloc compare` (JSON+MD: EN vs human vs RimLoc/LLM; structural ≠ linguistic), CLI dogfood → tickets; every real bug → minimized regression fixture.
- **F2 security** — Tauri trust-boundary audit: CSP (csp:null), innerHTML audit, `open_path` → opener plugin, `save_text_file` → dialog-scoped, plugin loading default-off/allowlist+integrity, zip-slip `enclosed_name()`, negative tests each fix. SECURITY_AUDIT.md. Stash finding: WIP `tauri-plugin-dialog`+`tauri-plugin-shell` — supersede with current versions.
- **F3 deps+CI** — resolve Dependabot via actual dep state (not blind PR merges; don't close human PRs accidentally); CI gates: fmt, clippy -D warnings, cargo test --workspace, cargo-deny, frontend typecheck/tests/build, tauri build; bump zip/image/quick-xml/schemars/reqwest/libloading where benefit>risk; unify dual Cargo.lock handling.
- **F4 architecture+quality** — layering domain→services→CLI/Tauri→GUI; split monoliths (CLI main.rs ~1805 lines, Tauri main.rs ~3928, frontend index.js ~2513); typed errors; remove stubs (scan_keyed_xml) + legacy RimLocError; unwrap/expect reduction in libs; atomic writes (tmp→validate→rename), resumable ops, bounded checkpoints, interruption tests; structured logs + run-reports (no secrets).
- **F5 competitors** — clones in ~/Developing/_competitors-rimloc/ (Text-grabber [unlicensed: behavior-only], RimLangKit Apache-2.0, RimTrans MIT, RimTranslate GPL-3, RimWorldAiTranslator, rimworld-autonomous-translator); COMPETITOR_MATRIX.md (feature/behavior/license/RimLoc-design/status/tests/decision); implement: tag wl/bl+filters, dependency-aware metadata/LoadFolders, filename collisions, WordInfo Case/Plural/Gender, encoding normalization, tag stats, Names transliteration, change reports, Keyed merge, Backstories, mod discovery+About.xml, pending/sourceChanged statuses, glossary built-in+user, obsolete PO, compendium/fuzzy, robust diff/update.
- **F6 LLM subsystem** — rimloc-llm: TranslationEngine/Provider/BatchPlanner/Glossary/Validator/RetryPolicy/CheckpointStore/CostEstimator; providers Anthropic Messages + OpenAI-compatible (presets OpenAI/Z.AI/Ollama) + MockProvider; keychain+env secrets; untrusted mod text (prompt-injection defense); structured IDs; validate outputs; checkpoints/resume; quality layers: deterministic compare → human gold → independent semantic review.
- **F7 GUI workstation** — Svelte 5+TS+Tauri 2; discovery/picker, project overview, EN|translation editor, search/filter/status (untranslated/TM/LLM/human/pending/sourceChanged), inline edit, validation badges, glossary, TM prefill, LLM actions, progress/notes, diff/validate/build/compare, provider config; thin typed IPC, no privileged logic in frontend; GUI E2E tauri-driver/WebDriver + IPC mocks + deterministic screenshots w/ visual inspection; **full GUI dogfood journey without CLI fallbacks** (fresh→discover→project→scan→inspect→filter→TM→LLM→edit→validate→fix→source-change→compare→build→restart→reopen→continue→verify); any CLI-only point = product gap.
- **F8 completeness+perf** — persistence/recent/autosave/undo/bulk/validation-nav/TM review/conflicts/cancel/resume/provider diagnostics (only if workflow-blocking); benchmarks first (startup/discovery/scan/diff/load/search/validate/build/editor render) on small+large real mods, then targeted opts.
- **F9 RimWorld acceptance** — isolated disposable profile (config copy, separate data dir, -logfile) — never touch production profile/saves/Workshop; chain: scan→translate→validate→build→isolated game load→game-log inspection→in-game smoke; real-load-only bugs → regression fixtures; if automation impossible → document + manual smoke steps.
- **F10 convergence** — Pass A engineering (arch/quality/security/tests/CI/deps/stubs/perf) + Pass B hostile product (clean state, GUI journey, real mods, adversarial input, cancel/interrupt/resume, restart, provider failures, large project, real RimWorld); meaningful fix → rerun pass; RC gate: clean passes, no known P0/P1; FINAL_ACCEPTANCE.md; STOP, await push approval.

## И3 PRODUCT & PUBLIC RELEASE (мандат A–AP + дополнения AQ-AU/AV)

Порядок по §AO: GUI RC → Pass A/B → beginner UX acceptance → repo/docs hardening → agent/CLI skill → branch cleanup → release engineering → cross-platform RC → release notes/readiness → STOP (decision packet §AP).

- **§AQ Codecov**: cargo-llvm-cov (Rust) + фронтенд-покрытие через тестовый стек; flags rust/frontend/integration, components core/services/extraction/validation/llm/tauri/frontend; базовый замер информационно, гейт = patch/new-code coverage; статус-чеки на main только после стабилизации; action pinned на immutable SHA.
- **§AR/AS Actions audit**: инвентаризация всех workflows с локального HEAD (цель/триггеры/jobs/permissions/secrets/actions), классификация KEEP/REWRITE/MERGE/DELETE/DEFER; Trusted Publishing/OIDC для crates.io; без `|| true` в гейтах; SHA-pinning + комментарии версий; actionlint + security analyzer; информационные джобы называются информационными.
- **§AT Release consolidation**: ОДИН release-authority; CI → artifacts → smoke → checksums → attestations → RC → явный approve-gate → stable → crates.io; Edge/nightly только если полезен.
- **§AU Positioning**: после acceptance — «RimWorld Localization Workstation» (моды + база + DLC + языковые паки + maintenance/QA/version-aware); обновить description/topics/README/docs синхронно; не рекламировать непротестированное.
- **§AV RimSort interop** (не блокирует RC): RIMSORT_INTEROP.md с классификацией VALUE/COUPLING/MAINTENANCE/RISK → IMPLEMENT NOW/DEFER/REJECT; RimLoc не становится мод-менеджером; чистый контракт «open in RimLoc».

## Parallelism
Subagents on Flash models per Daniel's policy; git worktrees for disjoint tasks; explicit file/crate ownership; one architectural hotspot at a time; mandatory integration review before merge; subagent output not trusted blindly.

## BACKEND STABILIZATION GATE — кумулятивная координация (мандаты 2026-09-23 вечер)

Все мандаты ниже **кумулятивны** к предыдущим. Это НЕ четыре независимых редизайна —
ОДИН связный DAG вокруг канонической архитектуры. Приоритет исполнения:

1. корректность/P1-фиксы независимого ревью (остатки) →
2. TKey round-trip + канонический матчёр/инвентарь →
3. точная effective-семантика контента RimWorld →
4. каноническая типизированная project/source/translation модель →
5. PO в роль адаптера →
6. Eligibility/Knowledge Engine в канонический инвентарь →
7. first-class поддержка существующих переводов (maintenance) →
8. contributor/debugging/observability фундамент →
9. фриз бэкенд-контрактов →
10. глубокая интеграция Svelte GUI →
11. продолжение одобренной GUI/product/pass-A/pass-B/public-release кампании.

Параллелить только неконфликтное: GUI-дизайн/визуал/i18n/onboarding/моки идут
пока бэкенд стабилизируется; НЕ привязывать постоянное GUI-состояние к legacy
PO-центричным структурам до принятия бэкенд-гейта (граница сервис/адаптер или моки).

**Не переделывать сделанное**: популяции 360/358/350/351/343, машинный reconcile,
7 unmatched, TKeyMeta, контекстные стратегии, дубль-контексты Royalty, TKeyRegistry,
exact-first+алиасы+гейт суффикса, coverage/compare матчёр, TKey-фикстура CLI-регрессии —
ЗАКРЫТО с эвиденсом (44fd9ac); не переоткрывать без новых противоречий.

### Нормализованный остаток независимого ревью — по ИМЕНАМ (A–L, авторитетный чек-лист)
Ведётся в AUTONOMOUS_STATUS.md; статусы OPEN/ACTIVE/BLOCKED/DONE с коммитом/эвиденсом.
A. TKey output/round-trip сериализация end-to-end (writer по TKeyMeta, все 4 стратегии;
   одна логическая идентичность → один primary output + алиасы + мульти-контексты)
B. каноническая унификация inventory/сервисов (scan/coverage/compare/validate/word-info/
   translate/GUI/MCP — одно определение «все записи»; машиночитаемая таблица consumer ×
   {entrypoint, effective-mod, version, TKey, eligibility, language, exceptions})
C. TKey-aware кросс-язычные валидаторы (placeholders/lists/orphans/sourceChanged —
   канонический матчёр; порядок exact → typed alias → allowed fallback → ambiguity/
   no-match; alias≠orphan; настоящий .slateRef не алиасится; ambiguity = диагностика)
D. полнота прямых coverage/consumer-регрессий (кейсы A .slateRef/B .value.slateRef —
   есть; C TODO→missing, D absent→missing — добавить)
E. внешний `--defs-dir` в scan_defs_tkey (ходит по root, фильтр starts_with) + регрессия
F. hardening аудит-скриптов (временные каталоги, без stale /tmp, полные нормализованные
   сравнения, воспроизводимость)
G. устаревшие доки/комментарии: TKey существует с 1.1 (май 2020), 1.6 = primary tested
H. effective RimWorld load precedence: Common/версии/LoadFolders/Def first-wins/
   Keyed last-wins/дубли полей/зависимости-DLC/path-case; authoritative источник = тот,
   что реально берёт игра; перекрытые контексты = диагностика (не «union как source»)
I. каноническая project model / persistence / адаптеры (см. Mandate 2)
J. eligibility/knowledge архитектура (см. Mandate 3)
K. existing-translation maintenance/import (см. Mandate 4 §1–7)
L. contributor/debugging/observability инфраструктура (см. Mandate 4 §8–39)

### MANDATE 2 — каноническая проектная модель / формат-адаптеры (кратко)
Цель: типизированная каноническая внутренняя модель, независимая от PO/JSON/XLIFF/GUI/CLI.
PO = interchange-адаптер, НЕ внутреннее состояние. Слои: RimWorld-адаптеры → effective
content resolver → eligibility/schema engine → канонический SourceEntry-инвентарь →
Project/Translation state → TM/глоссарий/LLM/валидация/diff/wordinfo → сервис-слой →
GUI/CLI/MCP → выходные адаптеры (RimWorld/PO/JSON/XLIFF). ОДНА доменная модель.
- Сначала АУДИТ реального дата-флоу из кода (не по памяти) → CANONICAL_PROJECT_MODEL.md;
  классификация зависимостей DOMAIN/ADAPTER/TRANSPORT/LEGACY × KEEP/MIGRATE/REMOVE.
- SourceEntry: стабильная identity (target-независимая!), EntryKind семейства Keyed/
  DefInjected/TKey/Strings-RulePack/Backstories/Patch-derived (структурная типизация по
  поведению), source locale, мульти-контексты, location/version/provenance/hash,
  serialization metadata (TKey), валидационные поля.
- Translation: source_entry_id + target locale + text + status + provenance + notes +
  validation + sourceChanged + origin (human/TM/LLM/imported). Статусы: UNTRANSLATED/
  TRANSLATED/TODO/SOURCE_CHANGED/PENDING_REVIEW/INVALID-ORPHAN/OBSOLETE/AMBIGUOUS —
  размерности (completeness/review/validation/lifecycle) вместо одного гигантского enum,
  если так чище по реальным workflow.
- TKey round-trip = обязательный модельный тест (весь цикл). Effective content = часть
  source provenance. Source-адаптеры (Mod/Core/DLC/LanguagePack) кормят одну модель.
- JSON machine format: явная схема+версия (не случайный serde-вывод). XLIFF: оценить,
  если bounded — сделать, иначе DEFERRED с границей адаптера. CSV — только под реальный
  workflow с честной матрицей потерь.
- Persistence: по evidence (SQLite/файлы/manifest+db), портируемость/экспорт обязателен.
  TM/глоссарий/LLM/валидация — на канонических юнитах, pair-scoped, без обязательного PO.
  GUI: update_translation(entry_id, locale, text). CLI: те же сервисы. MCP: напрямую
  сервисы. Fidelity-матрица FULL/PARTIAL/LOST/N.A. — фактическая.
- Миграция по шагам без big-bang, обратная совместимость версионируется; acceptance:
  no-PO workflow (scan→project→MockProvider→validate→build) + PO-interop workflow +
  reopen/recovery; после гейта — фриз контрактов.

### MANDATE 3 — adaptive eligibility/knowledge (кратко)
Цель: максимум детерминированной корректности, минимум AI. ОДИН eligibility-компонент
в каноническом инвентаре (никаких отдельных «AI-сканеров»). Возврат TRANSLATABLE/
NON_TRANSLATABLE/REVIEW/UNKNOWN + структурированный evidence; explainability (decision/
authority/evidence/rule); классы уверенности DETERMINISTIC/VERIFIED/STRONG_INFERENCE/
HEURISTIC/UNKNOWN (без фейковых процентов).
- Версионная translation schema из first-party семантики (MustTranslate/NoTranslate/
  TranslationHandle) для 1.6; кастомные моды: metadata-only статический инспекшн
  сборок (БЕЗ исполнения кода; untrusted input; негативные тесты).
- Прецедент знаний: first-party → assembly metadata → verified schema → built-in →
  verified community → user → project → reference evidence → эвристики → AI-пропозалы;
  явный NoTranslate не перекрывается эвристикой/AI молча.
- Словари/allowlist = кэш/совместимость, не единственный источник правды. Unknown
  строковые поля НЕ теряются молча: структурные fingerprint'ы (packageId+DefType+path+…
  → одна задача, не 500 AI-вызовов), POTENTIALLY TRANSLATABLE/TECHNICAL/UNKNOWN на
  review; adversarial-тесты на технические строки (defName/packageId/пути/классы/URL).
- Reference mining: blind inventory → reveal reference → unexplained target →
  классификация REAL MISS/OBSOLETE/GENERATED/ALIAS/INVALID/VERSION SKEW; reference
  НЕ утекает в слепые замеры. AI adjudication: маленький evidence-бандл, structured
  output (TRANSLATE/DO_NOT_TRANSLATE/UNCERTAIN + reason), кэш по fingerprint+версиям,
  платные API только с разрешения; AI = proposal, промоушен-лестница project→user→
  community candidate→verified; декларативные rule-паки (селекторы/решения/provenance,
  БЕЗ исполнения кода), конфликты диагностируемы. «Teach RimLoc» GUI-концепт;
  explain API; agent evidence API; `localization-audit` для авторов модов (DLL-строки —
  диагностика, defer если шумно). Фикстуры unknown-модов A–I; ре-модный бенчмарк после
  изменений (misses/false positives/unknown/resolution split). Доки: TRANSLATION_ELIGIBILITY.md.

### MANDATE 4 — existing-translation maintenance / contributor / observability (кратко)
- Issue #2 = обычный пользовательский workflow «продолжить существующий перевод»:
  source + существующий пак → import → канонический матчинг → сохранить валидную работу
  → классификация REUSABLE/SOURCE_CHANGED/NEW/OBSOLETE/ORPHAN/AMBIGUOUS/UNKNOWN/
  GENERATED → редактирование → validate → build. GUI: «Translate a new mod» vs
  «Open existing translation». Provenance импорта (human/tool/AI/unknown). Dry-run
  для деструктивных операций. Интеграция с version-diff/sourceChanged, без второго
  движка обновлений. Референс-майнинг = продвинутый побочный бонус. Реальные кейсы
  A–F (полный/частичный/старый/obsolete/алиасы-TKey/чужой инструмент).
- Contributor quality: лестница TIER 0–5 (compile → unit → integration/fixtures →
  реальные моды → GUI E2E → изолированный RimWorld) с матрицей «изменение → минимальный
  tier»; real bug → minimized fixture → regression; security-матрица негативных кейсов
  (path traversal/malformed/oversized/unsafe URL/секреты/arbitrary rule); coverage =
  поддержка, не вендетта; `cargo xtask verify` (--changed/--full/--real-mods/--security/
  --release), кроссплатформенно; Contributor skill для vibe-coder'а (issue→evidence PR),
  инспекция существующих AI-OS скиллов перед созданием (не дублировать), lazy discovery;
  evidence contract (7 вопросов). Non-programmer E2E-упражнение.
- Observability: structured logs (operation id/stage/counters/timing, без секретов),
  correlation IDs, user-facing ошибки (что случилось/что изменилось/что дальше),
  `rimloc doctor` + GUI «Diagnose», sanitized support bundle (report/diagnostics/
  environment/reproduction/логи/фикстура, manifest; redaction ключей/путей/авторских
  модов), «Copy for AI» (reproduce-first промпт), GitHub-issue-friendly значения,
  GUI semantic automation (roles/names/WebDriver/фикстуры/MockProvider), safe test mode
  (без privileged backdoor), failure artifacts, root-cause дисциплина, perf по замерам.
  Доки: CONTRIBUTOR_TEST_STRATEGY.md, DEBUGGING_OBSERVABILITY.md.

### BACKEND FREEZE ACCEPTANCE (до глубокой привязки GUI)
Один канонический инвентарь · корректный effective source view · типизированный TKey
round-trip · все кросс-язычные потребители на каноническом матчёре · импорт существующих
переводов сохраняет работу · PO не обязателен внутри · канонический project state ·
identity target-независима · мульти-контексты · eligibility объясним · unknown
представим · расширения безопасны · регрессии зелёные · ре-модный корпус без регрессий.
После гейта — компактный DELTA-бандл для финального ChatGPT-ревью (только дифф от
прошлого чекпоинта + перечисленные артефакты), GUI-работа продолжается параллельно.
Никогда: push/release/stash-delete/branch-delete.
