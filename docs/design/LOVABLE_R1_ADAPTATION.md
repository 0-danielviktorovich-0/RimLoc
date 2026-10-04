# LOVABLE R1 ADAPTATION AUDIT (PHASE A)

Источник: `~/Developing/RimLoc-reference/lovable-r1/RimLoc-Lovable-Source-b2c6fc38`
(identity верифицирована: RimLoc Magic / `858f79e8…` / commit `b2c6fc38…`,
MANIFEST.sha256 — 0 расхождений). Целевая роль: visual/interaction/IA baseline
R1 + React migration seed. НЕ источник доменной правды.

Классификация: **ADOPT** (берём как есть) · **ADAPT** (берём с изменениями) ·
**IMPROVE** (идея верна, реализацию делаем глубже) · **REJECT** (прототипное,
в прод не идёт) · **PRODUCTION_ALREADY_BETTER** (у Svelte-продакшена уже есть
живая реализация сильнее).

Факты о прототипе: ~1.5k строк app-кода, TanStack Start (web), dense-стиль
(однострочники), состояние — in-memory React context (`store.tsx`), данные —
18 фикстурных строк VWE, валидация — браузерный regex (плейсхолдеры/дубликаты),
экспорт — клиентский zip/PO/CSV/XLIFF (fflate), сканирование/сохранение —
`setTimeout` имитация. Всё доменное — prototype logic → REJECT по умолчанию.

---

## Shell / каркас

| Зона | Вердикт | Основание |
|---|---|---|
| Общая композиция (sidebar 222px + topbar + workspace-heading + route-content) | **ADOPT** | Ядро «дорогого» вида: serif-дисплей над плотным UI, hairline-ритм |
| Sidebar: brand (Literata 27px + винная точка), карточка проекта с обложкой, nav 39px с active-accent | **ADOPT** | Фирменный жест; карточка проекта → станет живой (реальный проект/прогресс) |
| Sidebar-progress (label + 4px progress + «N из M строк») | **ADOPT** | Пересчитывается из реального снапшота |
| `nav-count` — красная пилюля ошибок на «Проверки» | **ADOPT** | Счётчик из project_validate |
| Topbar: breadcrumb 10px, demo-badge, theme-toggle, «Новый проект» | **ADAPT** | + состояние сессии/сохранения (dirty-индикатор из контракта) |
| workspace-heading: eyebrow 8px «ENGLISH → РУССКИЙ», Literata h1 30px, version-select, layout-switch | **ADAPT** | Eyebrow из реального источника/цели (Language Registry); h1-тексты — на i18n |
| Onboarding-strip (3 шага, винная левая граница 2px) | **ADOPT** | Плюс существующий route-scoped coach (§43: strip + coach marks + demo) |
| workspace-summary (микростатус-строка с dot-индикаторами) | **ADOPT** | Данные из validate/snapshot |
| Footer «только переводы, оригинал в безопасности» | **ADOPT** | Это реальная гарантия RimLoc — не копирайтинг прототипа |
| Layout-switch (Фокус/Студия/Обзор) | **ADAPT** | CSS-grid-трюк переносится; практическая ценность проверяется на representative screen (§25) |
| Help-panel, toast-message, busy-status | **ADAPT** | Заменить на Sonner + честный busy из реальных операций |

## Редактор / workspace (representative screen)

| Зона | Вердикт | Основание |
|---|---|---|
| 3-колоночный грид (file-tree 176 / entries 1fr / inspector 304) | **ADOPT + IMPROVE** | Заменить фиксированные колонки на react-resizable-panels с min/max и персистом (§26) |
| File-tree (DefInjected/Keyed группы, счётчики, «Исходники не изменяются») | **ADOPT + IMPROVE** | Группировка из реального инвентаря (def_type); down: прототип хардкодит имена файлов |
| Entry-row 63px: source+key / target / статус, line-clamp-2, selection = тинт + винная полоска | **ADOPT + IMPROVE** | **Обязательно TanStack Virtual** — прототип рендерит map() (18 строк); стресс 10k+ (§27) |
| Фильтры (Все/Без перевода/С замечаниями + поиск) | **ADOPT** | Предикаты — из реального lifecycle/validate |
| EntryEditor (detail-pane): source-block на тинте, locale-чипы, textarea, char-count, «Сохранить и дальше», prev/next | **ADOPT + IMPROVE** | Edit→confirm→next сохраняется; dirty/acked-семантика из контракта (уже есть в project.svelte store) |
| Inline-warning (первая находка в редакторе) | **ADOPT** | Findings из реального validate-контракта |
| Context-tabs (Контекст/Термины/XML) | **ADAPT** | Контекст → Source Inspector (source_ref, provenance, reveal/open — PRODUCTION_ALREADY_BETTER); XML-превью — осторожно (не эмуляция семантики) |
| Связанные строки (label ↔ description) | **IMPROVE** | Прототип хардкодит VWE; в проде — related по canonical identity (defName-группа) |
| «Машинный перевод» полоса внизу | **ADAPT** | Ведёт в реальный AI/no-API flow (chatbatch), не демо-провайдер |

## Страницы

