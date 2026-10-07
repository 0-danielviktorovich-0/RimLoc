---
type: reference
status: current
tags:
  - project/rimloc
  - kind/release-gate
last-reviewed: 2026-10-07
related:
  - "[[RELEASE_CONVERGENCE_STATE]]"
  - "[[ACCEPTANCE_MATRIX]]"
  - "[[ADR-MCP]]"
---

# RELEASE GATE — rel22-rc (main @ `af06a4e`)

Дата: 2026-10-07. Кандидат: `rel22-rc` — production-артефакт
`RimLoc-evidence/artifact-rel22-rc/` (binary sha256 `32dcdf8b…`, source
af06a4e = main == origin/main; идентичность: IDENTITY.md рядом с артефактом;
soak-preflight `--gate --repo .` exit 0; release-guard static + runtime PASS).
Публикация — parked по решению владельца (ADDENDUM §27); этот документ —
вердикты по областям, не команда к публикации.

## rel22-rc: дельты против rel21 (R2-волна)

Базовая матрица ниже — вердикты rel21; rel22 изменяет три строки и
подтверждает остальные теми же или новыми доказательствами:

| Область | rel21 | rel22 | Доказательства rel22 |
|---|---|---|---|
| Chat batch (no-API) | OWNER_GATE (каркас) | **RELEASE_READY** | chat_batch.rs (7 Rust-тестов: полный цикл, rejection-матрица, stale+recovery, restart-persistence, BOM, drift-гейт); GUI `#/chatbatch` живой (WDIO route + экран в палитре 14-й командой); ApplyOrigin::Import через канонический apply; 2 кадра в screens/ |
| React UI | 33/33 WDIO, 16 скриншотов | **RELEASE_READY** | WDIO **54 зелёных**: palette 34/34 + LM 15/15 + chatbatch 4/4 (новый полный UI-цикл) + multitarget 1/1 (самодостаточный); спека self-seeded и выровнена по Wave B (состав 14 + экшены, LM-инвариант вместо EN-литерала); 18 скриншотов light/dark |
| Release engineering | OWNER_GATE | **OWNER_GATE** (не изменилось) | rel22: preflight --gate exit 0, release-guard static+runtime PASS, canonical tree hashes, MANIFEST; security: CI full matrix SUCCESS на af06a4e (workflow_dispatch), CodeQL af06a4e → 6 новых High разобраны (§9 CODE_SCANNING_RECONCILIATION.md) → **CS 0 open / DA 0 open**; Dependabot 6 PR post-beta по мандату. Остаток — только publish-апрув + level-7 visual |

Х trailing: **респин 2 артефакта** — E2E chat-batch поймал шов локалей (mapSnapshot 'ru' vs folder 'Russian': применённые переводы не показывались в workspace до переключения цели); фикс f99fc19 (mapSnapshot обе формы + commit() folderForm), прод 2305d564. Интеграционные гейты на af06a4e: cargo test services+cli **385/0**, parsers+services
**311/0**, clippy по канону CI (workspace excl gui + gui all-features) clean,
fmt clean, tsc+vite clean. Артефактные гейты и честные оговорки (DMG на SSD,
self-report `-dirty` = production-ACL) — IDENTITY.md артефакта. Хвост e23f6d0
после af06a4e — только testlab-спеки и security-док, продуктовый код не менял.

---

# Архив: rel21-rc (main @ `2ccdbd16`)

Дата: 2026-10-07. Кандидат: `rel21-rc` — production-артефакт
`RimLoc-evidence/artifact-rel21-rc/` (binary sha256 `1a70c4d5…`), собран с
`main == origin/main @ 2ccdbd16172e44ea87e6141815a9b7d81287b592` (identity:
IDENTITY.md рядом с артефактом; soak-preflight `--gate --repo .` = §89, exit 0;
release-guard static + runtime PASS). Публикация — parked по решению владельца
(ADDENDUM §27); этот документ — вердикты по областям, не команда к публикации.

## Легенда вердиктов

| Вердикт | Смысл |
|---|---|
| **RELEASE_READY** | область готова к релизу; доказательства живые/тестовые в этой сессии или на этой линии |
| **OWNER_GATE** | механика готова, но финальное решение/действие — владельца (визуальная приёмка, публикация, подключение платных API) |
| **EXTERNAL_BLOCKER** | остаток зависит от внешней инфраструктуры (CI-раннеры, чужие платформы), не от кода |
| **REJECTED_NOT_SCOPE** | осознанно НЕ в скоупе этого релиза, решение зафиксировано ADR |

