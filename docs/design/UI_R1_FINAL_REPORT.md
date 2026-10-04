# UI R1 FINAL CAMPAIGN REPORT (§66)

Дата: 2026-10-05. Лайна: `glm/ui-r1-react`, worktree `wt-ui-r1`, HEAD `925ed98`
(+ отчёт/пакет). Основной main: `d0506fb` (Phase A-доки). Пуш/тег/релиз — НЕ
выполнялись (§67).

## React frontend status: PASS (representative + Phase E core)

Живые маршруты на реальном контракте (каждый — wdio-проба на настоящем
бэкенде, изолированные данные):
- **J1 Wizard** — путь (нативный пикер + ручной ввод) → версия → create
  (реальный Rust-скан) → workspace (04bdc62 + createViaWizard).
- **Workspace representative** — LEFT file-tree (реальный инвентарь,
  kind/def_type группы) / CENTER виртуализированный список (TanStack
  Virtual, 63px канон) / RIGHT редактор (edit→commit→next, persist-
  before-ack) + инспектор (файл/строка/why-this-source из source_ref).
- **J2 Existing** — dry-run import_existing → классификация
  (reusable/conflicts/new/obsolete/ambiguous) → apply_existing только
  переиспользуемого (0ec8e30).
- **Checks** — живой project_validate: metrics-band, фильтры категорий,
  отчёт JSON (8aed4ea).
- **Glossary** — project_glossary CRUD: list/upsert (case-insensitive)/
  delete с подтверждением (8aed4ea).
- **Build/Export J6** — validation-gate → build_mod / export в выбранный
  каталог (d25d69a).
- **Multi-target §32** — registry-driven переключатель цели (9 builtin
  языков), setTargetLocale ре-мапит из того же снапшота (7b19b49).
- **J3 редактор-джорни** — внутри Workspace smoke: правка → commit →
  ревизия растёт durably → правка переживает свежий снапшот.

## Svelte legacy status: FROZEN (не тронут)

frontend-v2 не изменялся React-лайной (кроме аддитивных react-* e2e-файлов
харнесса). Остается fallback/regression oracle до отдельного решения
владельца. Дефолтный tauri.conf по-прежнему собирает Svelte; React — через
`tauri.react.conf.json`.

## Lovable parity status: ПОЧТИ → доводки раунда 5 приняты

- Раунды критики 1-5 (независимый критик-агент, §80): 1-2 нашли и мы
  закрыли Literata/13px/eyebrow/wine-выделение/топбар; 3 — file-tree/
  multi-target/табы; 4 — dark-selection/плотность; 5 (финальная
  приёмка): «Уровень R1 почти достигнут… после этих двух правок — ДА»,
  правки D1 (топбар-кнопка inline-flex 36px) и D2 (dark-selection
  явный wine-тинт) применены (925ed98), кадры пересняты
  (RimLoc-evidence/ui-r1/).
- Остаток (LOW, в бэклоге): воздух карточек Home,ступенька hairline
  под топбаром.

## LIVE/PARTIAL/MOCK матрица (React UI, факт)

| Возможность | Статус |
|---|---|
| J1 Новый перевод (пикер/ввод → скан → create) | LIVE |
| J2 Existing (dry-run → классификация → apply reusable) | LIVE |
| J3 Редактор (select/edit/commit/next, dirty, persist) | LIVE |
| J4 Review/Checks (живой валидатор, фильтры, отчёт) | LIVE |
| J5 Glossary CRUD | LIVE |
| J6 Build/Export (validation-gate, reparse-verified) | LIVE |
| J7 Multi-target переключение цели | LIVE (редактирование второй цели — через импорт/интенты той же цели) |
| J8 Selfloc (каталог как обычный проект) | LIVE (contribution — через Build, §6-гейт) |
| J9 Diagnostics (safe bundle, operation_id) | LIVE |
| Multi-target UI (переключатель цели, 9 языков реестра) | LIVE |
| Settings: тема/язык UI, дефолты | LIVE |
| Capability-таблица из handshake | LIVE |
| TM-предложения у редактора | NOT RUN (домен TM live не собран — ждёт брифа) |
| AI/no-API chat batches в React | NOT RUN (Svelte-стор существует; перенос — Phase E) |
| Providers/LLM-ключи в React | NOT RUN (клиентский стор Svelte-мира; перенос — Phase E) |
| Command palette в React | NOT RUN |
| Language Manager (пользовательские языки) | NOT RUN (реестр 9 builtin скопирован; CRUD пользовательских — Phase E) |

