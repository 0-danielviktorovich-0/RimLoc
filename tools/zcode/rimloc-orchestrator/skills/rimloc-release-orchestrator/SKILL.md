---
name: rimloc-release-orchestrator
description: Методология multi-workflow release convergence для RimLoc и подобных multi-subsystem кампаний. Триггеры: release convergence, кампания с ≥2 подсистемами и ≥3 независимыми deliverable, security/release/architecture работа, большая параллелизуемая задача. НЕ для: одиночного фикса, одной функции, тривиальных правок.
license: GPL-3.0
metadata:
  type: methodology
  version: "1.0.0"
  source: RimLoc release campaign 2026-10 (wave 1-5 проверено)
---

# Release Orchestrator — методология

## Ядро
1. **Reality audit** до всего: fetch, origin/main, clean tree, AGENTS.md, живые счётчики (не доверять старым SHA/документам).
2. **Plan → DAG** → lanes → hotspots → merge order → acceptance criteria. Plan ограничен по времени, заканчивается NEXT EXECUTABLE ACTION.
3. **Parallel lanes**: один Workflow (или lane) = одна bounded проверяемая цель, свой worktree/ветка, свои сабагенты/скрипты/инструменты. Никаких двух writers в один hotspot.
4. **Deterministic first**: инвентари и проверки — скрипты/API, не агенты; агенты анализируют группы; скрипт проверяет финал.
5. **Structured delivery**: LANE/BASE_SHA/HEAD_SHA/BRANCH/FILES/TESTS/EVIDENCE/RISKS/NEXT — не prose-дневник.
6. **Independent verification**: ревьюер ≠ имплементер; каждый значимый PR — чужая проверка.
7. **Durable state**: machine-readable state-файл + human-резюме; resume = state+git+GitHub reconcile, не рестарт.
8. **Zero-unknown finish**: ни одного UNKNOWN security alert / open PR / dead route / mock в проде / неисследованной ветки.
9. **No auto-publish**: публикации — только явный owner gate.
10. **Не over-orchestrate тривиальное**: один фикс — один коммит, без церемоний.

## Completion & reviewer discipline (mandatory)

- Multi-system work **cannot** close with only a short transcript or one branch.
  Capture the Workflow run id or tool-unavailability diagnosis, one bounded
  owner/worktree per lane, `LaneResult` receipts and independent verifier.
- Inspect current (not historical) CI runs and every CodeQL/Codecov bot comment
  at each PR's exact HEAD. Red check or unresolved security report = BLOCKED,
  not "all gates green". Stop only at a genuine external/owner gate.
- The owner approved direct browser-mediated communication with ChatGPT.
  Prefer native `local.handoff(reason,prompt)` if available; otherwise use
  the authorized ZCode/IAB **WF-REVIEWER-BRIDGE** lane to send AND receive
  real messages in the existing RimLoc ChatGPT Project. Capture the exact
  chat URL privately. If context limit is reached, create a new chat inside
  that SAME Project, transfer a verified state pack and keep URL lineage.
  Read `docs/development/CHATGPT_BROWSER_REVIEW_BRIDGE.md` before each
  reviewer handoff. Never claim that a GitHub comment notified ChatGPT;
  private `RimLoc-review-handoff` is only fallback evidence mailbox.
  Owner manual forwarding is NOT the default workflow.
- Treat documents such as `.rimloc-release-state.json` as **potentially stale
  snapshots**: reconcile them against live PR identities before updating.
  Keep owner UAT, RimWorld playtest, credentials and publication as owner gates.
- See `docs/development/PRODUCT_CLOSURE_PREFLIGHT.md` for objective-by-objective
  acceptance and truthful PARTIAL/BLOCKED status rules.

## Практические уроки (RimLoc 2026-10)
- Длинные лейны — только через фоновый Workflow-инструмент (переживает прерывания); обычные субагенты гаснут при прерывании хода.
- Таймбокс в брифе каждого сборочного/WDIO лейна (иначе зависание).
- Бисект — только со свежим таргетом на коммит (stale-fingerprint даёт ложного виновника).
- cargo-сборки параллельных лейнов — отдельные CARGO_TARGET_DIR; mkdir каталога до cargo при storage-guard.
- Финальные вердикты MUST_FIX обновляй в матрице сразу после фикса (не копи).

Подробный runtime-контракт инструмента: docs/development/ZCODE_WORKFLOW_RUNTIME.md (в репо продукта).
