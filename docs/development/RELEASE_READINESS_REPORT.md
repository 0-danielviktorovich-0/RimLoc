---
type: reference
status: current
tags:
  - project/rimloc
  - kind/release-doc
last-reviewed: 2026-09-27
related:
  - "[[CAMPAIGN_SNAPSHOT]]"
  - "[[BETA_TEST_CHECKLIST]]"
  - "[[REVIEW_SCREEN_MAP]]"
  - "[[VERSIONING]]"
  - "[[SELFLOC_BRIDGE]]"
---

# RELEASE READINESS REPORT — RimLoc Local Release Candidate

**Дата:** 2026-09-29 (ночь 2: existing-flow + REL-2 + UI-кампания + багфиксы + REL-3; вечер: REL-4 + агентский слой + живая приёмка) · **Ревизии:** tested baseline `2f8ff0e` → `9fe14e4` → `925858a` → `0afac4a` → `6a3dddd` (багфикс-волна 2) → **current `main bad4a69`** · **Артефакты:** контрольный `8056649` (`f396b0e2…`) · `925858a` (`478268f3…`) · `0afac4a` (`d79eb21c…`, `artifact-rel3-final/`) · REL-4 `6a3dddd` (`62f3af14…`, восстановим: повторная сборка побайтово совпала, `artifact-rel4-final/`) · **финальный `bad4a69` (`8dbe6503…`, `artifact-rel5-auto/`)** — check/build/package PASS, EXECUTION PASS. Origin/main на `9591f4f`; пуш только по явному ок владельца.
**Живая UI-приёмка закрыта (вечер 29.09, экран разблокирован).** Канал: автоустановка в /Applications без DMG (`testlab/auto_install.py`) → System Events keystroke (Tab/Enter) + `screencapture -l <windowid>` + визуальная верификация кадров. Подтверждено на release-бинаре: 1) Home рендерит тур-кнопку «Мастер перевода (демо-тур)», чип «Живой транспорт», selfloc-кнопку, recents-меты `r·#id`; 2) Enter на тур-кнопке открывает мастер-тур (шаг 1 «Откройте демо-проект», кадр `wiz1.png`); 3) selfloc-дедуп живьём: managed 8→8 после открытия (кадры `s2-workspace.png`/`s3-now.png`, шапка displayName + EN → RU); 4) экран проверки с раздельными счётчиками (плейсхолдеры/TODO/дубликаты — кадр `r1.png`). Кадры: `~/Developing/RimLoc-evidence/live-accept-rel5/`. Ограничение канала: web AX-дерево WKWebView недоступно внешнему драйверу на видимом окне release (метод Tab+Enter вместо AXPress); координатные CGEventPost-клики блокируются средой.
**Дельты после baseline:** 27.09 — документация, i18n S1, K-гапы (MSRV/legacy/гварды), JSON-мост; ночь 28.09 — **SF-1..5 (независимый ревью: вклад安全性 contribution: base_value preconditions, prototype-id отказы, секреты в метаданных, дубликаты, настоящая цепочка export→pack→contribution — все подтверждены CONFIRMED-FIXED отдельным ревьюером)**, **волна честности UI (13 success-симуляций заменены на честные статусы)**, **нативный выбор папки (pick_directory) в создании проекта и build/diagnose**, **честный Review-обзор из реального снапшота**, **selfloc UI entry: «Перевести RimLoc (бета)» открывает собственный каталог как обычный проект (доказано на release-артефакте: 1187 записей, validate 0/0/0)**.
**Ночь 29.09 добавила:** existing-translation flow (dry-run анализ + guarded apply через канонические интенты; Existing-экран живой; leaf↔locale гвард; analyze≡apply на TODO), полную сборку мод-пакета в GUI (`project_build_mod`, гварды export 1:1, ModMetaData About — доказано на release: мод-пакет на диске), дедуп selfloc-карточки (переоткрытие вместо дублирования — доказано: 0 новых managed-файлов), **UI-кампанию**: автономный AX-аудит 21 экрана → баг-лист 3 high/11 medium/12 low (`/tmp/rimloc-ui-bugs.md`) → 5 дизайн-вариантов по скилл-линзам → жюри (топ: ui-ux-pro-max) → Home-редизайн имплементирован + переснято.
**Вечер 29.09 добавил (мандат владельца «полный автоматизм + FOCP-дебаг»):** багфикс-волна 2 (M-3..M-11 + L-пакет, `6a3dddd`, гейты 324 Rust / 381 фронт); **агентский слой автоматизации** (`2d59562`, `bad4a69`): `RIMLOC_AUTOMATION=1` — AX-хуки release-capable (App Nap opt-out, AX-активация; web AX-дерево WKWebView на видимом окне остаётся недоступным — канал клавиатурный), `RIMLOC_TRACE=1` — JSONL action-trace контракных команд (`<app-data>/RimLoc/logs/trace.jsonl`); **`testlab/auto_install.py`** — автоустановка/запуск/стоп установленной копии без DMG и Finder-диалогов (одна версия, атомарная замена); живая приёмка закрыта (см. шапку).
**Волна 3 (ночь 30.09):** баг-лист UI закрыт ПОЛНОСТЬЮ (3 high + 11 medium + 12 low; merge `9c26d89`, ревью APPROVED). Финальный RC-артефакт: **REL-6 `ff0fd61a…`** от `d03f1cd` (evidence `artifact-rel6-final/`; live-smoke волны 3 — кадры `live-accept-rel5/rel6-*.png`: маркер «Нет перевода», скроллбар, отсутствие «:0»; selfloc-дедуп 8→8). Гейты: rust 326/0, фронт 381/381, fmt/clippy/svelte-check чисто.

