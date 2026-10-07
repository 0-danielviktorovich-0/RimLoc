# UI R1 FINAL PARITY AUDIT (2026-10)

Аудитор: Parity-Auditor (независимая лайна). Дата: 2026-10-07.
Задача: сверить каждый пункт `docs/design/LOVABLE_R1_ADAPTATION.md` (Phase A)
и `docs/design/RIMLOC_UI_R1_PRODUCT_CONTRACT.md` с текущей реализацией
`gui/tauri-app/frontend-react/src/` и вынести финальный вердикт по четырёхбалльной
шкале: LIVE_PARITY / LIVE_SUPERIOR / INTENTIONALLY_REJECTED / IMPLEMENT_NOW.
«Almost» запрещены. UI_R1_CHECKPOINT.md и UI_R1_FINAL_REPORT.md — исторические,
истиной не являются.

## Sources of truth и база аудита

- Замысел: `LOVABLE_R1_ADAPTATION.md`, `RIMLOC_UI_R1_PRODUCT_CONTRACT.md` (обнось — в этом же каталоге).
- Код: `gui/tauri-app/frontend-react/src/`, worktree `ba-main`, ветка `main`, HEAD `b9c53b1`.
- Поведенческая база: `RimLoc-evidence/reviewer/final-wdio-summary.txt` —
  **59 passing / 0 failing** (rel23-final, binary `7781dd39`, main `857b0d2/24d1d1f`,
  2026-10-07): palette-acceptance 34, lm-acceptance 15, chatbatch-acceptance 4,
  source-inspector 5, react-multitarget 1.
  Проверено `git merge-base --is-ancestor 24d1d1f HEAD` → YES и
  `git log 24d1d1f..HEAD -- gui/tauri-app/frontend-react` → пусто:
  **код, который читал аудитор, — тот же код, на котором снят пакет 59/59.**
- Пробы `react-smoke` (J1/J3/checks/glossary/J2), `react-a11y` 8/8,
  `react-geometry` 25/25, `react-stress-baseline.log` (20k строк) — из более
  ранних прогонов кампании (зафиксированы в FINAL_REPORT); в финальный пакет
  59/59 не входят. Помечены в тексте как «поведенческое (историч.)».
- Спеки: `gui/tauri-app/frontend-v2/e2e/wdio-spike/*.spec.ts`,
  `gui/tauri-app/frontend-v2/e2e/invariants/*.spec.ts`.

**Важно про сборку (deployment-факт, не вердикт пунктов):** дефолтный
`src-tauri/tauri.conf.json` всё ещё собирает Svelte-fallback (`frontend-v2/dist`,
`tauri.conf.json:7-9`); React-лайна R1 собирается через `tauri.react.conf.json`
(`frontendDist: ../frontend-react/dist`). Это осознанное состояние до
owner-переключения (зафиксировано в FINAL_REPORT, §3/§48). Все вердикты ниже —
про React-лайну R1 как таковую.

---

## СВОДКА ВЕРДИКТОВ ПО ЗОНАМ

