# Lovable R1 — живой исходник vs текущий UI: аудит визуальной верности (2026-10)

Мандат: §2 Final Beta Readiness — глубокий визуальный аудит против **живого** оригинала.
Verdict-правило: **defect** — только реальные ухудшения относительно Lovable R1 (что было
лучше и стало хуже/потеряно); совпадения — здесь таблицей; «в Lovable по-другому» без
ухудшения — DIFFERENT-принято.

## Материал и метод

| Источник | Путь | Факт |
|---|---|---|
| Reference (живой код) | `~/Developing/RimLoc-reference/lovable-r1/RimLoc-Lovable-Source-b2c6fc38/` | `src/components/rimloc/{shell,editor,pages,dialogs,primitives,store}.tsx`, `src/routes/__root.tsx`, `src/styles.css`, `DESIGN_REFERENCE_MAP.md` — прочитаны полностью |
| Current | `gui/tauri-app/frontend-react/src/` | `App.tsx`, `components/*` (16), `styles/{r1.css,r1-react.css,tokens.css}`, `lib/routes.ts`, `lib/i18n/index.ts`, `main.tsx` — прочитаны |
| Скриншоты | `RimLoc-evidence/artifact-rel23-final/screens/` — 18 кадров (9 экранов × light/dark) | Прочитаны: 01-home, 02-new-project, 03-workspace, 04-checks, 05-glossary, 08-build (light), 03-workspace (dark) |

**Расхождение с заданием:** путь `RimLoc-evidence/artifact-rel24-candidate/screens/` **не
существует** — в `artifact-rel24-candidate` лежит только .app-пакет без скриншотов. Свежайший
полный сет 18 кадров — `artifact-rel23-final` (source commit `857b0d2a`, verified: предок HEAD
`9623b91`). Проверено `git diff 857b0d2a..HEAD -- .../styles/` — **стили не менялись** после
съёмки, кадры точно отражают текущий CSS; компонентный TSX с тех пор дрейфовал
(Workspace +363, BuildExport +259, ProjectSwitcher +146 и др.) — кадры по этим зонам могут
отставать от кода; выводы по ним сделаны по коду, кадры — корроборация каркаса и токенов.

Проверки, выполненные в этой сессии:
- диф класс-селекторов `styles.css` vs `r1.css` (python, извлечение `\.([a-z0-9_-]+)`) —
  **0 потерянных классов** (все канон-классы портированы; «css» — ложное совпадение от `@import`);
- grep использования канон-классов из TSX: `table-head` **0**, `glossary-toolbar` **0**,
  `archive-tree` **0**, `format-grid` **0**, `comparison-config` **0**, `terminal-output` **0**,
  `check-summary` **0**, `inline-info` **0**; используются: `status-label` (2), `file-drop`,
  `passed-state`, `empty-state`, `add-term`, `metrics-band`, `diff-row` (Existing),
  `tools-grid`/`tool-catalog` (ProvidersScreen), `page-note` (13);
- токены: `:root`/`.dark` OKLCH-набор r1.css побайтово равен референсу; шрифтовая тройка
  Golos Text / Literata / IBM Plex Mono забандлена офлайн (`main.tsx`, @fontsource 400/500/600);
  расхождение только сознательное — `--muted-foreground` затемнён до `.52`/`.72` (WCAG AA,
  комментарий в `r1-react.css:165-170`).

---

## 1. Shell (shell.tsx vs App.tsx + r1.css/r1-react.css)

Совпадает (канон держится): каркас `.rim-app/.app-sidebar 222px/.app-main`; brand Literata 27px
с винной точкой `.brand-period`; `.primary-nav` 39px-ряды с active-accent (тint + wine-цвет) и
красной пилюлей `nav-count` на «Проверки» из **живого** `error_count` (App.tsx:159-160,261-265);
sidebar-progress «Ваш перевод N%» + 4px progress + «N из M строк»; `.sidebar-bottom` с
Настройками и `.sidebar-foot` с dot; topbar 66px: breadcrumb (FolderOpen → проект → маршрут,
честные лейблы без fallback на «Настройки»), theme-toggle Sun/Moon, `.topbar-divider`;
`.workspace-heading` на workspace: eyebrow 8px «● ENGLISH → РУССКИЙ» + Literata h1 30px
«Слова становятся миром.» + подзаголовок; onboarding-strip (винная левая граница 2px,
`.step-number`); footer «Только переводы. Оригинальный мод в безопасности.»; off-canvas
mobile-nav на ≤700px (класс `mobile-nav-open` сохранён); reduced-motion ветка.