**Микро-волна 4 + инцидент stale-dist (30.09):** merge `e0e7acc` (--space-5: 20px в шкалу токенов; displayName в ProjectOverview, паритет с тулбаром; ревью APPROVED, 2 файла). **Инцидент:** REL-6 (`ff0fd61a`) собран со stale dist без фронта волны 3 — в tauri.conf отсутствовал `beforeBuildCommand`, cargo tauri build молча брал готовый dist; «подтверждение» маркера в smoke REL-6 было ведущим вопросом и перечёркнуто. Исправление: `beforeBuildCommand` = `cd frontend-v2 && npm run build` (cwd = каталог приложения, установлен пробой pwd; три фикс-коммита). **Финальный артефакт: REL-8 `d710b26b…`** (evidence `artifact-rel7-final/`, тот же SHA у REL-7 — свежий dist волн 3+4, теперь гарантирован пайплайном). Честная живая перепроверка нейтральным вопросом: серый курсив «Нет перевода» рендерится (кадр `live-accept-rel5/rel8-table.png`).

**Волна 5 (30.09, ночь):** merge `22edca9` — UI-точка «Помочь с переводом RimLoc» на экране справки (мандат self-loc): общий модуль `selfloc.ts` для Home и Help, дедуп-инвариант `'RimLoc UI (en)'` сохранён; разбор «мок-вспышек» закрыт вердиктом воркера с правками по пилюлям провайдеров (e7beef9); ревью REQUEST_CHANGES (first-run карточка на сырой функции — отказ глотался) исправлен (`fda3624`). Гейты: rust 326/0, svelte 0/0, vitest **383/383** (+2 теста selfloc-модуля). **Артефакт REL-9 `3c2df848…`** (evidence `artifact-rel9-final/`; первый, собранный с автосборкой фронта through beforeBuildCommand). Живая приёмка: экран справки → карточка → Enter по «Перевести RimLoc» → workspace, **managed 8→8 через Help-вход** (кадры `live-accept-rel5/r9c-*.png`).

**Волна 6 (30.09):** merge `35a3fd6` — строгая placeholder-валидация selfloc-строк (аудит §5: множество `{name}` base-vs-перевод для ui-catalog записей; потеря/добавление/переименование = error-finding `placeholder-check`; 6 юнит-тестов + интеграционный на реальном каталоге >1000 записей, без дублей с lost-placeholder) и перевод бэкенд-сообщений по code (аудит §4/§7: `contract.error.<code>` / `finding.<kind>` с fallback на серверный текст, код-префикс сохраняется, 24+24 ключа ru/en, тесты backend-messages). Ревью APPROVED (независимый контекст: совпадение форм токенов с фронтенд-рантаймом, отсутствие ложных срабатываний подтверждено E2E). Гейты: **Rust 333/0** (+7), svelte 0/0, **vitest 389/389** (+6). **Артефакт REL-10 `9c334a72…`** (evidence `artifact-rel10-final/`, дым-кадр `live-accept-rel5/rel10-home.png`).