| Зона / пункт | Исходный вердикт | Финальный вердикт |
|---|---|---|
| Shell: общая композиция | ADOPT | **LIVE_PARITY** |
| Shell: sidebar brand + nav | ADOPT | **LIVE_PARITY** |
| Shell: sidebar карточка проекта (обложка/шеврон) | ADOPT | **IMPLEMENT_NOW** (F6) |
| Shell: sidebar-progress | ADOPT | **LIVE_PARITY** |
| Shell: nav-count (красная пилюля на «Проверки») | ADOPT | **IMPLEMENT_NOW** (F3) |
| Shell: topbar dirty-индикатор | ADAPT | **IMPLEMENT_NOW** (F2) |
| Shell: breadcrumb для routes вне NAV | ADAPT | **IMPLEMENT_NOW** (F4) |
| Shell: workspace-heading (eyebrow/h1/version-select) | ADAPT | **LIVE_PARITY** (version-select — IR, обоснован в коде) |
| Shell: layout-switch Фокус/Студия/Обзор | ADAPT | **IMPLEMENT_NOW** (F1) |
| Shell: onboarding-strip | ADOPT | **IMPLEMENT_NOW** (F14) |
| Shell: workspace-summary (микростатусы) | ADOPT | **LIVE_PARITY** |
| Shell: footer-гарантия | ADOPT | **LIVE_PARITY** |
| Shell: help-panel/toast/busy | ADAPT | **IMPLEMENT_NOW** (F15) |
| Editor: 3-панельный грид resizable+persist | ADOPT+IMPROVE | **LIVE_PARITY** |
| Editor: file-tree из реального инвентаря | ADOPT+IMPROVE | **LIVE_PARITY** |
| Editor: entry-row 63px + TanStack Virtual | ADOPT+IMPROVE | **LIVE_PARITY** |
| Editor: фильтры (предикаты из validate) | ADOPT | **IMPLEMENT_NOW** (F11) |
| Editor: EntryEditor dirty/acked, save-and-next | ADOPT+IMPROVE | **LIVE_PARITY** |
| Editor: inline-warning (первая находка) | ADOPT | **IMPLEMENT_NOW** (F9) |
| Editor: Context → Source Inspector | ADAPT | **LIVE_SUPERIOR** (F7a) |
| Editor: табы Термины/XML + связанные строки | ADAPT/IMPROVE | **IMPLEMENT_NOW** (F7b) |
| Editor: «машинный перевод»-вход из редактора | ADAPT | **IMPLEMENT_NOW** (F10) |
| Home: §21 New/Open/Recent/Help-translate | ADOPT+IMPROVE | **IMPLEMENT_NOW** (F17) |
| Checks: metrics/findings/passed + «Исправить» | ADOPT | **IMPLEMENT_NOW** (F18) |
| Compare/Existing: живой diff-язык | PROD_BETTER+ADAPT | **LIVE_SUPERIOR** (F22) |
| Glossary: живой CRUD | PROD_BETTER+ADAPT | **LIVE_PARITY** |
| Glossary: импорт JSON | ADAPT (часть формы) | **IMPLEMENT_NOW** (F20) |
| Export/Build: живое наполнение (gate+reparse) | ADOPT+IMPROVE | **LIVE_SUPERIOR** (F21) |
| Export: archive-tree превью | ADOPT+IMPROVE | **IMPLEMENT_NOW** (F19) |
| Tools: каталог инструментов | ADOPT+IMPROVE | **INTENTIONALLY_REJECTED** (F5) |
| Settings: appearance + capability-таблица | ADAPT | **LIVE_PARITY** («Параметры проекта» — IR, обоснован в коде) |
| Wizard: джорни §22 | ADOPT+IMPROVE | **IMPLEMENT_NOW** (F16) |
| TranslateDialog → AI/no-API flow | ADAPT | **LIVE_SUPERIOR** (F23) |
| Визсистема: OKLCH-токены | ADOPT | **LIVE_PARITY** |
| Визсистема: light/dark | ADOPT | **LIVE_PARITY** |
| Визсистема: типографика (офлайн-бандл) | ADOPT+IMPROVE | **LIVE_PARITY** |
| Визсистема: eyebrow/dot-язык | ADOPT | **LIVE_PARITY** |
| Визсистема: плотность/hairline | ADOPT | **LIVE_PARITY** |
| Визсистема: радиусы/тень/letter-spacing | ADOPT | **LIVE_PARITY** |
| Визсистема: responsive/reduced-motion | ADAPT | **LIVE_PARITY** |
| Визсистема: старый оранжевый Svelte | REJECT | **LIVE_PARITY** (заморожен fallback'ом; дефолт сборки — owner-решение, см. выше) |
| Прототип-логика: store/core/demo/TanStack Start/хардкоды | REJECT ×5 | **LIVE_PARITY** ×5 |
| Факт-чек §12: виртуализация | IMPROVE-цель | **LIVE_PARITY** |
| Факт-чек §12: resizable-панели | IMPROVE-цель | **LIVE_PARITY** |
| Факт-чек §12: keyboard-first CAT-переходы (§45) | IMPROVE-цель | **IMPLEMENT_NOW** (F12) |
| Факт-чек §12: inline token highlighting (§29) | IMPROVE-цель | **IMPLEMENT_NOW** (F13) |

Итог по 50 строкам таблицы: **27 LIVE_PARITY · 4 LIVE_SUPERIOR · 1 INTENTIONALLY_REJECTED (Tools; ещё version-select и «Параметры проекта» Settings — локальные IR, обоснованные в коде) · 18 IMPLEMENT_NOW.**

---

## FINDINGS (не-PARITY)

Формат: ZONE / ORIGINAL VERDICT / CURRENT / EXPECTED / BEHAVIORAL EVIDENCE /
FINAL VERDICT / FIX. Пометка «[стат]» = evidence только статический (чтение
кода); «[дин]» = поведенческий (WDIO-проба/лог); «[дин-ист]» = поведенческий из
более ранних прогонов.

### F1 · shell / §12 — Layout-switch (Фокус/Студия/Обзор)
- ORIGINAL: ADAPT — «CSS-grid-трюк переносится; практическая ценность проверяется на representative screen (§25)».
- CURRENT [стат]: CSS канон перенесён полностью — `.layout-switch`, `.layout-focus`, `.layout-console` присутствуют в `r1.css` (минифицированный блок строки 9/12), но в JSX не монтируются: `grep -rn "layout-switch|layoutSwitch" src --include=*.tsx --include=*.ts` → 0; `grep "Фокус|Студи|Обзор" src/lib/i18n/index.ts` → 0. В `workspace-heading` (App.tsx:307-327) только селектор цели; комментарий App.tsx:308-311 подтверждает, что из controls удалён только version-select, layout-switch не упоминается вовсе.
- EXPECTED: переключатель Фокус/Студия/Обзор в heading-controls, класс режима на гриде воркспейса, персист выбора.
- BEHAVIORAL [дин]: ни одна проба финального пакета 59/59 не щупает layout-switch (grep по `wdio-spike/*.spec.ts` — 0 совпадений): проба его отсутствие и не опровергает, и не покрывает.
- FINAL: **IMPLEMENT_NOW**.
- FIX: смонтировать `.layout-switch` в `workspace-heading` (App.tsx), состояние режима (focus/studio/overview) в App/store, класс `layout-focus`/`layout-console` на контейнер воркспейса, персист в localStorage (по образцу `useDefaultLayout` в Workspace.tsx:26-29); и18н-лейблы трёх режимов.

### F2 · shell / §11 — Shell-level dirty status
- ORIGINAL: ADAPT — topbar «+ состояние сессии/сохранения (dirty-индикатор из контракта)».
- CURRENT [стат]: контракт отдаёт `dirty` в снапшоте (`lib/client/types.ts:195-197`), но в шелле не показывается: топбар (App.tsx:261-284) содержит breadcrumb/theme/«Новый проект» и ничего о сессии. Dirty виден только внутри воркспейса: ws-footer `Workspace.tsx:183-186` (`snap.dirty ? ' · ' + t('ws.dirty')`) и char-count редактора (`Workspace.tsx:317`).
- EXPECTED: индикатор несохранённых правок сессии в topbar, видимый с любого маршрута.
- BEHAVIORAL [дин]: react-smoke J3 доказывает, что dirty-семантика живая (commit растит ревизию); проб на видимость dirty вне воркспейса нет.
- FINAL: **IMPLEMENT_NOW**.
- FIX: в `app-topbar` (App.tsx) вывести точку/лейбл из `st.snapshot?.dirty` плюс счётчик `Object.keys(st.drafts).length`; клик ведёт на `#/workspace`.

### F3 · shell / §10 — nav-count: красная пилюля ошибок на «Проверки»
- ORIGINAL: ADOPT — «счётчик из project_validate».
- CURRENT [стат]: стиль `.nav-count` ( destructive-пилюля) перенесён в `r1.css`, но не используется: рендер NAV (App.tsx:229-236) не создаёт бейдж; валидация вызывается только на экране Checks (Checks.tsx:23-50), счётчик нигде не живёт в шелле.
- EXPECTED: на nav-пункте «Проверки» — число ошибок последнего validate.
- BEHAVIORAL [дин]: palette-acceptance 34-й состав и rel22-кадры фиксируют NAV без бейджа — его отсутствие сейчас «закреплено» пробами как текущее состояние.
- FINAL: **IMPLEMENT_NOW**.
- FIX: держать последний `ValidateProjectResponseDto.error_count` в project-сторе (заполнять при open/commit/на Checks-ране) и рендерить `<span className="nav-count">{n}</span>` в nav-элементе `checks` при n>0; тесты обновить (сейчас они пинят состав без бейджа).

### F4 · shell / §18 — Breadcrumb для маршрутов вне NAV
- ORIGINAL: ADAPT — breadcrumb 10px в топбаре.
- CURRENT [стат]: App.tsx:269-274: `NAV.find((n) => n.to === route)?.label ?? t('nav.settings')`. Маршруты вне NAV — `selfloc`, `diagnostics`, `providers`, `tools` — получают ярлык «Настройки» (`nav.settings`, i18n/index.ts:16). `workspace` спец-кейс верен («Строки перевода»), `settings` совпадает с fallback'ом случайно.
- EXPECTED: корректное имя раздела в хлебной крошке для каждого маршрута.
- BEHAVIORAL [дин]: rel22-кадры покрывают только 9 маршрутов (rel22-screens.spec.ts:19-27), среди них нет selfloc/diagnostics/providers — проба не ловит неверный ярлык; дефект виден статически.
- FINAL: **IMPLEMENT_NOW** (дефект честности: ложный ярлык раздела).
- FIX: полная карта route→label (переиспользовать `palette.cmd.*`-ключи или завести `nav.*` для selfloc/diagnostics/providers) в App.tsx:273 вместо `?? t('nav.settings')`.

### F5 · shell / §16 — Tools route: каталог 12 инструментов
- ORIGINAL: ADOPT+IMPROVE — «отличная форма для продвинутых CLI-возможностей; terminal-output — реальные stdout-мосты где есть, иначе честный UNSUPPORTED».
- CURRENT [стат]: tools **осознанно убран**: комментарий App.tsx:102-103 — «tools удалён из навигации (W0): не было продуктового определения. Route 'tools' жив — внешний hash #/tools честно падает в fallback». `currentRoute()` знает `tools` (App.tsx:108), рендер уходит в generic-fallback «React-лайна R1 живёт…» (App.tsx:376-392) — без фейкового каталога. Экран Tools не написан.
- EXPECTED (по адаптации): экран-каталог инструментов с честным UNSUPPORTED.
- BEHAVIORAL [дин]: palette-acceptance.spec.ts:305 — тест состава палитры озаглавлен «все 14 маршрутных команд (**tools удалён**, +чат-перевод)» и проходит 34/34: отсутствие tools закреплено поведенчески как решение.
- FINAL: **INTENTIONALLY_REJECTED** (обоснование в коде есть; в доке дизайн-канона решение не отражено).
- FIX (док): пометить строку Tools в `LOVABLE_R1_ADAPTATION.md` решением «REJECTED-W0 — нет продуктового определения (App.tsx:102, palette-acceptance §состав)», чтобы Phase A-док не расходился с живой реализацией; возвращение Tools — отдельный продуктовый мандат.

### F6 · shell / §19 — Мёртвая аффорданс: карточка проекта сайдбара (ChevronDown)
- ORIGINAL: ADOPT — «карточка проекта → станет живой (реальный проект/прогресс)».
- CURRENT [стат]: App.tsx:220-228 — `.sidebar-project` показывает живой label (`projectLabel()` из snapshot/summaries, App.tsx:172-178) и статус-точку, но это `div` без onClick; `<ChevronDown size={14}/>` (App.tsx:227) изображает раскрытие меню, которого не существует. CSS (`r1.css` `.sidebar-project`) — статичная карточка: ни cursor, ни hover.
- EXPECTED: карточка либо интерактивна (меню: открыть другой проект/закрыть), либо без шеврона.
- BEHAVIORAL [дин]: проб на клик по карточке нет; rel22-кадр 03-workspace фиксирует её внешний вид.
- FINAL: **IMPLEMENT_NOW**.
- FIX: убрать ChevronDown (минимальный честный вариант) либо повесить меню-переключатель проектов (список из `st.summaries` → `projectStore.open`). Заодно решить судьбу обложки (CSS `.sidebar-project img` перенесён, в JSX не используется).

### F7a · editor / §13 — Context-tabs: Контекст → Source Inspector
- ORIGINAL: ADAPT — «Контекст → Source Inspector (source_ref, provenance, reveal/open — PRODUCTION_ALREADY_BETTER)».
- CURRENT [стат]: правая панель редактора показывает живой SOURCE-блок: file/line (`src.file`, `src.line` — Workspace.tsx:350-358), why-this-source (`src.selected-by`:360-362), provenance версия/условная ветка/патч (`src.version`/`src.conditional`/`src.patch`:363-380), TKey primary + прочие вхождения (`src.tkey*`:381-405), корень мода (`src.root`:406-411); отсутствие данных рендерится честным «—».
- EXPECTED: замена прототипного «Контекст»-таба инспектором реального источника — не эмуляция.
- BEHAVIORAL [дин]: **source-inspector 5/5 passing** (final-wdio-summary.txt:7): src.file = реальный путь Languages/English/Keyed/Fixture.xml, src.line = parser-guaranteed, selected-by локализован, src.root живой, честное отсутствие для Keyed-строки (source-inspector.spec.ts:97-137).
- FINAL: **LIVE_SUPERIOR** (реализовано глубже задуманного: provenance/TKey/patch-stage — сверх прототипа и сверх строки адаптации).

### F7b · editor / §13 — Табы «Термины»/«XML» + связанные строки (label ↔ description)
- ORIGINAL: ADAPT («XML-превью — осторожно, не эмуляция семантики») + IMPROVE «Связанные строки — related по canonical identity (defName-группа)».
- CURRENT [стат]: таб-полоса вырождена в одну неинтерактивную кнопку «Источник» (Workspace.tsx:347-349, единственный `context-tabs button.active` без соседей). Термины у редактора не показываются, хотя glossary и TM — LIVE-домены (Glossary.tsx, Tm.tsx): ни glossary-hit, ни tm-lookup не подтягиваются к выбранной строке. XML-превью отсутствует (CSS-заготовка `.xml-context` в r1.css не используется). Связанные строки label↔description по defName-группе не реализованы (класс `.related-string` навешан на TKey-блок — это другое: места сериализации ключа).
- EXPECTED: термины глоссария/памяти у редактора + связанные строки по identity; XML — только если семантически честно.
- BEHAVIORAL [дин]: проб на «Термины»-блок нет; source-inspector 5/5 покрывает только SOURCE-блок.
- FINAL: **IMPLEMENT_NOW**.
- FIX: в EntryEditor — секция «Термины»: локальный lookup по source-тексту через уже живые `glossaryList`+`tmLookup` (клиентские методы есть, client.ts); связанные строки — группировка entries по `identity.defName`-основе (реестр уже группирует tree по kind/def_type — Workspace.tsx:31-38); кнопку-заглушку «Источник» либо сделать честным заголовком блока (не button), либо убрать. XML-превью оставить отклонённым, пока не появится семантический рендер (записать решение в доке).

### F9 · editor / §17 — Inline-warning: первая находка валидатора в редакторе
- ORIGINAL: ADOPT — «findings из реального validate-контракта».
- CURRENT [стат]: в редакторе единственное предупреждение — клиентская проверка пустого черновика (Workspace.tsx:320-325, `ws.emptyWarning`). Находки `project_validate` в редакторе не показываются (валидация — только экран Checks, Checks.tsx:23-50).
- EXPECTED: для выбранной строки — первая находка валидатора инлайн.
- BEHAVIORAL [дин]: react-smoke J4/Checks-проба доказывает живость валидатора на экране Checks; в редакторе проб нет.
- FINAL: **IMPLEMENT_NOW**.
- FIX: хранить последний validate-отчёт в project-сторе (см. F3 — один источник), в EntryEditor фильтровать findings по `id.key === entryKey` и рендерить первую как `.inline-warning`.

### F10 · editor / §20 — Полоса «Машинный перевод» → вход в реальный AI/no-API flow
- ORIGINAL: ADAPT — «ведёт в реальный AI/no-API flow (chatbatch), не демо-провайдер».
- CURRENT [стат]: сам flow LIVE — маршрут ChatBatch (App.tsx:361-362, nav + палитра), чекбоксы → батч → экспорт промпта → импорт → apply (ChatBatch.tsx). Но из редактора входа нет: в EntryEditor/детейле нет ни полосы, ни кнопки «перевести через чат» (Workspace.tsx:275-419).
- EXPECTED: аффорданс у редактора, ведущий в chatbatch с текущей строкой/выбором.
- BEHAVIORAL [дин]: chatbatch-acceptance **4/4** (final-wdio-summary.txt:6) — полный UI-цикл; входа из редактора пробы не щупают (его нет).
- FINAL: **IMPLEMENT_NOW** (домен жив, отсутствует только вход).
- FIX: в `detail-actions`/`detail-foot` кнопка «Чат-перевод», ведущая на `#/chatbatch` (передать key через query/hash или общую селекцию).

### F11 · editor / фильтры — предикат «С замечаниями» — клиентская эвристика
- ORIGINAL: ADOPT — «предикаты — из реального lifecycle/validate».
- CURRENT [стат]: Workspace.tsx:49: `filter === 'issues'` → `completeness === 'todo' || (в target нет '{' при '{' в source)`. Первая половина — реальное lifecycle-поле; вторая — клиентская эвристика плейсхолдеров, дублирующая домен валидатора (§6: UI не дублирует домен) и способная расходиться с project_validate.
- EXPECTED: фильтр «с замечаниями» согласован с валидатором.
- BEHAVIORAL [дин]: проб на согласованность фильтра с validate нет.
- FINAL: **IMPLEMENT_NOW**.
- FIX: переиспользовать validate-отчёт из стора (F3/F9) для предиката `issues` (entry ∈ findings), эвристику с `{}` убрать.

### F12 · workspace / §45 — Keyboard-first core loop в редакторе
- ORIGINAL: контракт («keyboard-first core loop») + факт-чек §12 («keyboard-first CAT-переходы — НЕТ… обязательные production-улучшения»).
- CURRENT [стат]: палитра keyboard-first LIVE (Cmd+K/Esc/стрелки/Home/End/Enter — palette.ts:91-127). В редакторе переходы — только кнопки prev/next (Workspace.tsx:280-285); хоткеев «сохранить и дальше»/«следующая» нет (`grep keydown` в Workspace.tsx — 0).
- EXPECTED: клавиатурный путь select→edit→confirm→next без мыши.
- BEHAVIORAL [дин]: palette-acceptance 34/34 закрывает клавиатуру палитры; клавиатура редактора пробами не покрыта.
- FINAL: **IMPLEMENT_NOW**.
- FIX: хоткеи в EntryEditor (напр. Ctrl/Cmd+Enter — save-and-next, Ctrl+ArrowDown/Up — prev/next) с проверкой фокуса в textarea; добавить WDIO-пробу клавиатурного джорни.

### F13 · workspace / §29 — Inline token highlighting
- ORIGINAL: факт-чек §12 — «inline token highlighting — НЕТ… обязательные production-улучшения (§29)».
- CURRENT [стат]: подсветки плейсхолдеров `{0}`/`{{token}}` нет ни в source-block, ни в textarea (Workspace.tsx:303, 308-315); grep token/highlight по src — 0 релевантных.
- EXPECTED: подсветка токенов в source/черновике (риск рассинхрона плейсхолдеров виден глазом).
- BEHAVIORAL [дин]: проб нет.
- FINAL: **IMPLEMENT_NOW**.
- FIX: рендер source-block с обёрткой токенов (read-only, дёшево); для textarea — оверлей-подсветка или индикатор несоответствия числа токенов source/target (согласовать с F11/F9 — один validate-отчёт).

### F14 · shell — Onboarding-strip (3 шага)
- ORIGINAL: ADOPT — «плюс существующий route-scoped coach (§43: strip + coach marks + demo)».
- CURRENT [стат]: `.onboarding-strip`/`.onboarding-steps`/`.step-number` стили перенесены (r1.css), в JSX не монтируются (grep coach/onboarding по src — 0). Coach marks тоже отсутствуют.
- EXPECTED: полоса первых шагов (открыть мод → перевести → собрать) с винной левой границей.
- BEHAVIORAL [дин]: проб нет.
- FINAL: **IMPLEMENT_NOW**.
- FIX: смонтировать strip на Home/Workspace (шаги: wizard → первая правка → build), состояние шагов из живых данных (есть snapshot/revision/факт build).

### F15 · shell — Help-panel, toast, busy
- ORIGINAL: ADAPT — «заменить на Sonner + честный busy из реальных операций».
- CURRENT [стат]: busy — честный (флаги `busy` реальных операций гасят кнопки, LoaderCircle в wizard — NewProjectWizard.tsx:152; Workspace.tsx:327-340). Ошибки — инлайн `role="alert"` (.inline-warning на каждом экране). Sonner/тостов нет (grep sonner/toast — 0), help-panel нет. Замена тостов инлайн-паттерном нигде не зафиксирована как решение.
- EXPECTED: либо Sonner+честный busy, либо задокументированная альтернатива.
- BEHAVIORAL [дин]: честность busy покрыта поведенчески (chatbatch 4/4, palette 34/34 — состояния кнопок в джорни); тостов нет ни в одной пробе.
- FINAL: **IMPLEMENT_NOW** (по формальному пункту Sonner не выполнен; по сути inline-alerts состоятельны, но решение не записано).
- FIX: минимальный честный путь — записать решение «inline-alerts вместо тостов» в доку дизайн-канона (тогда пункт перейдёт в INTENTIONALLY_REJECTED для тостов); help-panel — отложить осознанным решением (сейчас её отсутствие нигде не отмечено).

### F16 · wizard / §22 — Джорни создания: язык цели, категории, «Открыть пример»
- ORIGINAL: ADOPT+IMPROVE — «источник → скан → язык → обзор → создание; категории = реальные EntryKind; „Открыть пример" = demo-проект».
- CURRENT [стат]: 3 шага; скан-шаг объединён с create обоснованно и честно — «The Rust scan IS the preview… no fake scanning timers» (NewProjectWizard.tsx:2-4) — это СВЕРХ прототипа. НО: (а) выбор языка цели захардкожен и заблокирован — `<select defaultValue="ru" disabled>` (NewProjectWizard.tsx:106-110), источник — `disabled` `en` (102-105); стора проекта `targetLocale` по умолчанию 'ru' (project.ts:69), т.е. проект фактически всегда создаётся на русский, переключение цели — только после создания в воркспейсе; (б) summary печатает «English → Русский» литералом (129); (в) category-picks (EntryKind) отсутствуют; (г) «Открыть пример» (demo-проект) отсутствует — демо-ветки в React-лайне нет вовсе.
- EXPECTED: выбор цели из реестра (9 builtin + пользовательские LM) на шаге языка; категории как Advanced; демо-проект с честным бейджем.
- BEHAVIORAL [дин]: source-inspector/palette пробы создают проект именно через этот wizard (например source-inspector.spec.ts:61-74) — путь «папка→1.6→создать» поведенчески живой; шаг языка пробой не проверяется (он disabled).
- FINAL: **IMPLEMENT_NOW**.
- FIX: включить select цели из `BUILTIN_LANGUAGES`+`loadUserLanguages()` (компоненты уже есть — App.tsx:126-133), писать выбор в projectStore перед create; убрать литерал «English → Русский» из summary (брать из реестра); категории — опциональный Advanced-шаг; «Открыть пример» — отдельное решение (демо-мод с пометкой DEMO по §42), при отказе — записать в доку.

### F17 · home / §21 — Home: Help-translate RimLoc + хардкод «EN → RU»
- ORIGINAL: ADOPT+IMPROVE — «Home = §21 (New/Open-existing/Recent/Help-translate)».
- CURRENT [стат]: New (wizard-карточка) и Open (карточки проектов, честные лейблы через `shortId` — Home.tsx:12-15) LIVE. «Help translate RimLoc» (selfloc) на Home нет и в NAV нет (App.tsx:90-104): маршрут `#/selfloc` LIVE (Selfloc.tsx — каталог открывается обычным проектом), но достижим только через палитру (`palette.cmd.selfloc`, palette-acceptance:376+ доказывает команду). Recent = карточки (ок). Дополнительно: мета карточки хардкодит `· EN → RU` (Home.tsx:49) независимо от фактической цели проекта — против REJECT-строки «hardcoded RU… → Language Registry + adapter-awareness».
- EXPECTED: вход «Помочь перевести RimLoc» на Home/NAV; язык цели на карточке — из данных.
- BEHAVIORAL [дин]: palette-acceptance:376 «команда „Языки"… ведёт на #/lm» и джорни-тесты закрепляют палитру как единственный вход selfloc; карточек selfloc на Home пробы не щупают.
- FINAL: **IMPLEMENT_NOW**.
- FIX: добавить nav-пункт/кнопку Home на `#/selfloc` ( Selfloc уже LIVE); убрать литерал «EN → RU» из project-meta (показывать target_version + цель, когда она появится в summaries, иначе опускать).

### F18 · checks / §35 — «Исправить»: находки не открываются в редакторе
- ORIGINAL: ADOPT — «metrics-band + finding-list + „Исправить" + passed-state».
- CURRENT [стат]: metrics-band, фильтры категорий, отчёт JSON, passed-state — LIVE (Checks.tsx:112-171). Но строки находок некликабельны (Checks.tsx:154-163, div без обработчика), кнопки «Исправить» нет; при этом подсказка на экране ОБЕЩАЕТ обратное: `checks.fixHint` = «Находки открываются в редакторе — правки возвращаются в валидацию» (i18n/index.ts:269; renders Checks.tsx:173-176). Текст обещает несуществующее действие — дефект честности §42.
- EXPECTED: клик по находке → workspace с выбранной строкой.
- BEHAVIORAL [дин]: palette-acceptance:616 «Проверки открывает ЖИВОЙ экран валидации» и react-smoke «checks: live validator» (историч.) закрывают данные, но не аффорданс исправления.
- FINAL: **IMPLEMENT_NOW**.
- FIX: `onClick` на `.finding-row` → `window.location.hash='#/workspace'` + `projectStore.select(f.id.key)`; если ключ непроектируем — переформулировать fixHint честно.

### F19 · export — archive-tree превью из реального результата сборки
- ORIGINAL: ADOPT+IMPROVE — «archive-tree — из реального результата сборки, не хардкод».
- CURRENT [стат]: результат сборки/экспорта показывается счётчиками files/reparsed + out dir (BuildExport.tsx:173-196); дерева файлов нет. Кнопка «Предпросмотр» честно отвечает «появится в следующей фазе» (BuildExport.tsx:161-168; i18n:236-237) — честный UNSUPPORTED, не эмуляция.
- EXPECTED: превью-структура архива из реального результата.
- BEHAVIORAL [дин]: t6-export-structural (историч.) и rel22-кадр 08-build закрывают сборку; дерева в пробах нет.
- FINAL: **IMPLEMENT_NOW** (или осознанный отказ с записью: контракт отдаёт счётчики, не список файлов).
- FIX: минимальный путь — вернуть из `project_build_mod` список записанных файлов (backing contract) и отрисовать дерево; если контракт расширять нельзя — записать осознанный отказ со ссылкой на честный счётчик.

### F20 · glossary — импорт JSON
- ORIGINAL: ADAPT (часть переносимой формы «таблица + импорт JSON + add-term»).
- CURRENT [стат]: таблица + add-term + delete с подтверждением LIVE (Glossary.tsx:124-187; поведенчески palette-acceptance:635 + react-smoke glossary CRUD, историч.). Импорта JSON нет (в TM импорт LIVE — Tm.tsx:213-225, `tmImport`).
- EXPECTED: массовый импорт терминов JSON.
- BEHAVIORAL [дин]: глоссарный CRUD покрыт пробами; импорт — нет.
- FINAL: **IMPLEMENT_NOW**.
- FIX: textarea-импорт по образцу TM: разобрать JSON на клиенте и цикл `glossaryUpsert` (метод уже есть), отчёт добавлено/пропущено.

### F21 · export/build — живое наполнение (ядро J6)
- ORIGINAL: ADOPT+IMPROVE — «наполнение — project_build_mod/export».
- CURRENT [стат]: нативный пикер каталога, validation-gate перед сборкой (BuildExport.tsx:64-70: `errors > 0` → отказ), build/export в активную цель в folder-форме (73, 97), результат — реальные files_written/reparsed/out_dir; никаких демо-путей.
- EXPECTED: домен из Rust-контракта вместо клиентского zip прототипа.
- BEHAVIORAL [дин]: rel22-кадр 08-build; palette-acceptance:642 «Сборка и экспорт открывает живой экран сборки»; t6-export-structural (историч.).
- FINAL: **LIVE_SUPERIOR** (gate + reparse-verified счётчики — сильнее прототипа; Composition 3-табов осознанно упрощена: импорт живёт отдельным маршрутом Existing — тот же домен import_existing).

### F22 · compare/existing — визуальный diff-язык на живых доменах
- ORIGINAL: PRODUCTION_ALREADY_BETTER + ADAPT — «живой Existing-flow уже делает классификацию и apply; визуальный diff-язык Lovable переносится на него».
- CURRENT [стат]: Existing — dry-run `import_existing` → метрики reusable/conflicts/new/obsolete/ambiguous → guarded apply переиспользуемого (conflicts не перезаписываются, persist-before-ack) — Existing.tsx:53-98,136-179; diff-список — первые 20 reusable (171-178). Плюс новый маршрут Compare — `version_diff` двух корней мода с категориями changed/new/missing/unchanged, цветными чипами и truncate-честностью (Compare.tsx:14-19,120-176) — сверх плана.
- EXPECTED: живая классификация+apply в визуальном языке Lovable.
- BEHAVIORAL [дин]: react-smoke «J2 existing: dry-run → apply reusable» (историч.); rel22-кадры 06-tm/07-chatbatch смежные; прямой пробы на Compare-маршрут в финальном пакете нет (покрыт статически + rel23 бинарка собрана с ним).
- FINAL: **LIVE_SUPERIOR** (построчное «Принять» прототипа не перенесено — apply пакетный по reusable; при желании донести как полировку, не как паритет-разрыв).

### F23 · translate/AI — TranslateDialog → no-API канон + провайдеры
- ORIGINAL: ADAPT — «реальный AI-стек: провайдеры из ProviderManager, no-API batches, „никогда не звать платное только потому что ключ есть" (§34)».
- CURRENT [стат]: ChatBatch — канонический no-API цикл (декларация в шапке файла; chatbatch-acceptance 4/4). Провайдеры — LIVE-конфиг поверх контракта: `provider_instance_list/upsert/delete/validate` (client.ts:318-346), ключ → OS keychain, экран не показывает ключ, только `has_key` (ProvidersScreen.tsx:1-6); валидация — честная офлайн-проверка формы. Платного вызова LLM из React-UI нет вовсе (в списке клиентских операций translate-by-provider отсутствует) — платное не зовётся не потому что «ключ есть», а потому что пути нет; это сильнее прототипного budget-слайдера.
- EXPECTED: реальный AI-стек без демо-провайдеров.
- BEHAVIORAL [дин]: chatbatch-acceptance 4/4 (final-wdio-summary.txt:6); rel22-кадр 07-chatbatch.
- FINAL: **LIVE_SUPERIOR**.

---

## PARITY-ИНВЕНТАРЬ (кратко, без включения в findings)

- **Shell-каркас**: sidebar 222px fixed (r1.css), topbar 66px, route-content, footer-гарантия (App.tsx:395-399) — канон воспроизведён; rel22-кадры 9 маршрутов × light+dark [дин].
- **Sidebar-progress**: живые счётчики из entries, orphan исключён (App.tsx:165-171, 237-248) [стат].
- **Грид воркспейса**: react-resizable-panels v4, `useDefaultLayout` persist, min 30/22% (Workspace.tsx:26-29, 75-179) [стат; geometry-пробы 25/25 — историч.].
- **File-tree**: группы `kind/def_type` с счётчиками из реального инвентаря + «Исходники не изменяются» (Workspace.tsx:31-38, 85-105) [стат].
- **Инвентарь**: TanStack Virtual, ROW_HEIGHT=63, overscan 12 (Workspace.tsx:16, 202-207); selection — wine-тинт + inset-полоса, dark-вариант литеральными oklch после раунда 6 (r1-react.css:70, 195-241) [стат; стресс 20k — react-stress-baseline.log, историч.].
- **EntryEditor**: persist-before-ack commit с expected_revision/epoch (project.ts:214-268), dirty-семантика, save-and-next, prev/next, char-count [стат; react-smoke J3 — историч.].
- **Glossary CRUD**: list/upsert case-insensitive/delete с подтверждением поверх project_glossary (Glossary.tsx) [стат; palette-acceptance:635 — дин].
- **Multi-target**: реестр 9 builtin + пользовательские LM в переключателе и в палитре (App.tsx:126-141, 316-326), re-map из того же снапшота (project.ts:202-207), folder-форма во всех записях (project.ts:220-228) [стат; react-multitarget 1/1 — дин].
- **LM**: CRUD пользовательских языков + поиск + inline-rename, builtin read-only (LanguageManager.tsx) [стат; **lm-acceptance 15/15** — дин].
- **Palette**: 14 маршрутов + экшены при открытом проекте, клавиатурная модель, сброс запроса, возврат фокуса (palette.ts) [стат; **palette-acceptance 34/34** — дин].
- **Selfloc**: каталог как обычный проект через createContractProject (Selfloc.tsx) [стат; палитра-вход — дин].
- **Diagnostics**: diagnose → safe bundle (Diagnostics.tsx) [стат].
- **Honesty §42**: mock не бандлится — без tauri-моста честная ошибка (instance.ts:19-30, client.ts:131); capability-таблица из handshake (Settings.tsx:63-81); «Предпросмотр» — честный UNSUPPORTED (i18n:237). Два отступления от честности пойманы и заведены в findings (F4 breadcrumb-ярлык, F18 fixHint-обещание).
- **Визсистема**: wine `oklch(.407 .123 15)` + полный набор ролей (sidebar/source/terminal/overlay) в tokens.css:1-3; типографика Golos/Literata/Plex Mono офлайн через @fontsource с кириллическими сабсетами в dist (main.tsx:4-20; dist/assets/*.woff2); eyebrow 8-9px, dot 5px, hairline-ритм, letter-spacing:0, радиусы 5-6px, prefers-reduced-motion (r1.css) [стат; light/dark — rel22 9×2, дин].
- **REJECT-строки соблюдены**: нет in-memory фейк-стора (контрактный store), нет клиентских парсеров/валидаторов/экспортёров, нет демо-провайдеров, нет TanStack Start (Vite SPA), нет хардкода языков в реестре (registry + LM) [стат].

## Рекомендованный порядок закрытия (для owner)

1. Честность (дешёво, высокой ценности): F4 (breadcrumb), F18 («Исправить»/fixHint), F6 (шеврон).
2. Сквозной validate-отчёт в сторе — разблокирует F3, F9, F11 одним движением.
3. Wizard: F16 (выбор цели — продуктовый пробел, не косметика).
4. Редактор: F7b (термины у редактора), F12 (хоткеи), F10 (вход в chatbatch), F13 (токены).
5. Записать осознанные решения: F5 (Tools — в доку), F15 (inline-alerts vs Sonner), F19 (archive-tree — делать или отказ), F1 (layout-switch — делать или отказ).
6. Полировка: F14 (onboarding), F17 (selfloc-вход), F20 (glossary import), F2 (shell dirty).
