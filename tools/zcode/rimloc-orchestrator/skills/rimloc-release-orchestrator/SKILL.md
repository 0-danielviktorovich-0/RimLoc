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

## Практические уроки (RimLoc 2026-10)
- Длинные лейны — только через фоновый Workflow-инструмент (переживает прерывания); обычные субагенты гаснут при прерывании хода.
- Таймбокс в брифе каждого сборочного/WDIO лейна (иначе зависание).
- Бисект — только со свежим таргетом на коммит (stale-fingerprint даёт ложного виновника).
- cargo-сборки параллельных лейнов — отдельные CARGO_TARGET_DIR; mkdir каталога до cargo при storage-guard.
- Финальные вердикты MUST_FIX обновляй в матрице сразу после фикса (не копи).

Подробный runtime-контракт инструмента: docs/development/ZCODE_WORKFLOW_RUNTIME.md (в репо продукта).