**Волна 7 (30.09):** merge `8c3ba1f` — офлайн-бандл вклада из GUI (бета, self-loc мандат, локальная часть): Rust-порт контракта v1 в `rimloc-services/contribution` (ревью сверил посимвольно: константы, валидаторы, 7 секрет-паттернов в том же порядке, wire-поля, байт-форма записи TS-писателя), §6-гейт с `placeholder_set_mismatch` волны 6, гварды out-dir как у экспорта; tauri-команда в обоих списках; UI-блок «Вклад в перевод RimLoc (бета)» только на selfloc-проекте, нативный выбор папки, статусы READY/PARTIAL-BUT-VALID/NEEDS-FIXES со счётчиками и перечисленными отказами. Тесты: +14 Rust (включая запись бандла из реальной сессии на диск), +10 фронт (три статуса на моке, скрытие на не-selfloc). Гейты: **Rust 347/0, svelte 0/0, vitest 399/399**. **Артефакт REL-11 `a4bdb592…`** (evidence `artifact-rel11-final/`): живой рендер блока вклада подтверждён кадром (`live-accept-rel5/w7-s8.png`); кликовый прогон до статуса не доведён (ограничение Tab/Enter-канала на нативной панели выбора папки; окно закрыто владельцем) — бэкенд-путь живого сборщика покрыт интеграционными тестами на реальном каталоге.

**Вердикт: LOCAL RC READY (macOS).** Публикация (push, тег, релиз, подпись) — за владельцем.

## 1. Гейты приёмки на current `main 8056649` (реальные прогоны 27.09)

| Гейт | Команда | Итог |
|---|---|---|
| Формат | `cargo fmt --all --check` | чисто |
| Линтер | `cargo clippy --workspace --all-targets -- -D warnings` | чисто |
| Тесты Rust | `cargo test --workspace --no-fail-fast` | **324 passed / 0 failed** (+SF chain, +selfloc seam, +existing-flow 7) |
| Типы фронтенда | `npm run check` (svelte-check) | 0 ошибок, 0 предупреждений |
| Тесты фронтенда | `npm test` (vitest) | **356/356** (28 файлов; +UI-аудит фиксы 11, +сatalog hygiene, +bridge, +SF-safety, +picker/honesty, +selfloc UI, +existing-flow 15, +home-synthesis 4) |
| Release-компиляция GUI | `cargo check --release -p rimloc-gui` | зелёный (**это конфигурационный чек, не proof сборки** — сборка/запуск см. §2) |
| **Release BUILD+PACKAGE+EXECUTION** | `cargo tauri build` → запуск .app без dev-сервера | **PASS ×3** — 8056649 (контроль), 7795e54, **9fe14e4 (финальный)** — см. §2 |
| Corpus | verify-source-hashes (1066 файлов) + 8 acceptance-кейсов | PASS (ночная кампания; база та же, corpus не менялся) |
| Независимый ревью дельты | `9591f4f..fd618c2` | outdir-guard APPROVE; gui-gates REQUEST_CHANGES → исправлено (`997210f`); последующие волны — гейт-набор выше |

## 2. Release-артефакт: BUILD → PACKAGE → EXECUTION (27–28.09, три прогона)

Уровни строго разделены: **check** (конфигурационный, выше) ≠ **build** ≠ **package** ≠ **execution**.

- **BUILD/PACKAGE — PASS ×3.** `cargo tauri build` (release-профиль, arm64). Финальные артефакты ночи (git `9fe14e4`):
  - `RimLoc GUI.app` — sha256 `72426289…8fdb836`, 21.6 MB; DMG `0552ce6f…` 8.5 MB; selfloc-каталог физически в Resources (1187 сообщений);
  - контрольный `8056649` (`f396b0e2…`) и промежуточный `7795e54` (`c99b329a…`) сохранены в evidence;
  - подпись: adhoc/linker — **не подписанный дистрибутив**; notarization — блокер владельца.
- **EXECUTION — PASS.** Релизный бинарь из `.app/Contents/MacOS/` без dev-сервера: фронт из вшитого dist, ноль internet-сокетов у процесса, `RIMLOC_*` env сняты (дев-хуки отсутствуют в release как задумано), badge — живой контракт-режим.
- **Живой workflow на артефакте — 4/4 стадии** (stage-split; окна коротко на экране, terminate после каждой): home (recents из project_list) → открыть VWE 247 записей (**live-badge**) → validate (честная error-находка lost-placeholder) → export в абсолютный путь (2 файла, reparse 2/2) → diagnose (sanitized bundle с causal chain). Кадры: evidence `release-accept/`.
- Находки: AX-ввод пути работает как замена; «AX-дроп macOS 27» на экранных окнах не воспроизвёлся.
- **Selfloc-смоук на финальном артефакте (28.09):** бета-карточка «Перевести RimLoc» → обычный create-поток → workspace 1187 записей → validate «0/0/0» — сценарий D работает в release. Хвост: каждый клик создаёт новый проект selfloc (дедупликация recents — решение владельца).

