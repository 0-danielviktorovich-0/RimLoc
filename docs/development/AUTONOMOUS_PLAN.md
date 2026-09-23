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
