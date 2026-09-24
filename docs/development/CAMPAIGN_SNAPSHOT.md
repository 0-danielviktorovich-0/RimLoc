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
- Последний main-срез: `dad131f` (L принят ad37903 + W4.5 + docs typed-intent/durable-ID;
  J identity gate открыт до 035-фикса) → **проверь `git rev-parse HEAD`**.
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
   Проверка WIP: счётчик TKey usages не заменяет реальные primary/other locations;
   target-only и TKey-only+foreign-pack не должны становиться English source;
   version-only layout без LoadFolders тоже обязан честно разрешать выбранную версию.
   Само наличие сериализуемого поля не закрывает Source Inspector gate.
   **Identity blocker доказан**: разные DefTypes с одинаковым `Dup.label` сливаются
   в bridge kind+key. До freeze — structured optional DefType discriminator,
   scoped import/matching и безопасная project-container v1→v2 migration; ambiguous
   старые проекты не переписываются и не получают угаданный перевод. Полный ID
   сохраняется в persistence, source-change detection и generated output.
3. **L observability — ПРИНЯТ на локальной платформе** (`ad37903` поверх `c8907cf`):
   doctor, support bundle + redaction preview, Copy-for-AI, structured operation logs.
   Lead независимо повторил build/fmt/clippy и **203 workspace tests (43 suites)**.
   Реальные CLI negative/positive controls на сохранённых фикстурах: пустой перевод
   и потерянный placeholder → failed; корректный `{0}` → succeeded с сохранённой
   info-находкой. JSON stdout целый, metadata отчёта видимы, manifest SHA/size верны,
   исходные файлы не изменены. Typed ValidationSeverity задаётся у каждого producer;
   все findings сохраняются, только ошибки проваливают операцию. Blind GLM-review
   только четырёх bundle-файлов правильно диагностировал обе ошибки и operation ID.
   Проверены compound secret/path masking, output containment (включая `..` после
   nonexistent component) и защита от заранее созданного temp symlink; общий
   source hash guard ранее подтвердил 1066 неизменённых файлов. Артефакты независимой
   проверки: `/tmp/rimloc-lead-L-026-*.log`, `/tmp/rimloc-lead-L-acceptance/`.
   Windows runtime и полный MSRV ещё не проверены; это локальная приёмка L,
   не backend freeze и не live GUI acceptance.
4. **BACKEND FREEZE** → компактный delta-бандл (дифф от 50f9483) для ChatGPT.
5. POST-FREEZE: real UI binding → LIVE ACCEPTANCE GATE (W6 §3 / W7 §26 мандатов).

## GUI LANE (сейчас)
Лендено: фаза 1 (роутер/rich-mock/Home/Wizard/Workspace UX), GUI-A (Review/Build/
Existing), GUI-B (Settings/Providers/Help/Cmd+K), W1 (wizard-автомат/replay/naming),
W2 (language registry/multi-target/stale-AI per-target), Style Lab, D1/D3 доки.
- **W3 лендена** (`935f70c`, acceptance `0f91203`): гибридная стратегия, chat-batch
  manager, stale-import/suggestion защита. Mandate identity guard: `91ed049`.
- **W4 лендена** (`3fa2039`): provider instances, glossary/TM, shortcuts, About.
  **W4.5 принята и включена в main** (`9cedd4e/ad9a002/d5c7c28`) с сохранением W5.
  Credential references/export, read-only
  reference-корпуса, shortcut safety, общая версия из GUI package metadata;
  transient connection status не клонируется, смена endpoint/key гасит stale probe.
  Lead проверил объединённый финальный срез `548f4cb`: check 0/0, **73 tests**, build.
  Git tree frontend-v2 в main совпадает с проверенным интеграционным срезом.
- **W5 mock принят** (`05876f5` + `13f7c31`): diagnostics + redaction preview,
  Project screen, QA по **8 персонам**; demo-подписи, sensitive-key redaction,
  сброс старого causal context при повторе и 22 новых регрессионных теста.
  Lead независимо повторил на `13f7c31`: check 0/0, vitest **42/42**, build;
  браузерный повтор на этом же срезе подтвердил demo-подпись, сброс старой причины,
  новый operation ID и сохранение этого ID в очищенном bundle.
  Общая версия из GUI package metadata интегрируется вместе с W4.5, без version bump.
  Live диагностика зависит от принятого L и будущего service binding.
- **W6 PRE-FREEZE — ACTIVE**: тот же W5-исполнитель продолжает в отдельном worktree
  `../_rimloc-worktrees/w6-demo` от `13f7c31`: честный глобальный demo-бейдж, scenario
  browser, anchored tour, изолированный demo project, REVIEW SCREEN MAP
  (MOCK_LIVE_ONBOARDING_MANDATE.md). Доставка и resume подтверждены.
  Первый срез включён только в `codex/gui-integration` (`3ef2e23`): check 0/0,
  **105 tests**, build. **Не принят**: браузер подтвердил tour-success без demo-сборки.
  `80a128c/51166f9` добавляют dev-guard и настоящие demo edit/fix, но ревью нашло
  преждевременный success до завершения build. Тот же GLM закрывает completion,
  cancellation/stale-epoch и запрет действий тура над обычным проектом (027).
