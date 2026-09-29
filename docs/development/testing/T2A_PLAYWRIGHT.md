---
type: reference
status: current
tags:
  - project/rimloc
  - kind/testing-architecture
last-reviewed: 2026-09-30
related:
  - "[[AUTONOMOUS_ACCEPTANCE_ARCHITECTURE]]"
  - "[[RIMLOC_ACCEPTANCE_MANIFEST]]"
---

# T2A_PLAYWRIGHT — semantic frontend invariants (headless Chromium)

Реализация **T2a** из `AUTONOMOUS_ACCEPTANCE_ARCHITECTURE.md`: реальный фронт
(`gui/tauri-app/frontend-v2`) под headless Chromium + Playwright, **mock-транспорт**
(дефолт dev-сервера: `import.meta.env.DEV` → devMode → client mode `mock`).
Инварианты — классы (boundingBox / DOM-состояние), не скриншот-ассерты.
Мандат 03 §3 (semantic control), §8 (geometry), §9 (scroll ownership), §10 (focus).

## Запуск

```bash
cd gui/tauri-app/frontend-v2
npm run test:e2e        # playwright test --config e2e/playwright.config.ts
```

Конфиг сам поднимает vite dev на **порту 5199** (`--strictPort`, `reuseExistingServer`
вне CI), проект — только `chromium`, `headless: true` — **никогда не переключать на
`false`**: контракт same-host невмешательства (мандат §4). Хостовые pointer/фокус/
Spaces/clipboard не затрагиваются. Прогон ~5 c, 17 тестов. Браузер ставится один раз:
`npx playwright install chromium` (кэш `~/Library/Caches/ms-playwright`).

HTML-отчёт: `npx playwright show-report` (артефакты — `frontend-v2/test-results/`,
в git не попадают). Скриншоты и trace прикладываются **только к падениям**
(`screenshot: only-on-failure`, `trace: on-first-retry`); visual-diff в этой волне нет —
это T5.

## Покрытие (инвариант → спека)

| Инвариант манифеста | Спека | Что проверяет |
|---|---|---|
| `geometry.no_giant_blank_tail` | `e2e/invariants/geometry.spec.ts` | Home/Workspace/Review: последний содержательный элемент `main`-области (видимый, со своим текстом или контрол) достигает ≥60% высоты viewport **или** данные продолжаются (есть real scroll-контейнер: `scrollHeight > clientHeight` при `overflow-y: auto/scroll`). Замер 30.09: Home 915px (127%), Workspace 2895px (402%), Review 1950px (271%); скроллятся `section.home`, `div.table`, `section.review`. |
| `geometry.no_unintended_hscroll` | там же | `document.scrollingElement.scrollWidth <= clientWidth + 1` на тех же трёх экранах (факт: 1280 = 1280). Внутренние скроллы панелей — легальная собственность (§9), документ не двигается. |
| `geometry.cta_visible_nonzero` | там же | Главная CTA Home («Продолжить», `btn-primary[data-testid^=home.recent-continue]`) и Workspace (`workspace.cta-review`/`cta-build`) — boundingBox > 0 и целиком во viewport. |
| `geometry.sticky_header_no_cover` | там же | `header.app-header` (role=banner): header.bottom ≤ top первого контент-блока + проба `elementFromPoint` внутри первого блока резолвится в `main`-контент, а не в шапку/оверлей. |
| `focus.tablist_roving` | `e2e/invariants/focus-tablist.spec.ts` | Два теста: (1) **бюджет Tab с body до вкладок** — см. «известный долг»; (2) стрелки переключают вкладку **без Enter** (selection follows focus: фокус и `aria-selected` совпадают после ArrowRight/ArrowLeft), ровно одна вкладка верхней таб-панели с `tabindex=0`. |
| `overlays.inside_viewport` | `e2e/invariants/overlays.spec.ts` | Найденные в mock-режиме оверлеи: (1) коуч-тур (`onboarding.overlay`, открывается сам на первом Workspace-монте свежего контекста) — карточка и подсветка якоря во viewport на всех 4 шагах, Back/Skip работают; (2) менеджер языков (`languages.manager.dialog`, открыт семантически: `languages.switcher.button` → `languages.switcher.add`) — внутри viewport, закрывается Escape. |
| `honesty.no_mock_action_in_prod` (статус-класс) | `e2e/invariants/honesty-mock-badge.spec.ts` | На **каждом** из 13 маршрутов роутера в mock-режиме виден честный бейдж: `transport-mock-badge` («Mock transport») или `mock-badge` («Demo data (mock)» после открытия демо-проекта). Список экран→бейдж прикладывается к отчёту теста (`mock-badge-screens.json`). |

## Известный долг (wave-вход для UX): фокус-бюджет

`focus.tablist_roving` — бюджет Tab с body до вкладок Workspace: **факт 16 нажатий**
(dev-режим; 14 продуктовых — без dev-панели, чей `<summary>` первый tab-stop в DOM и
съедает ещё один переход). Бюджет волны — 12. Тест **не ослаблен**: он каждый раз честно
идёт по DOM и вызывает `test.fixme(...)` с измеренным числом, пока превышение держится;
как только UX-волна ужмёт путь до ≤12, тот же тест сам станет живым регрессионным
гейтом. Число в заголовке теста — замер волны 10 (2026-09-30).

Состав пути (факт, dev-режим): DevPanel summary → nav(3) → тема(3) → язык интерфейса(2)
→ switcher(2) → закреплённый язык uk(1) → CTA(1) → «Закрыть проект»(1) → вкладки.

## Ограничения

- **Mock-транспорт.** Всё крутится на демо-данных `MockTransport`; реальные IPC,
  persistence, файловые эффекты и родные диалоги — **T2b** (WebDriver/AX-канал или
  спайк `tauri-webdriver-automation`, см. архитектурный док) + T0/T3. Буст-гейт
  «prod-сборка без моста = честная ошибка конфигурации» здесь не проверяется: на
  dev-сервере mock — явный дефолт; проверка «none»-ветки требует preview-сборки (T2b).
- Подсветка якоря коуча допускает перелив за нижний край **только на собственный
  пэддинг** (6px продукта: anchor-rect + pad; таблица редактора легитимно упирается в
  низ viewport). Карточка тура — строго внутри viewport.
- Headless-шрифтовой рендер может отличаться от WKWebView — геометрию эти тесты берут
  из layout-боксов, а не из пикселей, так что класс инварианта это не меняет.
- Порт 5199 занят → `--strictPort` уронит прогон с внятной ошибкой; свободный порт для
  параллельных сессий меняется в `e2e/playwright.config.ts` (webServer + baseURL).