| Зона | Вердикт | Основание |
|---|---|---|
| Home/Projects: карточка проекта с обложкой+прогресс+«Продолжить» | **ADOPT + IMPROVE** | Home = §21 (New/Open-existing/Recent/Help-translate); identity — adapter-aware (не raw ids) |
| Checks: metrics-band + finding-list + «Исправить» + passed-state | **ADOPT** | Counts/findings — только из реального валидатора (§35); категории error/warning/info |
| Compare (source/version/sets, PO-загрузка, diff-строки с «Принять») | **PRODUCTION_ALREADY_BETTER + ADAPT** | Живой Existing-flow уже делает классификацию и apply; визуальный diff-язык Lovable переносится на него |
| Glossary: таблица + импорт JSON + add-term | **PRODUCTION_ALREADY_BETTER + ADAPT** | Живой project_glossary CRUD (волна 13); визуальная форма Lovable переносится на контракт |
| Export (3 таба: сборка/экспорт/импорт; archive-tree превью; format-grid) | **ADOPT + IMPROVE** | Композиция отличная; наполнение — project_build_mod/export; archive-tree — из реального результата сборки, не хардкод |
| Tools (каталог 12 инструментов + настройки + terminal-output) | **ADOPT + IMPROVE** | Отличная форма для продвинутых CLI-возможностей; terminal-output — реальные stdout-мосты где есть, иначе честный UNSUPPORTED |
| Settings: Внешний вид/Параметры проекта/capability-таблица | **ADAPT** | Секции §37; capability-таблица — из handshake capabilities (честно LIVE/UNSUPPORTED) |
| Диалоги: New Project wizard (4 шага, wizard-dots, category-picks, success-emblem 38px) | **ADOPT + IMPROVE** | Джорни §22: источник → скан → язык → обзор → создание; категории = реальные EntryKind; «Открыть пример» = demo-проект |
| TranslateDialog (провайдер/область/бюджет/превью) | **ADAPT** | Реальный AI-стек: провайдеры из ProviderManager, no-API batches, «никогда не звать платное только потому что ключ есть» (§34) |

## Визуальная система

| Зона | Вердикт | Основание |
|---|---|---|
| OKLCH-токены: тёплые нейтрали, wine primary `.407 .123 15`, semantic roles (success/warning/destructive/info/sidebar/source/terminal/overlay) | **ADOPT** | Полный набор ролей §41 уже в прототипе — сеем как есть |
| Light/Dark как отдельные цели качества (полное переопределение .dark) | **ADOPT** | Обе темы — acceptance-цели |
| Типографика: Golos Text (UI 13px) / Literata (display 27-38px) / IBM Plex Mono (ключи/локали) | **ADOPT + IMPROVE** | Проверить кириллицу/японицу/офлайн-бандл и лицензии (§40); роль-based, serif не тащить в textarea |
| Eyebrow-микроподписи 6-9px + dot-индикаторы 5px | **ADOPT** | Единый язык статусов — главный опознавательный знак R1 |
| Плотность: entry-row 63px, панели 49px, hairline-разделители | **ADOPT** | Онтология «плотность по умолчанию» — HEURISTIC/DEFAULT, не инвариант (§17) |
| Радиусы 4-8px, одна тёплая тень, letter-spacing: 0 | **ADOPT** | |
| Responsive: 1500+/1150-/900-/700- брейкпоинты, off-canvas mobile-nav | **ADAPT** | Desktop-first Tauri; mobile-ветка упрощается, reduced-motion сохраняем |
| Старый оранжевый Workshop-dark Svelte UI | **REJECT** | Мандат §41: избегать оранжевого перегруза |

## Прототипная логика (всё — REJECT в прод, кроме идей)

| Зона | Вердикт | Основание |
|---|---|---|
| store.tsx (React context, in-memory, run()=setTimeout 850мс) | **REJECT** | Замена: RimLocClient + contract store (уже существует — переиспользуем семантику) |
| core.ts: fixture 18 строк, validate regex, po/csv/xliff/xml генераторы, parsePo, zip (fflate), download | **REJECT** | Всё живёт в Rust (парсеры, валидатор, экспортеры); UI никогда не дублирует домен (§6) |
| Демо-провайдеры, fake timers, браузерные скачивания | **REJECT** | Tauri: реальные пути/диалоги (§46) |
| TanStack Start / SSR / server.ts / start.ts | **REJECT** | §5: локальное Tauri desktop — React+Vite SPA |
| hardcoded RU/DE/FR, VWE-хардкоды, «RimLoc · прототип»-копирайтинг | **REJECT** | Language Registry + adapter-awareness + честные бейджи |

## Что прототип НЕ реализует (§12 — факт-чек, не переносить обещания)

Виртуализация списка — НЕТ; реальные resizable-панели — НЕТ; keyboard-first
CAT-переходы — НЕТ; inline token highlighting — НЕТ. Все четыре — наши
IMPROVE-цели production (§26/§27/§45/§29).

## Резюме

Ядро R1 = **токены (OKLCH warm+wine) + типографическая пара (Literata display /
Golos UI / Plex Mono) + eyebrow/dot-язык статусов + плотный грид-каркас с
hairline-ритмом + честные бейджи**. Компонентная форма переносится почти вся;
ВСЯ логика прототипа — REJECT; четыре «слабых места» прототипа становятся
обязательными production-улучшениями. Контракт фронт↔бэк уже существует и
проверен кампанией (RimLocClient: 19 typed ops + handshake capability report) —
React-слой реализуется ПРОТИВ него, без изменений домена.