- **W7 PRE-FREEZE — ACTIVE**: тот же GLM-исследователь переиспользован для source
  inspector mock в `../_rimloc-worktrees/w7-source` от `993115b`; ACK и running
  подтверждены. SOURCE tab, viewer, context/palette actions, source browser,
  external editor settings; backend-контракт пункта 2 пока не принят.
  Срез `9eef331..d8257c2` создан, но не принят: независимый review выявил потерю
  явного file target в viewer, искажение editor argv и неработающие remaps.
  Исправления 029 реально доставлены тому же GLM; нужны регрессии + browser QA.
- **W6/W7 POST-FREEZE**: live binding + live acceptance на реальных данных (машина
  владельца: RimWorld 1.6 + Odyssey).

## Исполнение и интеграционный review
- Lead: текущая Codex-задача; архитектура, snapshot, независимые проверки и интеграция.
  Основная реализация — существующие GLM-сессии ZCode; сначала reuse, не дублирование.
  Реализация — GLM-5.3-Flash через существующий план ZCode; Astra-субагентов нет.
  Один ограниченный независимый L-review выполнен GPT-5.6 Sol/medium через AI-OS
  Session Hub; findings проверены lead, reviewer завершён и detached, slot освобождён.
  Не переключаться на Ollama cloud/API ради делегирования: это отдельная авторизация.
- ZCode coordinator: `sess_be9620ba-0118-4457-8ee4-354d3e97c327` (RimLoc).
  L: `agent_2ba7172f-de60-4338-b7d9-8269ea223ee1`; W5:
  `agent_d2a2be70-b36b-4142-ad1b-f4480fbc7227`. Их состояние **verify at session start**.
  Временный operational handoff: `/tmp/rimloc-glm-handoff.md` (может отсутствовать).
- Provenance: `agent_ab5a8765-0acd-4e8e-9076-f91969976bfe`, worktree
  `../_rimloc-worktrees/rc-provenance`; W4.5:
  `agent_41830365-1e44-46e4-83d1-0f04f308ff13`, `../_rimloc-worktrees/w45-ux`.
  Старые J/W4 transports истекли; замены запущены только после неудачного reuse.
  `303d726` уже предок их базы; не портировать/реимплементировать существующий J.
- Provenance `feb0a63` прошёл независимое **статическое** GLM-ревью, без новых blockers
  кроме identity loss. На backend-integration `ce33458` lead повторил 12 provenance
  tests; этот срез содержит reproducer identity collision, не закрывает identity gate.
  028 требует typed ID maps, полное legacy type evidence, invariant check до save
  и сохранение фактически resolved версии после rescan; ACK доставки подтверждён.
  W4.5-reviewer переиспользован для узкого corpus acceptance helper в
  `../_rimloc-worktrees/rc-corpus` (ACK/resume подтверждены); реальные данные не коммитятся,
  вывод только в уникальный `/tmp/rimloc-corpus-*`, hash guard до/после, без `--update`.
- Доставку задания проверять по подтверждению активного turn; срочные UI-коррекции
  отправлять через Steer, не оставлять в очереди. Локальный mailbox
  `/tmp/rimloc-control/PROTOCOL.md` → background listener в существующем coordinator →
  native SendMessage → atomic ACK проверен реальными исправлениями и resume W6.
  Общий ZCode UI разделяется с другими проектами; mailbox устраняет гонку фокуса.
  Завершение отслеживает read-only CLI `app-server` → `session/subagents`; watcher
  `/tmp/rimloc-zcode-watch.py` без модельных вызовов. Нативный status success с текстом
  ошибки API считается transport failure, не успехом реализации. Runtime-хелперы
  в /tmp могут исчезнуть — перепроверять; ACK означает доставку, не приёмку кода.
- Общий cargo target между расходящимися worktrees дал подтверждённое повторное
  использование чужого workspace artifact. Все Rust-проверки сериализовать через
  `/tmp/rimloc-cargo-serial.py WORKTREE <cargo args>`: lock + при смене worktree очистка
  только workspace-пакетов, внешний dependency cache сохраняется. Проверить наличие
  wrapper при возобновлении. Не менять исходники под ошибки от чужих cached types;
  финальная приёмка — на едином интегрированном дереве.
- Независимый baseline на `0e37b0c`: workspace build/test (168 passed), fmt check,
  clippy `-D warnings`; GUI check 0/0, vitest 20/20, production build — PASS.
  Это baseline, не acceptance будущих изменений и не live acceptance.
- После L тот же worker готовит ограниченный read-only план первого typed binding
  seam; реализация начнётся после freeze/delta-review. Диск критически ограничен:
  новые target-кэши и повторные полные прогоны без причины запрещены; чистить только
  собственные воспроизводимые build artifacts в безопасной границе, не чужие данные.
- Freeze зависит от принятого L и доказанного provenance/identity-контракта; GUI W4.5–W7 продолжается
  независимо. После freeze — compact delta review → real binding → live acceptance.

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
