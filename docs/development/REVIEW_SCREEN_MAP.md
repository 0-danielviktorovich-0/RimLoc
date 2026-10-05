---
type: reference
status: current
tags:
  - project/rimloc
  - kind/release-doc
last-reviewed: 2026-09-30
related:
  - "[[RELEASE_READINESS_REPORT]]"
  - "[[BETA_TEST_CHECKLIST]]"
---

# REVIEW SCREEN MAP — карта экранов RimLoc GUI для ревью

Назначение: ревьюер/владелец открывает built-приложение и проходит экраны в этом порядке; для каждого — точка входа, что смотреть, какие состояния обязаны работать. Доказательства автоматизации: `/tmp/rimloc-accept2/` (кадры 00–19 двух прогонов 27.09), коммитнутые EN-кадры — `docs/screenshots/`.

## 1. Home

**Вход:** запуск приложения. **Скрин:** `01-home.png`, `11-home.png`.

- Строка «Недавние» — реальные проекты с диска через `project_list` (регрессия `dbae486`: список не молчит при живых файлах).
- Битый/чужой файл в managed-каталоге не роняет список (unloadable считаются, не скрываются молча).
- Онбординг-тур показывается один раз; dismiss-состояние персистится (повторный запуск — без тура).

## 2. Workspace (таблица записей)

**Вход:** «Открыть» на проекте. **Скрин:** `02-workspace.png`, `12-workspace.png`.

- Таблица рендерит записи (VWE 247 в acceptance; бэкенд-снапшот ~4 ms на 15k).
- Редактирование записи → dirty-состояние; рестарт приложения → черновик на месте (restart-safe).
- Статус-бар: data-mode badge честный (mock/live).

## 3. Recovery и ошибки контракта (ключевое ревью)

**Вход:** изменить файл проекта на диске извне → «Сохранить» в приложении.

- Баннер `role="alert"` с типизированной ошибкой `project_changed_on_disk` и кнопкой **«Принять версию на диске»** (refresh, disk wins, epoch+1, черновики сохраняются как dirty) — Pass A P1-1, тесты `w6-contract-binding.test.ts` «typed failure recovery».
- Любая typed-ошибка команды (`ContractError {code, message}`) видна в workspace-баннере — Pass A P1-2. Ветки `stale_revision`/`stale_epoch`/`contractAcked` не перекрыты (свои UI-состояния).
- Отказ интента (applied=0) показывает `code: message` первого skipped, не молчит.

## 4. Сборка перевода / Build

**Вход:** кнопка «Собрать перевод». **Скрин:** `03-build-screen.png`.

- Выбор целевой версии игры (pinned target version), понятный прогресс.

## 5. Проверка проекта (валидация)

**Вход:** «Проверить проект». **Скрин:** `04-validate-findings.png`, `14-validate-findings.png`.

- Находки с severity (Error/Warning/Info), классификация на месте эмиссии; «failed» только от Error.
- Проверенные зубы: lost-placeholder (`VWE_WeaponDeterioratedMessage` в acceptance), дубликаты, source-drift (после внешнего изменения `Languages/`) с советом rescan.
- Клик по находке ведёт к записи.

## 6. Экспорт

**Вход:** экспорт-панель. **Скрин:** `05`, `05b`, `15b`.