| Расхождение | Вердикт |
|---|---|
| **Иконки навигации переназначены и задублированы** — ref: Глоссарий=BookOpen, Сборка=Package, каждый пункт со своей иконкой; current (`lib/routes.ts:117-171`): Глоссарий=Package, Память переводов=Package, Сборка и экспорт=Wrench, Существующий перевод=GitCompareArrows (= Сравнение). Два куба и две «стрелки-сравнения» стоят соседями — иконный язык R1 (уникальная иконка у каждого пункта) деградировал | **DEFECT D2 (low)** |
| sidebar-progress перестилен: strong 20px Literata; потерян ref-линк «Выпустить перевод →» в блоке | DIFFERENT-принято (сильный жест, CTA дублируется nav; экспорт достижим) |
| demo-badge «ДЕМО-ПРОЕКТ» убран; в слоте — честная пилюля `.status-label` Saved/Unsaved/Saving/Error | DIFFERENT-принято (live-only доктрина; doc Shell: «+ dirty-индикатор из контракта») |
| layout-switch Фокус/Студия/Обзор отсутствует; вместо него react-resizable-panels с персистом | DIFFERENT-принято (doc: ADAPT «практическая ценность проверяется на representative screen»; решено панелями) |
| «Новый проект» в topbar — filled `.btn-primary` (ref — ghost sm) | DIFFERENT-принято (3 раунда критики зафиксировали размер/цвет; ясность CTA) |
| brand small «WORKSPACE» → «R1» | DIFFERENT-принято (лейбл лейна) |
| «О проекте» help-panel и GitHub-ссылка в футере отсутствуют (doc планировал замену на Sonner + честный busy) | DIFFERENT-принято с оговоркой: inline-фидбек покрывает save-state, но см. D5 |
| Скролл-модель: ref — страничный (min-height:100vh); current — app-shell 100vh с внутренним скроллом `.page-content` | DIFFERENT-принято (десктоп-приложение, footer всегда виден) |
| `.primary-nav a{font-size:11px;padding:0 8px}` в r1-react.css:54 жёстко применяет значение ref-медиа ≤1150px на всех ширинах | DIFFERENT-принято (плотность; заметно только рядом с каноном ≥1150px) |
| Onboarding-strip: 1 шаг вместо 3 с коннекторами | DIFFERENT-принято (шаги 2-3 требуют открытого проекта; strip скрывается с проектом — честный флоу) |

## 2. Editor (editor.tsx vs Workspace.tsx EntryEditor)

Совпадает: `.file-tree` 176px с `.pane-title` 49px, tree-root «Все строки N» (счётчик из живого
инвентаря, группы kind/def_type вместо хардкода — doc ADOPT+IMPROVE выполнен), `.tree-bottom`
(eyebrow ПАПКА ПЕРЕВОДА + `Languages/<folder>` + зелёное «Исходники не изменяются»);
`.entry-toolbar` (search-field + ListFilter reset); фильтры Все/Без перевода/С замечаниями
(замечания — из живого валидатора, §9); `.entry-row` 63px, selection = wine-тint + левая полоса,
`.untranslated` italic «Добавить перевод…»; `.list-footer` «N из M строк» + dot success
«Изменения в текущей сессии»; `.detail-pane`: `.detail-header` РЕДАКТОР СТРОКИ + prev/next,
`.detail-key` code + copy, `.label-line` с `.locale`-чипами, `.source-block` на тинте,
textarea с wine-бордером, `.char-count` (Сохранено/Есть несохранённые + N символов),
`.inline-warning` (первая находка живого валидатора — §17), `.detail-actions` «Сохранить и
дальше» (flex:1, 32px) + Save + RotateCcw revert, `.context-tabs` underline, `.detail-foot`
с FileCode2; тёмная тема selection — починена до полного соответствия канону (round-4/6).

