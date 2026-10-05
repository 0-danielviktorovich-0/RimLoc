# STATE CORRECTION — 2026-10-05 · продолжение той же сессии

Прежние «5 владельческих решений» — СТАЛЫЕ. Владелец выдал решения; этот документ —
канон текущего состояния для любой свежей сессии/компакции.

## Исправленные решения (больше НЕ спрашивать)

| Тема | Решение |
|---|---|
| **TM** | **A + B + C**, уже решено. A = авто-накопление, B = импорт, C = ручной CRUD. Статусы качества минимум DRAFT/ACCEPTED/REVIEWED, provenance обязателен. Слепой промоушен каждого Save в доверенный TM запрещён. Реализовать + практически протестировать. |
| **DMG** | DEFERRED. Не спрашивать. Сейчас владельцу нужен напрямую запускаемый `.app` (production, React, automation=false). |
| **Git** | Защита ветки остаётся как есть. Авторизованный флоу: feature-ветка → push → Draft PR → checks → merge по правилам репо. Обычный недеструктивный пуш веток разрешён. Запрещено: force-push, history rewrite, release tag, GitHub Release, signing, notarization. |
| **Svelte** | FROZEN LEGACY / FALLBACK. Не удалять. Разрешено собирать/запускать автоматически для benchmark-сравнения. |
| **React** | Прод-UI. Lovable R1 — канонический визуальный/интеракционный baseline. |

## Git-факт (2026-10-05)

- `main` (локальный) = `feature/ui-r1-convergence` = origin — **Draft PR #60**
  (<https://github.com/0-danielviktorovich-0/RimLoc/pull/60>), base `origin/main` (278+ коммитов).
- Push-защита GitHub: в истории синтетический Slack-токен в
  `frontend-v2/tests/contribution-bundle.test.ts` (тест-фикстура, не секрет) —
  allowlisted через `secret-scanning/push-protection-bypasses` API, reason `used_in_tests`.
- `codex/pass-b-tails` влит: забран `docs/development/COMMUNITY_TESTING_HANDOFF.md`;
  `perf_bench.rs` осталась версия main (новее: fmt/clippy-чистка 0d7adfe).

## Ограничение среды

- **Диск: ~13 GiB свободно (97%)** — полные cargo-сборки в свежих worktrees запрещены.
  Cargo-прогоны лейнов: инкрементально, на тёплом `ba-main/target` (4.9G) через
  `CARGO_TARGET_DIR`. Полную сборку Svelte-варианта приложения не делать без preflight.

## Лейны (параллельно, субагенты)

- **A** конкуренты 3–17: source-инспекция, evidence levels → `docs/competitive/`
- **B1** Text Grabber (kamikadza13) — clone/freeze/run на корпусе → `docs/competitive/TEXT_GRABBER_VS_RIMLOC.md`
- **B2** RimLangKit (OneCodeUnit) — clone/freeze/dotnet-run → `docs/competitive/RIMLANGKIT_VS_RIMLOC.md`
- **C** TM A+B+C: домен+контракт+session по паттерну глоссария + React `Tm.tsx`, тесты → worktree `wt-tm-live`
- **D** React vs Svelte perf: один и тот же оп на обоих фронтах, APP_INTERNAL ≠ WDIO roundtrip → `docs/design/FRONTEND_PERFORMANCE_EVIDENCE.md`
- **E** visual parity vs Lovable R1 + identity owner-артефакта (после мержа TM — артефакт пересобрать)
- **F** providers/keychain proof (keyring feature в rimloc-llm; БЕЗ платных вызовов)
- **G** палитра полная приёмка + Language Manager автопринятие (WDIO)
- **H** security: cargo deny, npm audit, secret scan, Tauri caps/CSP/IPC → `docs/security/PRE_BETA_SECURITY_AUDIT.md` + `DEPENDABOT_RECONCILIATION.md` (15 открытых PR)
- **I** скилл `product-ui-design` (AI-OS): патч CANONICAL REFERENCE MODE локально

## Корпус для диффа

`/Users/danielviktorovich/Developing/rimloc-test-corpus/` — 4 мода Workshop
(см. `CORPUS_METADATA.md`): PatchOperations+LoadFolders (3170653412), dll-библиотека
HugsLib (818773962), нативный multi-version VE Framework (2023507013),
DLC-зависимый Anomaly Patch (3242000764). Прод-Workshop не трогать.

## Evidence levels (без изменений)

0 NOT_IMPLEMENTED · 1 DOC_ONLY · 2 SOURCE_CONFIRMED · 3 UNIT_TESTED ·
4 INTEGRATION_TESTED · 5 BUILT_APP_E2E · 6 SAME_CORPUS_DIFFERENTIAL · 7 IN_GAME_PROVEN.
Tier A parity/superset не заявляется от evidence 1–2.