- Поля пути стартуют **пустыми** с подсказкой (литеральные `…`-дефолты убраны); кнопка запуска заблокирована, пока путь не абсолютный (`lib/paths.ts`: POSIX `/`, `C:\`, `C:/`, UNC).
- Относительный путь при обходе фронта → типизированный отказ `invalid_output_path` с понятным сообщением, **до любых fs-операций** (гард в `export_project` и `diagnose`, тест `relative_out_dirs_are_rejected_before_any_write`).
- Успех: «Записано и перепроверено», reparse-счётчик, файлы на диске.

## 7. Диагностика

**Вход:** «Полная диагностика». **Скрин:** `07-diagnostics-station.png`, `18-diagnose-bundle.png`.

- «Собрать бандл» поверх упавшей валидации: sanitized bundle с `report.md` (causal chain validate → error-findings, affected-записи), `diagnostics.json`, `environment.json`, `manifest.json`. Секретов нет.

## 8. Справка и bug-report

**Вход:** «Справка» → «Подготовить отчёт». **Скрин:** `06-help.png`, `09-bundle-preview.png`, `19-bundle-preview.png`.

- Превью Included-таблицы (13 полей), replay known-failure, путь сохранения выбирает нативный диалог (WebView не получает записываемый путь).
- Кнопка активна только после превью; экспорт bug-report не пишет в произвольные пути.

## 9. Дев-поверхность (только debug, ревьюю по желанию)

`RIMLOC_WINDOW_ORIGIN="x,y"` (+ `RIMLOC_WINDOW_MOVE=swizzle|borderless|tauri|hide`) — off-screen автоматизация: App Nap opt-out, AX-солиситация, 1 Hz park-поток. Блок атрибутно огорожен `#[cfg(all(debug_assertions, target_os = "macos"))]` — в release код отсутствует (гейт `cargo check --release -p rimloc-gui`). Пользователя не касается.

## 11. Ночь 29.09: Existing-поток живой

- **ProjectOverview → «Import existing translation»** (на live-проекте): путь через пикер или руками (абсолютный) → «Анализ» — dry-run `project_import_existing` (ничего не пишет): категории reusable/new/obsolete/ambiguous + конфликты, списки с лимитом.
- **«Применить»** — `project_apply_existing`: только после анализа и только для проанализированной папки (смена пути → disabled + подсказка); переносит только пустые слоты (origin=Imported), ручные правки не затираются (конфликты остаются списком); persist-before-ack, stale-ревизия ловится.
- Гварды: относительный путь → `invalid_output_path`; leaf ≠ локали → `contract_violation`; managed-root и symlink — deny fail-closed (симлинк на пачку внутри исходника читается легально).
- Тесты: `existing_pack_path_guards_are_typed`, `existing_pack_symlink_resolves_through_real_path`, `apply_existing_dirty_retry_completes_the_persist`, `apply_existing_moves_reusable_never_overwrites_conflicts`; фронт — `existing-flow.test.ts` (15).

## 12. Волны 29–30.09: новые точки ревью (owner-пакет)

- **Справка → «Помочь с переводом RimLoc»** (волна 5): карточка рядом с replay; кнопка ведёт тот же selfloc-флоу с дедупом (managed не растёт). Живой вход подтверждён кадрами live-accept-rel5/r9c-*.
- **Workspace → вкладка «Проект» на selfloc-проекте**: блок «Вклад в перевод RimLoc (бета)» (волна 7) — «Собрать вклад…» → нативный выбор папки → статус READY/PARTIAL-BUT-VALID/NEEDS-FIXES, счётчики, путь бандла, сворачиваемый список отказов. На обычных проектах блока нет.
- **Таблица записей**: пустые переводы — мягкий курсив «Нет перевода» (волна 3), не пустота; колонки не перекрыты скроллбаром; у карточек очереди нет мусорного «:0».
- **Вкладки Review/Glossary/TM/Project**: контент растягивается по высоте (blank-tail вылечен, волна 9); на Review при 0 проблем — осмысленная панель-сводка.
- **Вкладка ИСТОЧНИК на live-проекте** (волна 12): реальные файл/строка/«почему этот источник»; фикстурные usages скрыты честной пометкой. В mock — прежние демо-фикстуры с бейджем.
- **Пользовательские строки**: без внутренних терминов («контракт», транспорт) — вычищены (волна 9); ошибки контракта — по коду на языке интерфейса (волна 6), код виден префиксом.
- **Автоматизация**: npm run test:e2e — 17 семантических инвариантов headless (геометрия/фокус/honesty); see docs/development/testing/.

## 10. Ночь 28.09: новые точки ревью

- **Home → «Выбрать папку…»** (create-панель): открывает настоящий NSOpenPanel (доказано на release-артефакте); отмена тихая, ошибки видимые.
- **Home → «Перевести RimLoc» (бета)**: открывает собственный каталог как обычный проект (~1187 записей, live badge, validate 0/0/0); каждый клик — новый проект (known).
- **Build/Diagnose out_dir**: поля с пикером; relative-path отказ сохранён.
- **Review-обзор (contract-режим)**: счётчики из реального снапшота; невычислимые измерения — «—» с тултипом, не правдоподобные нули.
- **Настройки**: «Проверить обновления»/«Пересканировать»/логи/конфиг — честные «не подключено в этой сборке (демо)»; путь логов реальный; Steam/GOG — «пример данных»; провайдеры — «(мок)» в mock-режиме.