## 2b. Матрица acceptance по релиз-критичным сценариям (не всё = узкий VWE-джорни)

| Сценарий | Статус | Evidence |
|---|---|---|
| Новый перевод (scan → проект → перевод → validate → export → build-mod) | **TESTED** (CLI 8 corpus-кейсов + GUI-джорни + release-джорни) | corpus-волны, §1-2 |
| Сопровождение существующего перевода (source-drift, rescan-совет, reusable) | **TESTED** (M3 + corpus) | perf-отчёт §4, corpus |
| Multi-target изоляция | **TESTED** (unit/corpus) | волна multi-target |
| No-API chat export/import, stale-response защита | **PARTIAL** — бэкенд-тесты PASS; живого GUI-джорни чата на built-артефакте не было | волны LLM |
| Реальные Core/DLC | **PARTIAL** — синтетика+VWE+4 мода corpus; интерактивная игровая загрузка — за владельцем | corpus-волны |
| Source inspection | **TESTED** (W7 + corpus) | W7-волна |
| Diagnostics/bug-report | **TESTED** (§2, sanitized bundle) | release-accept |
| Update/release pipeline | **BLOCKED** — updater спроектирован (`AUTO_UPDATE_DESIGN.md`), подпись/ключи — владелец | — |
| Платформенные пакеты | **macOS: built+launched+workflow-tested (автоматически); Windows/Linux: NOT TESTED** человеком (сборка — CI dev-профиль, отключён push-триггер) | §2, §4 |

Намеренно сломанные входы (lost-placeholder, relative-path refusal) — негативные тесты; «чистый законченный перевод» и «игра загрузила перевод» — не покрыты этим и помечены выше.

## 3. Производительность (15 216 записей)

Полный отчёт: evidence `reports/rimloc-performance-report.md` + `rimloc-kgaps-report.md` (K1).

| Операция | dev | **release** | Ориентир | Вердикт |
|---|---|---|---|---|
| scan / validate / export-po / build-mod | 1.52 / 3.17 / 0.22 / 0.18 s | — | < 10 s | PASS |
| session open (cold) | 0.68 s | — | < 2 s | PASS |
| session snapshot | 4.1 ms | 2.4 ms | < 2 s | PASS |
| **session apply (batch=100)** | 857 ms | **51 ms** (batch=1: 47.5 ms) | < 500 ms | **PASS в release (запас ~10×)** |

Dev-профиль остаётся медленным (857 ms) — это накладные отладочной сборки, не продуктовая характеристика; persist-before-ack не менялся (замер, не оптимизация).

## 4. Известные ограничения / закрытые находки

**Закрыто в этом closeout:**
- ~~K1 apply FAIL~~ → PASS в release (§3); dev-замер оставлен как факт.
- ~~K3 MSRV~~ → политика «tested on 1.96.0, requires 1.89», `rust-version="1.89"` во всех 16 манифестах, AGENTS.md исправлен.
- ~~K4 legacy write path~~ → аудит 17 write-команд: 6 брешей закрыто (CWD-relative отказы на merge_keyed_gui/export_xliff_gui/import_xliff_gui/dump_schemas/learn_patches_cmd; form-гард apply_translation); symlink-тест «авторизуется назначение, не форма»; абсолютность пути ≠ авторизация назначения — containment-гварды сохранены.
- macOS 27 AX-дроп — не воспроизводится на экранных окнах (release-прогон), ограничение сужено до off-screen автоматизации.