| Расхождение | Вердикт |
|---|---|
| **Статус-колонка списка потеряна**: ref-ряд = source/target/Status (иконка+цвет: проверено/переведено/нет перевода/ошибка/изменено/не используется), плюс `.table-head` «ИСХОДНИК EN / ПЕРЕВОД RU / СТАТУС». Current: DOM-ряд двухколоночный (`Workspace.tsx:323-328`), `table-head` не рендерится (grep = 0), статус виден только косвенно (пустая цель / фильтр «С замечаниями»). Список перестал быть статус-обзором | **DEFECT D1 (medium)** |
| **line-clamp: 1** на source/target (`r1-react.css:204-206`) против канона clamp-2 (при тех же 63px) — длинные строки RimWorld обрезаются на первой строке, ряды полупустые; комментарий «canon density — line-clamp 1» неверно цитирует канон (в каноне clamp-2) | **DEFECT D6 (low)** |
| Копирование ключа безвозвратно молча: ref давал toast «Ключ скопирован»; current — `void navigator.clipboard.writeText(...)`, переменная `copied` мертва (`Workspace.tsx:370,642`) | **DEFECT D5 (low)** |
| Табы Контекст/Термины/XML → Источник/Термины; XML-превью убрано; взамен Source Inspector с живыми file:line/selected_by/version/patch/TKey-локациями | DIFFERENT-принято (doc: «XML-превью — осторожно; Source Inspector — PRODUCTION_ALREADY_BETTER») |
| Полоса `.translation-action` (Машинный перевод · N без перевода · Настроить) стала тихой inline-кнопкой в инспекторе, ведёт в чат-перевод | DIFFERENT-принято (doc: ADAPT «ведёт в реальный AI/no-API flow»); счётчик «N без перевода» остался во фильтре/футере |
| «Перевод проверен» checkbox и inline-info «Исходник изменён · Раньше» (демо-концепты) не перенесены | DIFFERENT-принято (reviewed/orphan-семантика заменена живым lifecycle/валидатором) |
| `.ws-footer` с «На главную» и `rev N · epoch N` | DIFFERENT-принято (честная сессионная метаданные; новая зона) |
| version-select RimWorld → select цели перевода с лейблом «→» | DIFFERENT-принято (живой эквивалент select-в-шапке; лейбл «→» — косметика, eyebrow уже несёт направление) |

## 3. Pages (pages.tsx + routes/* vs Home/Checks/Glossary/Tm/BuildExport/Compare/Settings)

Совпадает: `.page-content` 28px + `.section-heading` (eyebrow + Literata h2 24px 500 + подпись
+ действие справа) на всех экранах; копирайт канона сохранён дословно («ЛОКАЛЬНАЯ БИБЛИОТЕКА /
Ваши проекты», «КОНТРОЛЬ КАЧЕСТВА / Уверенность в каждой строке», «ОДИН МИР. ОДИН ЯЗЫК. /
Глоссарий проекта», «ОТ СТРОК — К ГОТОВОМУ МОДУ / Выпустить перевод», «РАБОЧАЯ СРЕДА»);
Checks: `.metrics-band` (31px Literata-цифры, error/warning цветность, hairline-вертикали) +
`.section-tabs` underline + «Отчёт» скачиванием JSON + `.finding-row` (kind+code+сообщение+
переход «Исправить» в редактор — §35) + `.passed-state` (45px ShieldCheck + serif h3 «Всё
чисто»); Glossary: `.glossary-table` (шапка 8px uppercase на `.muted`, строка термин→перевод,
icon-btn удаления с confirm-рядом — добавлен, лучше прототипа) + `.add-term` grid c wine-CTA;
Build: `.build-grid` (форма + `.build-preview` aside), `.safe-label` «Без изменения оригинала»,
`.build-actions`, живой preview-панелью статуса валидации и честным result-панель + reveal
папки (§8); Compare: `.metrics-band` 4 метрики + `.cmp-table` на chrome глоссаря с
cmp-source-line «было → стало»; Settings: `.settings-section` + field-селекты + честная
capability-таблица из handshake (text-success/text-warning); Home: `.project-grid` 3 колонки +
`.project-card` + dashed `.new-project-card` + `.project-storage` заметки + selfloc-карточка.
Пустые состояния — честные (`empty-state`, needProject-экраны с btn-primary «На главную»).

