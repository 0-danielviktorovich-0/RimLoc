# UI R1 CONTROLS AUDIT (§6, Controls lane)

- Дерево: `_rimloc-worktrees/ba-main` @ `b9c53b1` (ветка `main`), фронт `gui/tauri-app/frontend-react/src/`.
- Sources of truth: `docs/design/LOVABLE_R1_ADAPTATION.md`, `docs/design/RIMLOC_UI_R1_PRODUCT_CONTRACT.md`, текущий код. `UI_R1_CHECKPOINT.md` / `UI_R1_FINAL_REPORT.md` не использовались как truth (исторические).
- Метод: аудит чтением (мандат: worktree не нужен, код не менялся). Прочитаны все 14 компонентов, `lib/state/*`, `lib/client/*`, `lib/i18n`, `lib/languages/*`, `lib/palette.ts`, стили (`r1.css`, `r1-react.css`), `src-tauri/tauri.conf.json`, `src-tauri/src/contract_adapter.rs`, `src-tauri/src/main.rs` (grep), Svelte-бейзлайн `frontend-v2/src/lib/stores/theme.svelte.ts`, `settings.svelte.ts`, `i18n/store.svelte.ts`, `project.svelte.ts`.
- Что НЕ прогонялось: сборка и рантайм. Причина: `tauri.conf.json` (`build.frontendDist`) во ВСЕХ деревьях указывает на `../frontend-v2/dist` (Svelte) — React-лейн ещё не является бандлящимся UI, рантайм-проверка контролов React в упакованном приложении невозможна из этого состояния. Все выводы — из кода и обработчиков; там, где рантайм мог бы опровергнуть вывод (F9), это оговорено явно.

---

## Структурные факты

1. **React-лейн не бандлится.** `gui/tauri-app/src-tauri/tauri.conf.json`: `frontendDist: ../frontend-v2/dist`, `beforeBuildCommand: cd frontend-v2 && npm run build`. Ни одна из 33 worktree не бандлит `frontend-react`. Пока это так, все вердикты ниже — про исходник лейна, а не про упакованное приложение.
2. **Клиент-слой живой, без моков.** `lib/client/instance.ts:11-31`: нет Tauri-моста → конфигурационная ошибка на весь экран (`App.tsx:287-293`), тихий мок-фолбэк отсутствует. `transport.ts:61-195` — полная карта контракта; все команды зарегистрированы в бэкенд-адаптере (`contract_adapter.rs:39-89`), `pick_directory` — `main.rs:3932` (нативный диалог).
3. **Паттерн persist-before-ack соблюдён** в ядре: `project.ts:214-268` (commit → applyIntents → свежий snapshot), ChatBatch apply и Existing apply зовут `projectStore.adoptExternal()` (`ChatBatch.tsx:199`, `Existing.tsx:92`). Исключения — Glossary/Tm (finding F8).

## Инвентарь контролов (по экранам)

Формат: контрол — действие — наблюдаемый эффект — persistence — error state. Верди́ты честных контролов сведены в суммарный LIVE_PARITY (см. findings).

### Shell (App.tsx)
| Контрол | Действие | Эффект | Persistence | Error |
|---|---|---|---|---|
| Brand `#/home` | навигация | Home | — | — |
| Карточка проекта + ChevronDown (`App.tsx:220-228`) | **нет обработчика** | ничего | — | — → **F1** |
| Nav ×10 (`App.tsx:90-104,230-235`) | hash-роуты | живые экраны; «Строки перевода» → `#/home` = список проектов → **F5** | — | — |
| Sidebar-progress (`App.tsx:237-248`) | display | % из живого snapshot (непустые ≠todo / не-orphan) | — | — |
| Sidebar «Настройки» (`App.tsx:250-252`) | `#/settings` | Settings | — | — |
| Mobile menu btn (`App.tsx:262-268`) | toggle navOpen | off-canvas nav; на десктоп-широте `display:none` (`r1.css`) | нет | — |
| Theme toggle (`App.tsx:276-278`) | `setDark` → `html.dark` | тема переключается | **нет** (Svelte: `rimloc.theme`) → **F6** | — |
| «Новый проект» (`App.tsx:280-282`) | `#/projects` | Home с визард-картой | — | — |
| Target-локаль select (`App.tsx:312-326`) | `setTargetLocale` → remap entries | живой; пишет folder-форму во все мутирующие вызовы | нет — паритет с Svelte `$state('ru')` (`project.svelte.ts:58`) | — |
| Command palette (Cmd+K, `palette.ts`) | 14 nav-команд + open-project + target-N при открытом проекте | все action реальны (hash / setTargetLocale); курсор, Esc, стрелки, Enter, hover-синк — по контракту WDIO; состав без проекта зафиксирован | — | empty-state честный (`palette.empty`) |
| Fallback-роут `#/tools` | display | честный placeholder «React-лайна R1 живёт» | — | — |

