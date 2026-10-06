# RELEASE CONVERGENCE STATE — human summary

Машинный authority: `.rimloc-release-state.json` (корень репо, не публикуется).
При resume: state + git + GitHub + артефакты → reconcile. Не доверять stale SHA.

## 2026-10-07 — старт FULL RELEASE CONVERGENCE (мастер-мандат + addendum + workflow-native)
- origin/main = 719d09d (Codecov lock; вся линия wave 1-5 влита и в main).
- Live security inventory (старт): Code Scanning ~92 open, Dependabot alerts ~72, открытых PR 13. Точный разбор — WF-SEC.
- Существует и валиден: rel20 (palette-mustfix, 358e17a) + level-7 kit (level 6.5 proven).
- Lanes запущены (Wave A, аудиты): WF-ARCH, WF-SEC, WF-UI, WF-COMP, WF-REL.
- Мастер-мандат supersede'ит BACKLOG_PO_OPTIONAL_ARCHITECTURE «bounded»-ограничение: PO-decoupling теперь P0.

## 2026-10-07 (2) — Wave A+B интегрированы
- PR #80 (release/wave-integration): security sweep + canonical no-PO translate + edge-trim + ui-gaps + Compare screen — merged, 459/0 + clippy/fmt/tsc/build чисты.
- CodeQL triage: 177→9 (9 auto-fixed по удалению 0.55.1, 69 FP/18 групп, 90 used_in_test, 9 vendor won't-fix) — финальный dismiss на candidate SHA.
- Dependabot: #65/#74 влиты; major (#51/#53/#54/#64) — post-beta с причинами; grouped #73/#75 — GitHub считает.
- Найден и исправлен PO-#:-traversal (write-guard + 3 теста); lru 0.18.5; мёртвый lock удалён.
- MCP: ADR вариант B (CLI = agent-API, revisit-условия в ADR-MCP.md).

## 2026-10-07 (3) — rel21-rc: ФИНАЛЬНЫЙ ЦИКЛ
- **rel21-rc**: artifact-rel21-rc/ (main@2ccdbd1, бинарь 1a70c4d5…, identity fail-closed PASS с --repo, WDIO palette 33/33, LM 13/15 ×2 — 2 красных = спек-дефекты i18n-волны, не артефакт), OWNER_TEST_PACKET 9 шагов.
- **Issue #2**: практический цикл (75 reusable → apply → edit → validate → build, 0 расхождений), ответ опубликован (issuecomment-6024184727), issue открыт — решение за owner.
- **RELEASE_GATE.md**: 18 RELEASE_READY / 5 OWNER_GATE / 1 EXTERNAL_BLOCKER (CodeQL rescan на candidate) / 1 REJECTED_NOT_SCOPE (MCP ADR).
- CodeQL: candidate SHA 2ccdbd1, последний анализ 2083253 (предок) — финальный triage+dismiss на свежем анализе candidate = единственный оставшийся engineering-гейт.

### rel21-rc детали (LEVEL7_REPORT, WDIO-логи, ISSUE2)
- Level-7 visual: ожидает владельца по ACCEPTANCE_CHECKLIST.md (уровень 6.5 proven).
- LM-spec ревизия: 2 красных = EN-литералы/селектор .form-error в спеке vs локализованные коды в rel21 — спек-дефект, не артефакт.
- Pipeline деталь: build-mod --po формально обязателен даже при --from-root — шероховатость CLI, отмечена в Issue#2 ответе.