| Расхождение | Вердикт |
|---|---|
| **Glossary потерял `.glossary-toolbar`** (поиск по терминам + «Импорт JSON») и JSON-выгрузку из шапки — все три аффорданса ref отсутствуют (grep toolbar = 0). Живой глоссарий может быть длинным демо-в 18 строк | **DEFECT D3 (low)** |
| **`.archive-tree` мёртв**: signature-превью структуры мода (mono, line-height 2.4, indent-уровни) не рендерится нигде (grep = 0); BuildExport в idle показывает пустой aside «Пока ничего не собрано», planned structure — плоскими code-строками в preview-режиме (`BuildExport.tsx:359-370`). Правая колонка экрана сборки опустела | **DEFECT D4 (low)** |
| `.format-grid` (карточки PO/POT/XLIFF/CSV/JSON/XML) не перенесён — экспорт живёт одной командой контракта | DIFFERENT-принято (форматы диктует CLI; карточки возвращать вместе с опциями экспорта) |
| Compare: режимы source/version/sets, PO-загрузка и diff-ряды «БЫЛО/СТАЛО» с «Принять» заменены непроектным version-diff (две папки → таблица категорий) | DIFFERENT-принято (doc: «PRODUCTION_ALREADY_BETTER + ADAPT»; Existing несёт apply-флоу с `.diff-row`) |
| Project-карточки без обложки/прогресса/N-из-M (только метка, имя, ревизия, «Открыть →») | DIFFERENT-принято (в summary-DTO нет счётчиков; рисовать нечего — честно) |
| `.check-summary` полоса (CheckCheck «Проверены пустые…») не перенесена | DIFFERENT-принято (перечень проверок диктует живой валидатор) |
| Tools-экран (каталог 15 инструментов + terminal-output) удалён целиком (Route без 'tools', §16 INTENTIONALLY_REJECTED) | DIFFERENT-принято (документированный отказ; `tools-grid` переиспользован ProvidersScreen) |
| Settings: секции «Параметры проекта» и «Скачать предпочтения» убраны (audit v2 #3 — без персистенции и консьюмеров); layout-select убран вместе с лейаутами | DIFFERENT-принято (задокументировано в шапке Settings.tsx) |
| Wizard: 3 шага вместо 4 (нет category-picks и имени проекта — имя из папки; «Открыть пример» невозможен без демо); dialog-actions right-aligned вместо space-between | DIFFERENT-принято (§22 задокументирован; диалог сохранён: rim-dialog 530px, wizard-dots, file-drop, form-grid, success-emblem) |
| ProjectSwitcher заменил статичную карточку с обложкой на живой дропдаун недавних проектов | DIFFERENT-принято (doc: «карточка проекта → станет живой»; обложка — демо-ассет) |

## 4. Primitives/tokens (primitives.tsx + styles.css vs r1.css + r1-react.css)

Совпадает: OKLCH-палитра light/dark равна референсу (проверено дифом, единственное осознанное
отклонение — WCAG-затемнение muted-foreground, записано); `--source`/`--terminal`/
`--overlay`/`--shadow`/`--radius` роли на месте; типографская пара Golos Text (13px база) /
Literata display / IBM Plex Mono для code/pre; прогресс-бар 4px wine; hairline-ритм
(1px border везде, 49px панели, 63px ряды, 66px topbar); eyebrow 6-9px uppercase + dot-язык
(5px, primary/success/warning/destructive); радиусы 4-8px; `letter-spacing:0`; status-цветность
text-success/warning/destructive/info; `.spin` анимация; reduced-motion.

| Расхождение | Вердикт |
|---|---|
| `--muted-foreground` затемнён (.565→.52 light, .671→.72 dark) | DIFFERENT-принято (WCAG AA для 13px, задокументировано в CSS-комментарии) |
| Toast/busy-пилл из прототипа не заменены на Sonner (план doc) — фидбек инлайн | DIFFERENT-принято с оговоркой (см. D5 — единственное место, где фидбек потерян совсем) |
| r1-react.css добавляет ~340 строк мостов и round-фиксов, включая literal-oklch `!important` ядерные правила тёмной темы (round-6) | DIFFERENT-принято (техдолг зафиксирован в комментарии «Revisit if tokens.css ever redefines»); визуально кадры подтверждают корректную тёмную тему |

## 5. Dialogs (dialogs.tsx vs NewProjectWizard.tsx)

Совпадает: `.wizard-overlay` + `.rim-dialog` 530px/30px padding + shadow; eyebrow «НОВЫЙ
ПЕРЕВОД · N / 3» + Literata h2 24px + description; `.wizard-dots` (заполнение done до текущего
шага); step 0 `.file-drop` dashed с folder-иконкой; step 1 `.form-grid` (источник EN locked,
цель, версия 1.6/1.5/1.4); step 2 `.wizard-summary` с `.success-emblem` (52px круг) и mono-путь;
`.dialog-actions` с LoaderCircle-spin на busy; Escape-отмена с полным reset (лучше прототипа).

| Расхождение | Вердикт |
|---|---|
| 4 шага → 3 (category-picks и имя проекта удалены), «Открыть пример» отсутствует | DIFFERENT-принято (§22; сканирование реальное — таймер-демо невозможен) |

---

## Итог: дефекты (реальные ухудшения против R1)

| ID | Severity | Что потеряно/ухудшено | Где | Fix |
|---|---|---|---|---|
| **D1** | **medium** | Статус-колонка и `.table-head` списка строк: ref-ряд source/target/Status делал список обзором состояния; сейчас статус строки не виден в списке вообще | `Workspace.tsx:303-333` (EntryList), canon: `r1.css` `.table-head`, `.entry-list .entry-row` (3-колоночный грид уже в CSS — DOM не заполняется) | Вернуть третью колонку 22px с компактным `.status-label` (иконка 13px, title): статус из живого отчёта валидатора (finding → error/warning, пустая цель → «нет перевода», иначе «переведено»); на layout ≥900px добавить `.table-head`; не менять грид в CSS — он уже канонный |
| **D2** | low | Иконный язык навигации: задвоение соседних иконок (Глоссарий и Память переводов — Package; Существующий перевод и Сравнение — GitCompareArrows), канон-маппинг нарушен (Глоссарий был BookOpen, Сборка — Package) | `lib/routes.ts:117-171` | `glossary: BookOpen`, `export: Package`; для `tm` и `existing` взять свободные из канон-набора (например `Database`/`ArrowRightLeft`) — по иконке на пункт |
| **D3** | low | Глоссарий: нет поиска по терминам и импорта/экспорта JSON (в ref — toolbar с поиском + Import + JSON-download) | `Glossary.tsx` (весь), canon: `pages.tsx:69` | Вернуть `.glossary-toolbar` (класс в r1.css есть): клиентский фильтр по `term/translation` + download JSON списком + import с upsert через существующий `glossaryUpsert` |
| **D4** | low | `.archive-tree` — signature-превью структуры мода — мёртвый CSS; экран сборки в idle пустой, planned-структура — плоские строки | `BuildExport.tsx:288-393`; canon: `pages.tsx:74`, `r1.css` `.archive-tree/.indent*` | Рендерить planned-структуру классами `.archive-tree` + `.indent/.indent-2/.indent-3` (About/ → Languages/<folder>/ → DefInjected/Keyed) из preview-отчёта; показывать дерево в idle как честный «план сборки» |
| **D5** | low | Копирование ключа строки без фидбека (ref: toast «Ключ скопирован»); в Workspace осталась мёртвая `copied`-переменная | `Workspace.tsx:370,408-414,642` | Оживить `copied`: на успех писать в состояние и на 1.5с показывать Check вместо Copy в `.detail-key` (инлайн, без тоста), либо вернуть `.toast-message` |
| **D6** | low | line-clamp:1 на source/target при каноне clamp-2 (ряды 63px полупустые, длинные строки обрезаны); CSS-комментарий неверно называет это «canon» | `r1-react.css:202-206` | Убрать override clamp-1 (canon-правила r1.css уже дают clamp-2) **или** осознанно оставить и поправить комментарий на «deviation, не canon» |

## DIFFERENT-принятые (не дефекты) — сводный список

Demo-badge → честная save-state пилюля; layout-switch → resizable-панели; app-shell скролл;
muted-foreground WCAG; Tools-маршрут удалён (§16); wizard 3 шага; onboarding в 1 шаг;
project-карточки без обложки/прогресса; Compare как version-diff; Existing с apply-флоу;
format-grid отложен; check-summary не перенесён; Settings сокращён (audit v2 #3); help-panel/
toast/busy не заменены (инлайн-фидбек, кроме D5); sidebar-progress restyle + без CTA-линка;
topbar filled CTA; brand «R1»; футер без GitHub-ссылки; nav 11px/8px на всех ширинах;
version-select → select цели («→» лейбл); ws-footer rev/epoch; ProjectSwitcher без обложки;
Source Inspector вместо Контекст/XML; Машинный перевод — inline-кнопка; dead responsive-ветка
≥1500px для sidebar (перебита r1-react.css:63) — косметика.

## Ограничения аудита

- 18 кадров rel23-final сняты с коммита `857b0d2a`; компонентный TSX после них менялся — по
  Workspace/BuildExport/ProjectSwitcher вердикты поставлены по коду HEAD, кадры подтверждают
  только каркас/токены/стили (стили после съёмки не менялись — проверено git diff).
- Запуск приложения в этой сессии не выполнялся (аудит исходников + существующие кадры);
  интерактивные состояния (resize-панели, палитра, drag) не воспроизводились.
- Tm/ChatBatch/Providers/LanguageManager/Selfloc/Diagnostics — канон-chrome переиспользуют
  (glossary-table/tools-grid/section-heading, проверено grep'ом и чтением), детальная сверка
  с прототипом не проводилась: в прототипе этих экранов нет — сравнивать не с чем.
