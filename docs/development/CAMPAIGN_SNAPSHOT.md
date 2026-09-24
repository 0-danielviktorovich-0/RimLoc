# CAMPAIGN SNAPSHOT — RimLoc (авторитетный слепок намерения/состояния)

## Правила этого файла
1. **Git — авторитет живых фактов репозитория**: в начале КАЖДОГО нового контекста
   проверить `git rev-parse HEAD`, `git status --short`, `git worktree list`, и df при
   дисковых утверждениях.
2. Этот файл — авторитет **намерения и состояния кампании** (гейты, полосы, инварианты,
   решения), НЕ изменчивых машинных фактов. Изменчивое помечено «verify at session start».
3. После каждого гейта/волны обновлять ТОЛЬКО: закрытые гейты, текущие полосы, следующие
   задачи, новые инварианты/дыры. Хронологический лог не вести — он живёт в git log.

## Цель и финальная граница
Довести продукт до качества публичной релизной версии: новичок проходит полный GUI-путь,
проект сохраняется и восстанавливается, переводы валидируются и собираются вне источников,
ошибки объяснимы, реальные моды проходят изолированную игровую проверку. Затем независимые
engineering/product passes, документация, упаковка и воспроизводимый пакет пользовательской
приёмки по AUTONOMOUS_PLAN.md. Моки, unit-тесты и CLI baseline не заменяют live acceptance.
Непроверенные платформы и внешние release-гейты явно остаются открытыми; локальный RC
готовится к распространению, но push, публикация и релиз требуют решения владельца.

## Операционная модель (решение владельца, 25.09)
GLM-координатор сессии владеет операционной интеграцией: рутинные ревью, верификации,
обновления снапшота и stage-acceptance — без ожидания Astra на обычные фиксы,
cherry-picks, гейт-чеки и роутинг задач. Те же 5 GLM-воркеров, реюз/резюм; новые
инстансы моделей не создавать. Astra — куратор только нерешённых архитектурных /
безопасностных / платных / релизных решений и финального RC-acceptance
(поля needs_astra в milestone-файлах /tmp/rimloc-control/milestones/). EPIPE-подобные
«успехи» = сбои: resume того же воркера с WIP. Рутинный сетевой retry — не повод для
Astra. Ресурсная дисциплина: только /tmp/rimloc-cargo-serial.py (flock, общий target,
CARGO_INCREMENTAL=0), без самодельных локов/вторых таргетов/избыточных полных прогонов.

## Verify at session start (изменчивое)
- Последний main-срез: `8484c4f` (BACKEND FREEZE; GUI-интеграция в gui-integration a5b9825) → **проверь `git rev-parse HEAD`**.
- Рабочее дерево: возможен незакоммиченный L-WIP в main; provenance и W4.5 —
  в отдельных worktrees. Владение и статус перепроверять; чужие изменения сохранять.
- Диск: свободное место разделяется с другими активными проектами; **проверяй df перед
  сборками и утверждениями**. Не размножать cargo-кэши в worktrees.
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
**BACKEND FREEZE (25.09, координатор по evidence; интегрированный HEAD `4b4d8c3`)**:
L observability (принят независимо: 203 теста/43 сьюта + CLI негатив/позитив контролы),
J typed identity + v2 миграция + честный WriteReport (независимое ревью L: P1 нет),
corpus harness на 3 реальных модах + синтетике: status=pass (дважды: J и координатор),
source hash-гард 1066 unchanged. Windows runtime/MSRV и подпись — явно неверифицированы
(открытые RC-пункты, не блокируют локальный freeze).

A TKey round-trip · B канон-инвентарь (один пайплайн) · C typed Resolution валидаторов ·
D coverage-регрессии (TODO=missing, absent=missing) · E внешний defs-dir · F hardening
аудита · G TKey-доки «система с 1.1» · H effective precedence (Keyed last-file/in-file
first, Defs first-file, SetOrAdd, LoadFolders-effective scan) · I1-I4 canonical model
(модель+bridge+persistence+acceptance A/B/C) · J контракт+движок (156 seed-правил,
NoTranslate-финальность) · K-core detect_source_changes · патч-этап (li-Class/
FindMod/or-предикаты, ~50% реального покрытия). Ключевые коммиты: 8595129, 98db4b2,
c634ee0, 5df8bd6, 8c49e83, 042e0b5, 34dc83d, 916202b, b29ef68, 0c882d2.