### Workspace (Workspace.tsx)
| Контрол | Эффект | Вердикт-заметка |
|---|---|---|
| Tree toggle (`:81-84`) | сворачивает группы | работает; иконка не отражает состояние (косметика) |
| Tree-группы (`:85-100`) | kindFilter, реальный фильтр | честно |
| Поиск (`:110-118`) | фильтр по key/source/target | честно |
| «Сбросить фильтры» (`:119-128`) | сбрасывает search+filter, **не kindFilter** | → **F10** |
| Фильтры Все/Без перевода/С замечаниями (`:130-137`) | реальные предикаты, но «С замечаниями» — клиентская эвристика, не находки валидатора | → **F11** |
| Entry-строки (виртуализированные, `:192-241`) | select → редактор | честно; TanStack Virtual с первого дня |
| Resize-разделитель (`:150,75`) | react-resizable-panels | **персист в localStorage** (`rimloc-ws-panes`, `:26-29`) |
| Prev/Next (`:280-285`) | move по entries | честно |
| Copy key (`:291-297`) | `navigator.clipboard.writeText` — реальный вызов; `copied`-стейт мёртвый (`:266,413`), фидбэка нет | честно (заметка: мёртвый код) |
| Textarea draft (`:308-315`) | setDraft; dirty-семантика | черновики сессионные, commit = persist-before-ack |
| Сохранить и дальше / Сохранить (`:327-332`) | `projectStore.commit` → applyIntents → свежий snapshot; stale-ревизия = типированная ошибка, черновик сохраняется | честно |
| Revert (`:333-340`) | draft := target | честно |
| Context-tabs «Источник» (`:347-349`) | **кнопка без onClick, всегда active** | → **F12** |
| Source Inspector block (`:350-412`) | только контрактные данные; absent → `—` | честно (никаких синтетических значений) |
| ws.error (`:71-73`) | role=alert | честно |

### Home (Home.tsx)
Карточки проектов → `projectStore.open` → `#/workspace` (`App.tsx:365-375`) — живое открытие; raw id не рендерится (`shortId`). Визард-карта → диалог. Заметка: мета-строка карточки хардкодит «EN → RU» (`Home.tsx:49`), хотя мульти-таргет существует (наблюдение, не контрол).

### NewProjectWizard
Pick folder (нативный диалог, реальный) → шаг 2: source/target select **disabled** (честно видимый disabled; таргет прибит к ru — реестр не подключён, наблюдение) → версия select живой → «Создать» → `project_create` → workspace. Ошибка скана показывается. Cancel/Back работают. Мёртвый тернарник в Cancel-лейбле (`:143`) — код-смелл, не влияет.

### Checks (Checks.tsx)
Автозапуск `project_validate` по открытию; «Проверить снова» — реальный re-fetch; фильтры категорий реальные; «Отчёт» — blob-скачивание → **F9**; покрытие `строк без ошибок/всего` — честная метрика из живого отчёта; «Всё чисто» только при реальном пустом фильтре.

### Glossary (Glossary.tsx) и TM (Tm.tsx)
Полный CRUD по живому контракту (list/upsert/delete/import/lookup), confirm перед удалением, ошибки поверх. Дефект общего вида: `reload()` в обоих экранах перечитывает snapshot и **выбрасывает результат** (`Glossary.tsx:57-61`, `Tm.tsx:108-114`) → **F8**. Status-select в TM — реальный upsert (provenance→MANUAL задокументирован, `Tm.tsx:172-185`). Импорт JSON/CSV — реальный, DRAFT-семантика на бэкенде, результат честно показывается.

### ChatBatch (ChatBatch.tsx)
Чекбоксы → «Создать батч» (реальный, строгий) → экспорт промпта (реальный, гейты ревизии+хеша) → копирование (реальный clipboard + «Скопировано») → вставка ответа → строгий импорт (реальный) → превью → «Применить» (реальный gated apply + adoptExternal). Опрос статуса/устаревания — реальный (`:65-84`). «Сбросить батч на экране» честно назван локальным. Полный джорни живой.

### Existing (Existing.tsx)
Pick (нативный) → «Разобрать (dry-run)» (реальный read-only) → классификация (5 метрик) → «Применить переиспользуемое(N)» (реальный, N=0 → disabled; после — adoptExternal). Честно.

### Compare (Compare.tsx)
Два нативных пикера → «Сравнить» (реальный read-only diff) → метрики + таблица + честная нота усечения. Честно.

### BuildExport (BuildExport.tsx)
Каталог: input + нативный «Выбрать…» → Экспорт/Собрать мод — реальные контракты, перед сборкой живая валидация-гейт (`:51-54,65-70`), результат — файлы/перечитано/каталог. «Предпросмотр» → **F2**.

