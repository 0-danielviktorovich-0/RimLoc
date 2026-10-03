---
type: reference
status: current
tags:
  - project/rimloc
  - kind/testing-architecture
last-reviewed: 2026-09-30
related:
  - "[[CAMPAIGN_SNAPSHOT]]"
  - "[[RELEASE_READINESS_REPORT]]"
---

# AUTONOMOUS ACCEPTANCE ARCHITECTURE — RimLoc

Авторитетная архитектура автономной приёмки. Персистена из мандата владельца
(2026-09-30), чтобы переживать компакты и смены сессий. Живые артефакты конфигурации —
`RIMLOC_ACCEPTANCE_MANIFEST.yaml` и `ACCEPTANCE_MATRIX.md` рядом; конвейер волн —
`docs/development/CAMPAIGN_SNAPSHOT.md`.

## Принципы (канон)

- **Semantic-first.** Управление и утверждения — через ARIA-роль, accessible name,
  стабильный testid и DOM-состояние. Координаты/пиксели — только когда семантика
  недоступна, и никогда как первичный драйвер.
- **Same-host по умолчанию.** Без VM. Физические мышь/клавиатура хозяина не трогаем:
  pointer не двигается, фокус переднего приложения не крадится, Spaces не переключаются,
  буфер обмена не меняется (если явно не тестируем его).
- **Vision — вторичный сенсор** для визуальных фактов (тема, эстетика, наложение).
  Визион-модель НЕ решает, существует ли кнопка или прошла ли backend-операция.
- **UI-успех коррелирует с правдой домена:** файловая система, revision проекта,
  вывод команд — обязательная часть утверждения успеха (мандат §21: causal chain
  UI → IPC → service → revision → FS → показанный результат).
- **Источники только read-only** (RimWorld, Workshop, сторонние моды, сейвы);
  выводы и тестовые профили — изолированные.
- Каждый release-acceptance фиксирует **исходный SHA + SHA артефакта**.

## Лестница тиров

| Тир | Что | Когда обязателен |
|---|---|---|
| T0 | domain/service (cargo test) | каждая волна, всегда |
| T1 | headless frontend (vitest, browser-mode) | каждая фронт-волна |
| T2 | real Tauri semantic E2E (WebDriver, IPC+backend живые) | release-gate, новые live-workflow |
| T3 | native OS integration (установка, запуск, логи) | release-gate |
| T4 | package/install/update | release-gate |
| T5 | visual/design review (пакет кадров, Light/Dark, RU/EN, narrow/wide) | перед owner-пакетом |
| T6 | real product/game integration (RimWorld грузит перевод) | перед публичным релиз-решением |

Дорогие тиры не гоняются для нерелевантных изменений (см. матрицу).

## Приоритет контроля/сенсора

1. canonical service/domain API (контракт-команды, tauri invoke)
2. Tauri/WebDriver semantic control (`@wdio/tauri-service`, embedded WebDriver macOS)
3. accessibility / generic computer-use adapter (кандидат: open-computer-use — спайк)
4. native OS automation (System Events; текущий Tab/Enter-канал — fallback-класс)
5. coordinate computer-use (CGEventPost — в этой среде блокируется)
6. vision/пиксели (screencapture + analyze_image — вторичный сенсор)

Используется самый высокий слой, способный доказать поведение.

## Same-host non-interference контракт (измеримо)

Для routine-приёмки фиксируем до/после: позиция физического pointer-события не
генерируется; frontmost-приложение хозяина не меняется; активный Space тот же;
clipboard неизменен. Владелец за машиной — обязательное условие дизайна теста
(инцидент 30.09: Tab/Enter-джорни перехватывался переключением фокуса хозяина).
Off-screen окно (RIMLOC_WINDOW_ORIGIN, dev) и `screencapture -l <windowid>` —
допустимые неинтерферентные сенсоры. Нативные диалоги выбора папки, требующие
физического клика, классифицируются foreground/native acceptance и не входят в routine.

## Платформенное ограничение macOS (research 30.09, v2.tauri.app + lib.rs)

Apple НЕ предоставляет WebDriver-реализацию для WKWebView → официальный
`tauri-driver` на macOS НЕ работает (Windows/Edge и Linux/WebKitWebDriver только).
Поэтому T2 расщеплен:
- **T2a — browser-mode semantic E2E (headless Chromium + Playwright)**: реальный
  фронт RimLoc (та же кодовая база) в mock-транспорте; семантические локаторы
  (role/name/testid), геометрия (boundingBox), скролл/фокус/оверлей-инварианты.
  Same-host, неинтерферентно (headless), быстрые. ПОКРЫВАЕТ все фронт-классы
  owner-дефектов (blank-tail, фокус-бюджет, overlay-коллизии, hscroll).