## BACKEND LANE — FREEZE (25.09)
Backend заморожен на интегрированном HEAD `4b4d8c3` (см. BACKEND FREEZE в закрытых
гейтах). Все правки бэкенда после фриза — только через delta-процесс: компактный дифф
от `50f9483` → независимое GLM-ревью → фиксы реальных P0/P1 → повторные гейты.
Дальше по бэкенду: typed binding seam (proposal v2: /tmp/rimloc-binding-first-slice.md)
и live acceptance — после GUI-гейта.

## GUI LANE — интеграция W6+W7 (сейчас)
- **W6 сдан** (`codex/w6-demo`, 8 коммитов `51ada48..eb9fe40`): глобальный Demo-бейдж
  (без DEV-гейта), демо-проект + no-mods действия, anchored тур на реальных действиях,
  dev-гейт scenario tooling, общий MOCK build engine, pending-save generation-гварды.
  85/85 тестов, check 0/0, build ok, браузер оба пути тура, Workshop Dark без wash.
- **W7 сдан** (`codex/w7-source`, 8 коммитов `9eef331..1982bbf`): SOURCE-таб (честный
  provenance, nullable локации), read-only Viewer (без innerHTML), palette/context menu,
  Advanced Browser + Compare, External editor argv, shortcut dispatcher (unbound),
  Unreleased. 99/99 тестов, check 0/0, build ok. Все 5 коррекций 029 закрыты.
- **GUI-интеграция** (`codex/gui-integration` @ `a5b9825`): волны W6 (42334a8) и W7
  (2c9f3e9) смёржены, repair Workspace merge artifact + реальная версия в бандле.
  Ожидается: независимое merge-ревью, полный гейт check/test/build + браузер
  (оба пути тура, dev-гейт, SOURCE-флоу, изоляция обычного проекта).
- Известный хвост: после смёрживания W4.5 заменить `MOCK_APP_VERSION` →
  `src/lib/version.ts` (инструкция в /tmp/rimloc-W45-handoff.md).
- **W6/W7 POST-FREEZE**: live binding + live acceptance на реальных данных
  (машина владельца: RimWorld 1.6 + Odyssey).

## Исполнение и интеграционный review
- Операционная модель — секция «Операционная модель» выше: GLM-координатор
  (sess_be9620ba) владеет интеграцией/рутинным ревью/снапшотом/stage-acceptance.
- Воркеры (reuse, не дублировать): L `agent_2ba7172f…`, W6 `agent_d2a2be70…`,
  W7 `agent_2b0654cc…`, J `agent_ab5a8765…`, corpus/W4.5 `agent_41830365…`.
  Состояние — verify at session start. Взаимные независимые ревью обязательны
  на интеграционных коммитах; каждый чек — через /tmp/rimloc-cargo-serial.py.
- Milestone-файлы: /tmp/rimloc-control/milestones/NNN.json (needs_astra только
  для архитектуры/безопасности/платного/релиза и финального RC).
## Mock/live boundary
Моки/адаптеры до фриза; dev-бейдж «Demo data»; прод не шипит MockTransport (guard в
xtask/CI после фриза); Style Lab = dev-only (?stylelab=1); бизнес-логика вне фронтенда;
один транспорт (UI → RimLocClient → TauriTransport), никаких invoke() в компонентах;
project data не в фронтенд-форматах.
Решения post-freeze binding сохранены в UI_SDK_MANDATE.md: persist-before-ack,
revision/epoch, защищённый source/output, реальная диагностика, закрытие unsafe legacy
Tauri routes и обязательная приёмка нового GUI; это требования, не статус реализации.

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