### Providers (ProvidersScreen.tsx)
Карточки → startEdit; «Новый провайдер» → форма; preset-select подтягивает шаблон; label/model/baseUrl/secret/local — форма; secret один раз уходит в OS Keychain на бэкенде (`upsert`), никогда не рендерится; «Сохранить»/«Удалить» (удалит ключ тоже, tooltip честный) — живые. «Проверить конфигурацию» → `contract_provider_instance_validate` — офлайн-валидация формы, **честно раскрыта трижды** (лейбл, результат «(проверка без сети)», нота «Сетевые вызовы этим экраном не выполняются») → LIVE_PARITY, см. F4. Статус «Готов/Не настроен» из `has_key||local` → **F3**. Реальный сетевой тест существует только в CLI (`crates/rimloc-cli/src/commands/provider_test.rs`) и в GUI-контракт не проброшен; потребительских операций провайдеров в контракте лейна тоже нет (transport.ts:150-165).

### Selfloc (Selfloc.tsx)
«Открыть каталог RimLoc UI» → `selfloc_catalog_dir` → `project_create` → workspace. Состояние «открыт» — по имени. Ошибка — показывается. Честно.

### Diagnostics (Diagnostics.tsx)
Pick + «Собрать пакет поддержки» → реальный `project_diagnose`; результат — operation_id/файлы/санитизировано/исключено. Честно.

### Settings (Settings.tsx)
Тема select → live (но не персистится → F6); язык UI select → мгновенный ререндер всего дерева (но не персистится → F7); capability-таблица — из handshake, LIVE/«не входит в сборку» честно. Наблюдение: `.catch(() => undefined)` на handshake (`:29`) — при ошибке навсегда висит «Запрашиваю handshake…», error state не показан.

### LanguageManager (LanguageManager.tsx)
Поиск-фильтр, add (валидация кодами отказа → t()), inline-rename (U в CRUD), delete; всё в localStorage (`rimloc.languages.user.v1`, `manager.ts:14-29`) — переживает рестарт. Builtin-строки без кнопок. Честно.

---

## Findings

Полные вердикты — в структурированном результате; ниже сводка.

| # | Контрол | Fail-класс | Вердикт |
|---|---|---|---|
| F1 | Sidebar project card + ChevronDown | мёртвый аффорданс | IMPLEMENT_NOW |
| F2 | BuildExport «Предпросмотр» | текст вместо превью, в error-канале | IMPLEMENT_NOW |
| F3 | Providers статус «Готов» (has_key\|\|local) + Wifi | офлайн-конфигурация в семантике подключения | IMPLEMENT_NOW |
| F4 | Providers «Проверить конфигурацию» | — (подозрение мандата снято: офлайн раскрыт ×3) | LIVE_PARITY |
| F5 | Nav/palette «Строки перевода» → список проектов | label ≠ destination | IMPLEMENT_NOW |
| F6 | Тема (toggle + Settings) | persistence сломана (Svelte персистит) | IMPLEMENT_NOW |
| F7 | Язык UI (Settings) | persistence сломана (Svelte персистит) | IMPLEMENT_NOW |
| F8 | Glossary/Tm `reload()` — snapshot выбрасывается | fake-adopt: комментарий обещает, код нет | IMPLEMENT_NOW |
| F9 | Checks «Отчёт» (blob-скачивание) | без Tauri download-моста контрол не делает видимого | IMPLEMENT_NOW |
| F10 | Workspace «Сбросить фильтры» | частичное действие | IMPLEMENT_NOW |
| F11 | Workspace фильтр «С замечаниями» | эвристика вместо находок валидатора | IMPLEMENT_NOW |
| F12 | Workspace context-tab «Источник» | мёртвый таб-аффорданс | IMPLEMENT_NOW |
| — | Все остальные контролы (≈115) | — | LIVE_PARITY суммарно |

## Наблюдения (не findings)

- Карточка Home хардкодит «EN → RU» (`Home.tsx:49`).
- Wizard: source/target select прибиты к en/ru, disabled (`NewProjectWizard.tsx:102-110`) — честный disabled, но реестр языков к визарду не подключён.
- Settings: ошибка handshake не показывается — вечное «Запрашиваю handshake…» (`Settings.tsx:21-33`).
- Workspace copy-key: мёртвый `copied`-стейт, нет фидбэка (`Workspace.tsx:266,413`).
- Tree-toggle иконка не отражает свёрнутое состояние (`Workspace.tsx:81-84`).
- ADOPT-элементы шелла из LOVABLE_R1_ADAPTATION, которых в React-шее ещё нет: nav-count (красная пилюля ошибок на «Проверки»), onboarding-strip, dirty-индикатор в топбаре, layout-switch. Это backlog фаз, не нечестность существующих контролов.
- Мёртвый тернарник в wizard cancel (`NewProjectWizard.tsx:143`).
