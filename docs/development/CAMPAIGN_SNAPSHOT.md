# CAMPAIGN SNAPSHOT — RimLoc (авторитетный слепок намерения/состояния)

## Правила этого файла
1. **Git — авторитет живых фактов репозитория**: в начале КАЖДОГО нового контекста
   проверить `git rev-parse HEAD`, `git status --short`, `git worktree list`, и df при
   дисковых утверждениях.
2. Этот файл — авторитет **намерения и состояния кампании** (гейты, полосы, инварианты,
   решения), НЕ изменчивых машинных фактов. Изменчивое помечено «verify at session start».
3. После каждого гейта/волны обновлять ТОЛЬКО: закрытые гейты, текущие полосы, следующие
   задачи, новые инварианты/дыры. Хронологический лог не вести — он живёт в git log.

## Verify at session start (изменчивое)
- Проверенный lead baseline: `0e37b0c` → **проверь `git rev-parse HEAD`**.
- Рабочее дерево: expect clean; чужие незакоммиченные хвосты параллельных агентов —
  не трогать мимоходом.
- Диск: на проверке 25.09 было ~17 GiB свободно; **проверяй df перед
  утверждениями**. Не размножать cargo-кэши в worktrees.
- Style Lab dev-сервер: nohup на :5199, лог /tmp/rimloc-stylelab.log; может быть мёртв —
  рестарт `npm run dev -- --port 5199 --strictPort` из frontend-v2.

## Safety invariants (абсолютные)
No push / no release / no force-push / no stash·branch deletion. RimWorld-установка,
Workshop-моды, сейвы, прод-профиль — READ-ONLY (hash-гвард тестлаба). Локальная LLM
запрещена (ресурсы); платные API — только явное «ок». Коммиты: явный pathspec, Conventional
Commits (feat|fix|docs|chore|refactor|test|ci|build|perf|revert), body с «- » буллетами,
subject ≤72 символов. Факты (диск/порты/процессы) проверять в момент отчёта — урок
verify-facts-before-reporting.

## Закрытые гейты (не переоткрывать без новых доказательств)
A TKey round-trip · B канон-инвентарь (один пайплайн) · C typed Resolution валидаторов ·
D coverage-регрессии (TODO=missing, absent=missing) · E внешний defs-dir · F hardening
аудита · G TKey-доки «система с 1.1» · H effective precedence (Keyed last-file/in-file
first, Defs first-file, SetOrAdd, LoadFolders-effective scan) · I1-I4 canonical model
(модель+bridge+persistence+acceptance A/B/C) · J контракт+движок (156 seed-правил,
NoTranslate-финальность) · K-core detect_source_changes · патч-этап (li-Class/
FindMod/or-предикаты, ~50% реального покрытия). Ключевые коммиты: 8595129, 98db4b2,
c634ee0, 5df8bd6, 8c49e83, 042e0b5, 34dc83d, 916202b, b29ef68, 0c882d2.

## BACKEND LANE (сейчас)
1. **J-проводка лендена** (`303d726`): eligibility diagnostics + optional
   `SourceProvenance.selected_by`; см. TRANSLATION_ELIGIBILITY.md. Это не доказательство
   завершённого live GUI binding.
2. **PRE-FREEZE contract check — OPEN по новому evidence**: в `build_project()` flat
   scan оставляет `selected_by=None`, LoadFolders получает общую batch-метку;
   `canonical_bridge` берёт source location из `TransUnit.path`, хотя Defs merge
   переписывает этот путь в выходной DefInjected. Требуются реальные исходные локации,
   per-entry winner-reason и регрессии (Defs/Keyed/LoadFolders/TKey multi-context).
   Само наличие сериализуемого поля не закрывает Source Inspector gate.
3. **L observability — ACTIVE, не принят**: doctor, support bundle + redaction preview,
   Copy-for-AI, structured logs, operation IDs. Acceptance: известный сбой → causal
   context → очищенный bundle → независимое воспроизведение/диагноз.
4. **BACKEND FREEZE** → компактный delta-бандл (дифф от 50f9483) для ChatGPT.
5. POST-FREEZE: real UI binding → LIVE ACCEPTANCE GATE (W6 §3 / W7 §26 мандатов).