**Остаются (ночь 28.09 добавила по матрице полноты):**
- **Multi-target в live не существует** — снапшот жёстко маппит `ru` (`project.svelte.ts`), свитчер инертен; контракт v1 single-target — решение владельца (расширение контракта).
- **Chat batch — mock-only**, маршрут недостижим в release (открывался из скрытого Wizard).
- **«Обновить существующий перевод» недостижим из GUI** (Existing mock-only; бэкенд matching/merge готов и corpus-проверен; capability import_pack unsupported).
- **Selfloc: preview/contribution — dev/CLI-пути** (DevPanel dev-gated, скрипты); UI-точек проверки/preview в пользовательском потоке нет.
- **Build-mod в GUI** — #/build покрывает export/build-перевода контрактом; полная сборка мод-пакета — CLI (README не скрывает).
- **7 selfloc-дубликатов в recents** при повторных кликах карточки — дедупликация не реализована. (Уточнение 30.09, живой прогон REL-12: recents показывают 8 записей с уникальными `#-хвостами` — визуального дубля имён «RimLoc UI (en) ×7» это не снимает.)
- **M-7 (подтверждён живьём 30.09 на REL-12, фоновый AX-джорни):** вкладка «Проект» живого проекта — «Источник (мод)» без значения вовсе, «Сборка перевода» = шаблон `/Users/<user>/Projects/<mod>/Languages`, при открытых реальных путях бэкенда (247 записей, r2). Кадр: RimLoc-evidence/live-accept-rel12-ax/project-tab-m7-template-paths.png. Гейт-следствие: инвариант `honesty.no_template_placeholders` добавлен в манифест (T2a).
- **K2 — версия (владелец):** CHANGELOG заявляет `[0.1.0] - 2025-09-25`; реконсиляция показала — заявление внесено коммитом `00dc91e` 2025-09-25 через 13 минут после фиксации CLI `0.1.0-alpha.1` (противоречие с первого дня), тега нет ни локально, ни на origin. Решение о бампе/теге — владельцем (`VERSIONING.md` I1-I5). Имя DMG `0.1.0` отражает tauri.conf, не материализованный релиз.
- **M-10 ИСПРАВЛЕН 01.10 (в main, ждёт следующего release-артефакта):** отказ экспорта при невалидном пути видим — кнопка disabled + инлайн-причина (не-абсолютный путь), поздний guard-отказ рендерится внутри карточки у кнопки; регрессия tests/m10-export-refusal.test.ts 3/3. Сенсор post-action-состояний: fill-пре-шаги в семантическом снимке + компонентные тесты (контрактный build-экран в mock-браузере недостижим из роутера — задокументировано в sensor_gaps манифеста).
- **K5** — построчный сканер строк: многострочные eprintln вне поля зрения (P3).
- **K6** — off-screen AX-дроп macOS 27 (только автоматизация); AX-ввод = вставка в конец в dev (в release-джорни ввод worked as replacement).
- **K7 — платформы:** Windows/Linux NOT TESTED человеком.
- **Бэкенд-сообщения GUI** (`ContractError.message`/`Finding.message`) приходят свободным английским; локализация по `code` на фронте — спроектирована (SELFLOC_BRIDGE.md), не реализована.

## 5. CI и гейты владельца

**CI-матрица (честно):** 6 workflow Prepared: ci.yml (frontend/schema/actionlint/semver), semver.yml, release-plz (dispatch-only), publish.yml (recovery), docs.yml (ручной deploy), changelog-check. **Executed locally на `8056649`:** fmt, clippy, cargo test, svelte-check, vitest, actionlint-проверки CI-волны, mkdocs (release-аудит). **Not executed:** все GitHub-workflow прогоны (push-триггеры сняты — политика владельца сохранена; dispatch не запускался). Никакие «CI green» заявления не делаются.

**Гейты владельца (блокируют публикацию):**
1. Push дельты `9591f4f..8056649`.
2. Версия/тег (K2).
3. Подпись/notarization (Apple Developer ID) — до этого артефакт adhoc-подписанный, «локальный тестовый», не дистрибутив.
4. Windows/Linux человеком.
5. Возврат push/schedule-триггеров CI — политика владельца.
6. Бета-тестеры: `BETA_TEST_CHECKLIST.md` (переработан: обычный тестер vs разработчик).

## 6. Материалы

- **Evidence-хранилище (долговечное, вне git):** `~/Developing/RimLoc-evidence/2026-09-27-rc/` — MANIFEST.md (sha256, санитизация), built-acceptance кадры/логи, release-accept кадры + диаг-бандл, все отчёты волн.
- Кампании: `CAMPAIGN_SNAPSHOT.md`, `NIGHT_HANDOFF_2026-09-26.md`
- Ревью: Pass A/B, вердикт дельты, K-отчёт, i18n-аудит — в evidence `reports/`.
- Self-localization: `SELFLOC_BRIDGE.md` (мост), i18n-аудит (evidence `reports/`).