Правило §42 соблюдено: ни один MOCK не выглядит как LIVE; в React-лайне
мок-транспорт вообще не бандлится (честный отказ), демо-бейдж — на
fixture-ветке Home.

## Performance (§55, базовые измерения, бейслайн)

- Стресс 20k строк: create→first-row-paint 11.9с (реальный скан+передача
  снапшота); селекция ×5 p50=222мс max=317мс; прыжки/поиск — канал WDIO
  в фоновом WebKit доминирует (честная оговорка, см.
  react-stress-baseline.log).
- Смоук-джорни целиком: 6.6с (5 секций).
- Сравнение с Svelte-бейслином: NOT RUN (нет эквивалентного замера на
  Svelte — вносить при owner-test).

## A11y (§56): PASS 8/8 (fdaa8a4)

Имена кнопок, tabindex-дисциплина, контраст (OKLCH→sRGB математика;
**найден и исправлен реальный дефект**: канонный muted-foreground Lovable
4.43/4.21 < 4.5 → токен .52/.72, девиация от канона задокументирована),
фокус-обход, label/for редактора, aria-label икон-кнопок, focusable
textarea. Human review — в очереди.

## Deterministic QA (§50): PASS 25/25 (faa7da6)

root-scroll · horizontal-overflow · blank-tail · controls-onscreen ·
panels-no-overlap · editor-inside-viewport · dark-applied — home/
workspace/checks/glossary/export + тёмные дубликаты.

## Первичные джорни (§58)

J1 PASS · J2 PASS · J3 PASS (edit+persist) · J4 PASS (validate) ·
J5 PARTIAL (glossary apply PASS; TM-предложения NOT RUN) · J6 PASS ·
J7 PASS (переключение; независимость целей пробой) · J8 PASS (open+edit;
contribution через Build) · J9 PASS (диагностика; «реальная неудача»
сценарий — ручная проверка владельцем).

## Визуальный QA (§51)

Кадры light/dark: home, workspace, checks, glossary —
`RimLoc-evidence/ui-r1/` (финальные после раунда 5). Независимый критик
×5: финальный вердикт — «после D1/D2 — ДА, не хуже Lovable R1» (925ed98).

## AI-OS design skill (§60-61)

`product-ui-design` (8 файлов, 614 строк, check-skills 0/0, generic) —
**на диске, коммит владельцу** (Layer7-атрибуция субагентских файлов).

## Remaining findings (честно)

1. Human a11y/visual review (§52/§56) — автоматические пробы PASS,
   человеческий проход не заменяет.
2. Perf-сравнение с Svelte (§55) — нет Svelte-бейслайна; React-бейслин
   записан.
3. Providers/LLM/chat/palette экраны — NOT RUN (Phase E продолжение).
4. Топбар-кнопка в dark — цвет-пара требует доводки (критик раунд 4).
5. DMG-стадия на NFS-таргете — дистрибуцию собирать на внутреннем томе.

## Owner test packet (Phase G)

- Артефакт: `RimLoc-evidence/artifact-rel15-production/RimLoc GUI.app`
  (sha256 `24e87ffd…`, release-guard static+runtime PASS, source
  `02e7ae1`+soak) — **ПРЕДУПРЕЖДЕНИЕ: собран ДО волны 13 в main; для
  owner-test волны 13 пересобрать** (команда ниже).
- Пересборка: `cd gui/tauri-app/src-tauri && rm -f capabilities/automation.json
  && CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/AI-OS cargo tauri build`
  (frontend-v2 = Svelte fallback; React-вариант owner-test — после
  переключения дефолта, отдельная задача).
- Тест-инструкции: запустить .app → Home → карточка проекта → Workspace →
  правка → Сохранить и дальше → Checks → Glossary → Existing → Build →
  Settings (тема) → Diagnostics.
- Регрессия владельцу: frontend-v2 остаётся дефолтом, пока React не
  подтверждён (§3/§48).