## GUI LANE (сейчас)
Лендено: фаза 1 (роутер/rich-mock/Home/Wizard/Workspace UX), GUI-A (Review/Build/
Existing), GUI-B (Settings/Providers/Help/Cmd+K), W1 (wizard-автомат/replay/naming),
W2 (language registry/multi-target/stale-AI per-target), Style Lab, D1/D3 доки.
- **W3 лендена** (`935f70c`, acceptance `0f91203`): гибридная стратегия, chat-batch
  manager, stale-import/suggestion защита. Mandate identity guard: `91ed049`.
- **W4 лендена** (`3fa2039`): provider instances, glossary/TM, shortcuts, About.
  **W4.5 остаётся**: явные credential references при duplicate, безопасный export,
  mutability/provenance reference-корпусов, shortcut safety, version из build metadata.
- **W5 ACTIVE, не принята**: diagnostics root-cause + redaction preview, Project screen,
  capability parity и QA по **8 персонам** (базовые 6 + base-game/DLC + keyboard/a11y).
  Уточнения acceptance: AUTONOMOUS_STATUS.md, секция W4.5/W5.
- **W6 PRE-FREEZE**: mock-бейдж «Demo data», scenario browser, anchored tour, demo
  project, REVIEW SCREEN MAP (MOCK_LIVE_ONBOARDING_MANDATE.md).
- **W7 PRE-FREEZE**: source inspector mock — SOURCE tab, one-click actions (контракт уже
  покрыт backend пунктом 2) (SOURCE_INSPECTOR_MANDATE.md).
- **W6/W7 POST-FREEZE**: live binding + live acceptance на реальных данных (машина
  владельца: RimWorld 1.6 + Odyssey).

## Исполнение и интеграционный review
- Lead: текущая Codex-задача; архитектура, snapshot, независимые проверки и интеграция.
  Основная реализация — существующие GLM-сессии ZCode; сначала reuse, не дублирование.
- ZCode coordinator: `sess_be9620ba-0118-4457-8ee4-354d3e97c327` (RimLoc).
  L: `agent_2ba7172f-de60-4338-b7d9-8269ea223ee1`; W5:
  `agent_d2a2be70-b36b-4142-ad1b-f4480fbc7227`. Их состояние **verify at session start**.
  Временный operational handoff: `/tmp/rimloc-glm-handoff.md` (может отсутствовать).
- Независимый baseline на `0e37b0c`: workspace build/test (168 passed), fmt check,
  clippy `-D warnings`; GUI check 0/0, vitest 20/20, production build — PASS.
  Это baseline, не acceptance будущих изменений и не live acceptance.
- Freeze зависит от L и доказанного provenance-контракта; GUI W4.5–W7 продолжается
  независимо. После freeze — compact delta review → real binding → live acceptance.

## Mock/live boundary
Моки/адаптеры до фриза; dev-бейдж «Demo data»; прод не шипит MockTransport (guard в
xtask/CI после фриза); Style Lab = dev-only (?stylelab=1); бизнес-логика вне фронтенда;
один транспорт (UI → RimLocClient → TauriTransport), никаких invoke() в компонентах;
project data не в фронтенд-форматах.

## ⏸ Pending owner decisions
1. Визуальное направление (Precision/Aurora/Workshop/Editorial + палитра + гибрид) —
   пакет ПОСЛЕ W3-W5 на представительных экранах (скриншоты: testlab/artifacts/
   gui-style-lab/; лаборатория :5199/?stylelab=1).
2. (мелкое) NoTranslate-финальность поднять из engine-guard в domain::resolve().

## Architecture constraints (не блокируют, но обязуют)
Appearance packs (APPEARANCE_PACK_ARCHITECTURE.md): L1 theme / L2 layout / L3 extensions,
декларативно, без кода, safe fallback, a11y-гейт; встроенные направления — будущие
first-party паки. Replaceable frontend (UI_SDK_MANDATE.md): логика вне фронтенда, один
транспорт, framework-neutral контракты, моки — постоянная dev-фича; альтернативные UI —
build-time, не сейчас.
