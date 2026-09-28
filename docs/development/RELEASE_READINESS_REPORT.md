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

**Дата:** 2026-09-28 (ночь: SF-фиксы + волна честности UI + selfloc entry) · **Ревизии:** tested baseline `2f8ff0e` → **current `main 9fe14e4`** · **Финальный артефакт ночи:** бинарь sha256 `72426289…8fdb836`, DMG `0552ce6f…` (git `9fe14e4`, evidence `artifact-final-night/`; контрольный `8056649` сохранён). Origin/main на `9591f4f`; пуш только по явному ок владельца.
**Дельты после baseline:** 27.09 — документация, i18n S1, K-гапы (MSRV/legacy/гварды), JSON-мост; ночь 28.09 — **SF-1..5 (независимый ревью: вклад安全性 contribution: base_value preconditions, prototype-id отказы, секреты в метаданных, дубликаты, настоящая цепочка export→pack→contribution — все подтверждены CONFIRMED-FIXED отдельным ревьюером)**, **волна честности UI (13 success-симуляций заменены на честные статусы)**, **нативный выбор папки (pick_directory) в создании проекта и build/diagnose**, **честный Review-обзор из реального снапшота**, **selfloc UI entry: «Перевести RimLoc (бета)» открывает собственный каталог как обычный проект (доказано на release-артефакте: 1187 записей, validate 0/0/0)**.
**Вердикт: LOCAL RC READY (macOS).** Публикация (push, тег, релиз, подпись) — за владельцем.

## 1. Гейты приёмки на current `main 8056649` (реальные прогоны 27.09)

| Гейт | Команда | Итог |
|---|---|---|
| Формат | `cargo fmt --all --check` | чисто |
| Линтер | `cargo clippy --workspace --all-targets -- -D warnings` | чисто |
| Тесты Rust | `cargo test --workspace --no-fail-fast` | **313 passed / 0 failed** (+SF chain, +selfloc seam) |
| Типы фронтенда | `npm run check` (svelte-check) | 0 ошибок, 0 предупреждений |
| Тесты фронтенда | `npm test` (vitest) | **320/320** (24 файла; +сatalog hygiene, +bridge, +SF-safety 25, +волна 2 (пикер/честный Review) 15, +selfloc UI 8) |
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
- **7 selfloc-дубликатов в recents** при повторных кликах карточки — дедупликация не реализована.
- **K2 — версия (владелец):** CHANGELOG заявляет `[0.1.0] - 2025-09-25`; реконсиляция показала — заявление внесено коммитом `00dc91e` 2025-09-25 через 13 минут после фиксации CLI `0.1.0-alpha.1` (противоречие с первого дня), тега нет ни локально, ни на origin. Решение о бампе/теге — владельцем (`VERSIONING.md` I1-I5). Имя DMG `0.1.0` отражает tauri.conf, не материализованный релиз.
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