Проверено в этой сессии на кандидате: `cargo test --workspace` — **481 passed /
54 сьюты, exit 0**; `cargo fmt --all --check` (clean), `mkdocs build --strict`
(PASS), прод+automation сборки, живой WDIO (palette **33/33**, LM **13/15**×2 —
оба красных = устаревшие EN-литералы/селектор самой спеки после i18n-волны,
артефакт корректен:
`RimLoc-evidence/artifact-rel21-rc/palette-wdio-results.txt`), Issue #2 end-to-end
на корпусном HugsLib (75/75 → apply → edit → validate 0 errors → build, XML
байт-в-байт; ответ:
https://github.com/0-danielviktorovich-0/RimLoc/issues/2#issuecomment-6024184727),
16 скриншотов light/dark. Clippy/tsc чисты на мерже линии PR #80
(`docs/development/RELEASE_CONVERGENCE_STATE.md`).

## Матрица Area × доказательства → Verdict

Колонки: **Code** (реализация) · **Unit** · **Integration** · **E2E** (нативный
артефакт: AX/WDIO) · **Corpus** (реальные моды) · **In-game** (RimWorld 1.6) ·
**X-plat** · **Docs**.

| Область | Code | Unit | Integration | E2E | Corpus | In-game | X-plat | Docs | Verdict |
|---|---|---|---|---|---|---|---|---|---|
| Source discovery (scan, LoadFolders root Keyed) | ✅ | ✅ | ✅ (export_po_game_version_loadfolders_keeps_root_keyed) | ✅ (визард WDIO, скриншоты) | ✅ HugsLib 75, VE 736 | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Version semantics (пин версии, newest-wins, version diff) | ✅ | ✅ | ✅ | ✅ (compare-экран `cmp.old-dir`, WDIO 33/33) | ✅ (VE 143→736) | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Patches (learn-patches, PatchOperations, IfModActive) | ✅ | ✅ | ✅ | частично (провенанс `patch-applied`) | ⚠️ тест-фикстуры | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Canonical project (store v2, session, revision, ApplyOrigin) | ✅ | ✅ | ✅ | ✅ (весь WDIO живёт через контракт) | ✅ (issue2-проект HugsLib) | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Manual/Existing/Updates (import_existing, lang-update) | ✅ | ✅ | ✅ | ✅ (Existing: классификация 75 → «Применено 75», скриншоты /tmp/rel21-issue2/) | ✅ HugsLib pack | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Build (build-mod, --skip-empty, --from-root, GUI build) | ✅ | ✅ | ✅ | ✅ (Сборка и экспорт — живой экран) | ✅ (issue2 build = ожидаемый XML; L7 мод-пакет) | ✅ мод загружен игрой (level 6.5) | macOS arm64 | ✅ | **RELEASE_READY** |
| Validation (severity, support bundle) | ✅ | ✅ | ✅ | ✅ («Проверки» — живой экран, WDIO) | ✅ (issue2: 0 errors / 14 info) | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Translation Memory (prefill, tm_upsert, live-экран) | ✅ | ✅ | ✅ | ✅ (TM live 6/6 на кадрах) | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Glossary (upsert/merge-keyed, live-экран) | ✅ | ✅ | ✅ | ✅ (WDIO glossary-live) | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| AI (providers, rimloc-llm, translate) | ✅ (честные заглушки, capability-report) | ✅ | ✅ | ✅ (экран AI-провайдеры) | n/a | n/a | macOS arm64 | ✅ (не подключены намеренно) | **OWNER_GATE** — подключение провайдеров и платные вызовы только по явному ок владельца |
| Chat batch (no-API) | ✅ (мандат+каркас) | ⚠️ | — | MOCK | — | — | — | ✅ | **OWNER_GATE** — запуск после решения по провайдерам |
| Multi-target (ru/uk, folderForm, изоляция) | ✅ | ✅ | ✅ | ✅ (WDIO: uk-коммит 16/16 без ru-фазы, изоляция ru↔uk) | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Source Inspector (provenance src/selected_by/locations) | ✅ (данные реальные) | ✅ | ✅ | ⚠️ экран — mock-скоуп W7 (честно помечен) | ✅ (provenance на корпусе) | n/a | macOS arm64 | ✅ | **OWNER_GATE** — live-экран/расширение по решению владельца (данные уже настоящие) |
| CLI | ✅ | ✅ | ✅ (tests/ интеграционные) | ✅ (issue2: scan+build-mod --from-root) | ✅ HugsLib 815 инвентарь / 75 keyed | n/a | macOS arm64 (кросс-платформенный Rust) | ✅ | **RELEASE_READY** |
| Contract (UI_CONTRACT_VERSION, DTO, capability report) | ✅ | ✅ | ✅ | ✅ | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Tauri IPC (permissions, legacy surface, guards) | ✅ | ✅ | ✅ | ✅ (release-guard static+runtime PASS) | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| MCP | — | — | — | — | — | — | — | ✅ ADR | **REJECTED_NOT_SCOPE** — [ADR-MCP](ADR-MCP.md): CLI является агентным API; условия revisit — в ADR |
| Diagnostics (doctor, support bundles, sanitizer) | ✅ | ✅ | ✅ | ✅ (экран Диагностика) | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| Selfloc (каталог-адаптер, contribution build/apply) | ✅ | ✅ (416 тестов волны 8) | ✅ | ✅ (карточка «Translate RimLoc», экран selfloc) | ✅ (собственный каталог ~1.2k) | n/a | macOS arm64 | ✅ | **RELEASE_READY** |
| React UI (13 маршрутов, палитра, compare, LM) | ✅ | ✅ | ✅ | ✅ WDIO palette **33/33**; 16 скриншотов | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** (визуальный level 7 — owner) |
| Accessibility (AX-канал, фокус, aria) | ✅ | ✅ | ✅ | ✅ (ноль краж фокуса; фокус-рестор палитры rel21) | ✅ | n/a | macOS arm64 | ✅ | **RELEASE_READY** (fixme: фокус-бюджет T2a — не блокер) |
| Performance | ✅ | ✅ | ✅ (perf-v2 спеки testlab) | ⚠️ | — | — | — | ✅ ([FRONTEND_PERFORMANCE_EVIDENCE](../design/FRONTEND_PERFORMANCE_EVIDENCE.md)) | **RELEASE_READY** (регрессий на кандидате нет) |
| Security (CodeQL, deps, traversal) | ✅ | ✅ | ✅ (PO-#: traversal fix + 3 теста; lru 0.18.5; dead lock удалён) | ✅ (release-guard ×2) | — | — | — | ✅ | **DOCUMENTED_WITH_EVIDENCE** — CodeQL на candidate SHA `916705f`: 0 open alerts (177 → 0: реальные фиксы + per-group dismiss w/ evidence); Dependabot alerts 0 open (glib tolerable_risk upstream-blocked, 5 npm dev-only tolerable_risk); Secrets 0. Детали: docs/security/RELEASE_SECURITY_GATE.md |
| Docs (EN/RU, MkDocs strict) | ✅ | — | — | — | — | — | — | ✅ (mkdocs --strict PASS в этой сессии) | **RELEASE_READY** |
| Release engineering (identity, gates, артефакт, packet) | ✅ | — | ✅ (§89 preflight --repo, release-guard ×2, canonical tree hashes) | ✅ (WDIO приёмка automation того же коммита) | ✅ | — | ⚠️ DMG-стадия на SSD падает (известное rel19/20); Linux/Win артефакты — CI | ✅ | **OWNER_GATE** — финальный publish-апрув (ADDENDUM §27) + level-7 visual (ACCEPTANCE_CHECKLIST) |

Сводка: **18 RELEASE_READY · 6 OWNER_GATE · 1 REJECTED_NOT_SCOPE · 0 EXTERNAL_BLOCKER · 0 UNKNOWN.** (EXTERNAL_BLOCKER security закрыт: CodeQL на candidate = 0 open, документировано с evidence)

## Owner-гейты кампании (не блокируют сборку, блокируют публикацию)

1. **Level-7 visual confirmation** — мод уже загружается игрой с нулём ошибок
   (level 6.5 proven: `RimLoc-evidence/level7-hugslib/evidence/Player.log.l7`,
   `verify-summary.txt`); финальный визуальный осмотр текста в настройках мода —
   5 минут по `RimLoc-evidence/level7-hugslib/ACCEPTANCE_CHECKLIST.md`.
2. **Publish approval** (ADDENDUM §27) — workflows публикации parked; теги/релизы
   только по явному решению владельца.

## Внешние остатки (не код)

- **CodeQL финальный rescan** на candidate SHA после мержа → единый
  triage/dismiss на финальных номерах (главный поток).
- **Кросс-платформенные артефакты** (Linux/Windows) — CI-раннеры; сам Rust
  платформо-нейтрален, но артефактов этих платформ в этой сессии нет.
- **DMG-дистрибуция** — bundle_dmg падает на внешнем SSD-таргете (воспроизводится
  с rel19); поставка RC — `.app`.

## Evidence-указатель

- Артефакт+identity: `RimLoc-evidence/artifact-rel21-rc/` (IDENTITY.md,
  preflight.json, release-guard-static.txt, release-guard-runtime.txt,
  build-log.txt, sha256.txt, MANIFEST.sha256, screens/ ×16).
- WDIO: `RimLoc-evidence/artifact-rel21-rc/palette-wdio-results.txt`;
  сырые логи `/tmp/rel21-wdio/palette-run1.log`, `lm-run1.log`, `lm-rerun.log`,
  `screens-run.log` (переживут сессию до перезагрузки /tmp).
- Issue #2: комментарий
  <https://github.com/0-danielviktorovich-0/RimLoc/issues/2#issuecomment-6024184727>;
  артефакты прогона `/tmp/rel21-issue2/` (driver, built XML, no-po-out,
  01-existing-classification.png, 02-existing-applied.png).
- In-game: `RimLoc-evidence/level7-hugslib/` (level 6.5, Player.log).
- Линия: `docs/development/RELEASE_CONVERGENCE_STATE.md` +
  `.rimloc-release-state.json` (машинный authority).
- Предыдущая точка: `RimLoc-evidence/artifact-rel20-palette-mustfix/` (rel20,
  358e17a — устарел).

## rel22-сводка

**19 RELEASE_READY · 5 OWNER_GATE · 1 REJECTED_NOT_SCOPE · 0 EXTERNAL_BLOCKER · 0 UNKNOWN**
(chat-batch перешёл в RELEASE_READY против rel21-сводки 18/6; security cycle
закрыт на af06a4e: 6 новых High → 6 evidence-dismiss, zero-open; остача
owner-gates: AI-провайдеры, Source Inspector live, publish-апрув + level-7 visual)