- **T2b — real Tauri IPC**: на этой машине — AX/keystroke-канал (T3-класс) или
  спайк tauri-webdriver-automation (крейт-обходчик; зрелость проверить);
  живые backend-эффекты дополнительно доказываются T0+T3 (auto_install + live).
Отклонение от «@wdio/tauri-service как первичный» зафиксировано осознанно:
мандат сам предписывает browser mode для frontend-only сценариев, а физический
драйвер на macOS недоступен.

## Известные ограничения среды (2026-09-30)

- web AX-дерево WKWebView release-сборки недоступно внешнему System Events-драйверу
  (макОС); слепой KVC AXManualAccessibility абортирует процесс — guard обязателен.
- Координатные клики (CGEventPost) из osascript-сессии не доставляются вовсе.
- Рабочий неинтерферентный канал сегодня: System Events keystroke (Tab/Enter) на
  frontmost-окне + снимок окна по windowid + анализ кадра. Он медленный и хрупкий —
  потому целевой канонический драйвер: T2 WebDriver (tauri-service).

## Evidence-journal (мандат §22)

Каждый run: manifest + timeline JSONL + семантические наблюдения UI + operation events
(operation_id = run_id + scenario_id + шаг) + фронт/бэкенд логи (RIMLOC_TRACE=1,
gui.log) + filesystem delta + environment + summary. Секреты и личные пути — санитизация.
Скриншоты опциональны (визуальные тесты/фейлы/финальный дизайн-ревью).

## Long-run lessons (owner soak-hardening, 03.10) — generic для RimLoc/BookKeeper/FreeWorld

1. **LONG-RUN TESTS MUST VERIFY TARGET ARTIFACT IDENTITY BEFORE EXECUTION.**
   Источник: 60-мин soak 03.10 молча ехал на rel14 (дефолт враплера), BIN-override
   раннера не доходил до спавна. Канон: перед циклом 1 — identity-гейт (файл sha256,
   класс поверхности, expected source commit) + приложение САМО сообщает
   `build_identity` (build.rs → RIMLOC_SOURCE_COMMIT, команда / build_identity).
   Mismatch = ABORT до цикла 1, никогда не warning. Регрессия: soak-preflight.sh
   --selftest (fail-closed + три пути враппера).
2. **STATEFUL UI ACTIONS REQUIRE SEMANTIC COMPLETION CONDITIONS, NOT IMMEDIATE
   POST-ACTION SNAPSHOTS.** Источник: «клик → немедленный isExisting» гонил
   ~20% ложных отказов под нагрузкой (macOS 27 WebKit откладывает диспетчеризацию
   в нефокусированном окне — upstream #540 класс). Канон: один семантический клик →
   waitUntil(route hash) → waitFor(landmark); route truth = hash + landmark
   (заголовок один никогда не PASS); латентность route/render — раздельные метрики
   (p50/p95/p99/max в отчёте); ретраев нет; таймаут = FAIL цикла с сохранённым
   evidence; счётчики раздельные (functional / route_timeouts / render_timeouts /
   driver_errors) — никогда не сваливать в один «error».
3. **0 failures при 4.9-секундной навигации ≠ 0 failures при 20мс.** Латентность
   навигации — метрика отчёта: деградация производительности не прячется за
   retry/wait-логикой.
4. **EXPECTED IDENTITY БЕРЁТСЯ ИЗ ЗАПИСИ СБОРКИ АРТЕФАКТА, НЕ ИЗ ЖИВОГО HEAD.**
   Источник: ложный старт soak #3 — раннер ждал текущий HEAD, а бинарь был испечён
   на предыдущем коммите (харнесс-коммиты двигают HEAD). Канон:
   `<артефакт>/source-commit.txt` пишется при сборке; спека верифицирует ЖИВОЙ
   процесс против этой записи; суффикс `-dirty` — объявленный (gitignored-оверлей
   capability), срезается при сравнении и не маскирует неверный коммит.
5. **BACKGROUND-ОКНА: УСЛОВИЯ СУЩЕСТВОВАНИЯ, НЕ ОТОБРАЖАЕМОСТИ.** Источник:
   первый запуск спеки v3 дал 20/20 фантомных render-таймаутов при 100%
   достигнутых hash-маршрутах — WDIO `element.waitFor` поллит isDisplayed,
   а окклюдированное фоновое WebKit-окно отвечает displayed=false навсегда.
   Канон: в фоновых прогонах ожидание landmark = waitUntil(isExisting);
   сплит route/render изоляет такой фантом немедленно.
6. **WIRES ПРОВЕРЯЮТСЯ ЖИВЫМ ПРОГОНОМ, НЕ ТИПАМИ.** Источник: build_identity
   ответил camelCase (прецедент AppInfo), TS-DTO ждал snake_case — svelte-check
   зелёный, живой префлайт упал. Первое живое обращение к новой команде — часть
   гейта, а не «после того как всё собрали».
