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
---

# RELEASE READINESS REPORT — RimLoc Local Release Candidate

**Дата:** 2026-09-27 · **Ревизия:** `main 2f8ff0e` (локально; origin/main на `9591f4f`, локальная дельта `9591f4f..2f8ff0e` не запушена — пуш только по явному ок владельца)
**Вердикт: LOCAL RC READY.** Кодовая база прошла все локальные гейты приёмки, включая два независимых built-app acceptance-прогона. Публикация (push, тег, релиз) заблокирована до явного решения владельца — см. «Гейты владельца».

## 1. Гейты приёмки (все зелёные, реальные прогоны 2026-09-27)

| Гейт | Команда | Итог |
|---|---|---|
| Формат | `cargo fmt --all --check` | чисто |
| Линтер | `cargo clippy --workspace --all-targets -- -D warnings` | чисто (впервые полностью: objc/dev-hook блок приведён в порядок) |
| Тесты Rust | `cargo test --workspace --no-fail-fast` | **297 passed / 0 failed** (45 сьютов) |
| Типы фронтенда | `npm run check` (svelte-check) | 0 ошибок, 0 предупреждений |
| Тесты фронтенда | `npm test` (vitest) | **205/205** (15 файлов) |
| **Release-компиляция GUI** | `cargo check --release -p rimloc-gui` | зелёный (гейт добавлен после находки ревьюера F1) |
| Corpus | verify-source-hashes (1066 файлов) + 8 acceptance-кейсов | PASS (ночная кампания) |
| Built-app acceptance | off-screen journey на собранном приложении, VWE 247 записей | **12/12 × 2 независимых прогона** (`/tmp/rimloc-built-acceptance-run2.md`, скриншоты `/tmp/rimloc-accept2/`) |
| Независимый ревью дельты | `9591f4f..2f8ff0e` | outdir-guard APPROVE; gui-gates REQUEST_CHANGES (P1 release-компиляция) → **исправлен** `997210f`, влит `2f8ff0e` |

## 2. Что доказано на built-приложении (не на тестах, а на живом GUI)

- Home показывает реальные недавние проекты с contract-транспорта (проверка регрессии `dbae486`).
- Полный цикл: открыть VWE (247 записей) → редактор → собрать перевод → проверить (честный error-finding «потерянные плейсхолдеры») → записать экспорт («Записано и перепроверено», файлы на диске, reparse 2/2) → полная диагностика → sanitized support bundle (causal chain по реальному упавшему validate) → превью bug-report (13 полей).
- Recovery-баннер после внешнего изменения файла проекта: «Принять версию на диске» (Pass A P1-1), typed-ошибки workspace видны пользователю (Pass A P1-2).
- Относительный out_dir теперь типизированно отклоняется (`invalid_output_path`) на export и diagnose **до любых fs-операций**; дефолт-строки с литеральной `…` убраны из UI.

## 3. Производительность (15 216 записей, dev-сборка — консервативно)

Полный отчёт: `/tmp/rimloc-performance-report.md`, харнесс — `crates/rimloc-services/examples/perf_bench.rs`.

| Операция | Медиана | Ориентир | Вердикт |
|---|---|---|---|
| scan | 1.52 s | < 10 s | PASS (6.6×) |
| validate | 3.17 s | < 10 s | PASS |
| export-po / build-mod | 0.22 / 0.18 s | — | PASS |
| session open (cold) | 0.68 s | < 2 s | PASS |
| session snapshot (таблица GUI) | 4.1 ms | < 2 s | PASS (~500×) |
| **session apply (save, batch=100)** | **857 ms** | < 500 ms | **FAIL (1.7×)** |

`apply` — единственный FAIL: persist-before-ack перезаписывает весь envelope (8 MB на 15k) + двойной SHA-256. Стоимость плоская по батчу (интенты почти бесплатны). Варианты (дебаунс/инкрементальный журнал/фоновый persist) требуют анализа контракта persist-before-ack — **не трогались в RC**, задокументированы как известное ограничение (см. §4, K1).

## 4. Известные ограничения (честно, без приукрашивания)

**K1 — Save на больших модах:** ~0.85 s на 15k записей за каждый инкрементальный save (см. §3). Для типичных модов (<3k записей) малозаметно. Бэклог с вариантами.

**K2 — Версионный разрыв (I1 из release-аудита):** CHANGELOG заявляет `[0.1.0] - 2025-09-25`, но тега нет, CLI в `0.1.0-alpha.1`. Решение о бампе версии/теге — за владельцем (`docs/development/VERSIONING.md`, I1–I5).

**K3 — MSRV-декларация:** AGENTS.md репозитория заявляет MSRV 1.70, реальный минимум выше (например `is_multiple_of` требует 1.87+). Требуется ревизия деклараций отдельной задачей.

**K4 — Legacy-поверхность:** при операторском opt-in `RIMLOC_LEGACY_COMMANDS=1` команда `merge_keyed_gui` по-прежнему принимает относительный out_dir (тихо подставляет базу). Обычные пользователи этой поверхности не видят.

**K5 — Сканер-тест:** `no_hardcoded_user_strings_anywhere` построчный — многострочные `eprintln!` не флагуются (слепая зона, P3).

**K6 — Автоматизация, не продукт:** macOS 27 выбрасывает off-screen окно из AXWindows через 2–30 с (исследование ночи 27.09); на пользователей не влияет, канон acceptance — stage-split. AX-ввод текста работает как вставка в конец — ограничение автоматизации, не UI.

**K7 — Платформы:** проверено на macOS (M4 Pro, macOS 27). Windows/Linux — компилируются в CI-джобах dev-профиля, человеческой верификации не было.

## 5. Гейты владельца (блокируют публикацию, не блокируют RC)

1. **Push** локальной дельты `9591f4f..2f8ff0e` в origin/main — только по явному ок.
2. **Версия и тег**: решение по I1 (бамп до `0.1.0` или честный `0.1.0-alpha.x`), затем тег `vX.Y.Z` от main.
3. **Подпись/notarization**: Apple Developer ID, Tauri updater keys — только владелец; ключи никогда не изобретаются.
4. **Windows/Linux**: человеческая верификация сборок.
5. **CI-политика**: push/schedule-триггеры сознательно сняты (решение владельца 26.09); возврат — его решение. Ожидают внимания: dependabot github-actions PRs, branch protection, coverage-baseline, судьба `publish.yml` (спроектировано в `docs/development/CI.md`).
6. **Бенч тестеров**: `BETA_TEST_CHECKLIST.md`.

## 6. Материалы

- Кампанию-слепок: `CAMPAIGN_SNAPSHOT.md`; ночь 26/27: `NIGHT_HANDOFF_2026-09-26.md`
- Acceptance-матрица ×2: `/tmp/rimloc-built-acceptance-run2.md` (+ скриншоты 00–19 в `/tmp/rimloc-accept2/`, коммитнутые EN-кадры — `docs/screenshots/`)
- Ревью: Pass A (`/tmp/rimloc-pass-a-engineering.md`), Pass B (`/tmp/rimloc-pass-b-hostile.md`), вердикт дельты 27.09 (в логе `/tmp/rimloc-glm-handoff.md`)
- Карта экранов для ревью: `REVIEW_SCREEN_MAP.md`
