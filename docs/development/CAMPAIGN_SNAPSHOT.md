# CAMPAIGN SNAPSHOT — авторитетный слепок для следующего контекста (25.09)

## HEAD и состояние
HEAD 3e2b208 (main, RimLoc). Rust: 165/0 тестов, clippy 0. GUI: build + svelte-check 0,
vitest 20/20, w2-regressions 51/51. **НЕ пушено.** Диск: ~48 GB свободно (владелец почистил;
проверять df перед любыми утверждениями о диске). Style Lab dev-сервер: nohup на :5199
(переживает сессии; лог /tmp/rimloc-stylelab.log; kill: lsof -ti :5199 | xargs kill).

## Safety invariants (никогда)
No push / no release / no force-push / no stash или branch deletion. RimWorld-установка,
Workshop-моды, сейвы, прод-профиль — READ-ONLY (hash-гвард). Локальная LLM запрещена
(ресурсы), платные API — только явное «ок». Cargo target/ — регенерируемый кэш (23 GB,
владелец в курсе). Коммиты — явный pathspec, малые, Conventional Commits (типы: feat/fix/
docs/chore/test/refactor/ci/build/perf/revert; body с «- » буллетами; subject ≤72).

## Закрытые гейты (не переоткрывать без новых доказательств)
A TKey round-trip (8595129) · B канон-инвентарь (98db4b2, VWE 38→45%) · C typed Resolution
валидаторов (c634ee0) · D регрессии coverage · E внешний defs-dir · F hardening аудита
(5df8bd6) · G TKey-доки «с 1.1» · H effective precedence + LoadFolders (8c49e83: Keyed
last-file/in-file first, Defs first-file, SetOrAdd; VWE 195@1.5/247@1.6) · I1-I4 (аудит
63309a7, модель+bridge 05432de, persistence bdfb3a4, acceptance 4a7f5d2: A no-PO / B PO-interop /
C existing — на одной модели) · J контракт+движок (69e9163, 042e0b5: 156 seed-правил,
NoTranslate-финальность) · K-core detect_source_changes (34dc83d) · патч-этап (c4384b2,
916202b: li-Class/FindMod/or-предикаты, ~50% реального покрытия).

## BACKEND LANE (параллельная, сейчас)
1. **J-проводка**: scan_canonical/GUI вызывают evaluate() вместо собственных решений;
2. **L observability**: doctor, support bundle + redaction preview, Copy-for-AI, structured
   logs, operation IDs;
3. **PRE-FREEZE contract check (Source Inspector)**: canonical SourceEntry/SourceContext уже
   несёт file/line/def_type/role(Effective|Overridden)/provenance(version_selected/
   conditional/patch_stage) — ДО фриза добавить winner-reason (почему этот кандидат выиграл:
   версия/LoadFolders/precedence-правило) в provenance, чтобы после фриза UI мог ответить
   «what file / where / which won / why»;
4. **BACKEND FREEZE** → delta-бандл для ChatGPT (компактный: дифф от 50f9483) → real UI
   binding → LIVE ACCEPTANCE GATE (мандаты W6 §3, W7 §26).

## GUI LANE (параллельная, моки/адаптеры до фриза)
Сделано: фаза 1 (f1fc933: роутер/rich-mock 64 записи/Home/Wizard/Workspace UX),
GUI-A (c537ad1: Review/Build/Existing), GUI-B (44199ac: Settings/Providers/Help/Cmd+K),
W1 (4e41c85: wizard-автомат/replay/naming + 20 vitest), W2 (29064f2: language registry/
multi-target/stale-AI per-target + 51 регрессия), Style Lab + D1/D3 доки.
- **W3 (СЛЕДУЮЩАЯ ЗАДАЧА)**: гибридная стратегия (existing/TM/glossary чекбоксами +
  «перевести оставшееся» радио; Manual всегда; стратегия меняется после создания) +
  chat-batch manager (стабильные batch ID, revision/source-change защита, preview/apply,
  retry-split, чат-профили, sizing Auto/пресеты/advanced; мандат CHAT_BATCH_MANDATE.md).
- W4: provider templates+instances, glossary/TM редакторы, rules inspector, shortcuts
  editor, panel collapse, About (мандат GUI_QA_MANDATE.md §6-15/24).
- W5: diagnostics root-cause + redaction preview, lifecycle-терминология, Project screen,
  capability parity audit, fresh-user QA 6 ролей (GUI_QA_MANDATE §16-21/28-29).
- **W6 PRE-FREEZE**: mock-бейдж «Demo data», scenario browser, anchored tour, demo project,
  REVIEW SCREEN MAP в отчётах (MOCK_LIVE_ONBOARDING_MANDATE.md).
- **W7 PRE-FREEZE**: source inspector mock (SOURCE tab, one-click actions — моки; контракт
  уже покрыт пунктом 3 backend lane) (SOURCE_INSPECTOR_MANDATE.md).
- **W6/W7 POST-FREEZE**: live binding + live acceptance на реальных данных.

## Mock/live boundary
Моки/адаптеры до фриза; dev-бейдж «Demo data»; прод не шипит MockTransport (guard в
xtask/CI после фриза); Style Lab = dev-only (?stylelab=1); бизнес-логика вне фронтенда,
один транспорт (UI → RimLocClient → TauriTransport), никаких invoke() в компонентах.

## ⏸ Pending owner decisions
1. Визуальное направление (Precision/Aurora/Workshop/Editorial + палитра + гибрид) —
   пакет будет ПОСЛЕ W3-W5 на представительных экранах; скриншоты уже в
   testlab/artifacts/gui-style-lab/, лаборатория :5199/?stylelab=1.
2. (мелкое) NoTranslate-финальность поднять из engine-guard в domain::resolve().

## EXACT NEXT TASKS (параллельно, новые контексты)
- **Frontend: W3** — гибридная стратегия + chat-batch manager мок (CHAT_BATCH_MANDATE.md),
  владение: screens/Wizard + новые chat-компоненты.
- **Backend: J-проводка** — scan_canonical/validate вызывают eligibility evaluate,
  TRANSLATION_ELIGIBILITY.md; затем L.
Диск/факты проверять в момент отчёта (урок verify-facts-before-reporting).
