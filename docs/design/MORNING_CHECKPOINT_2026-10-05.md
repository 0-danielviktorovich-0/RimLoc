# MORNING CHECKPOINT — RimLoc UI R1 (ночь 05/06.10)

Время: 07:20 JST 2026-10-05 (окно до 10:00 — работа продолжается ниже).
Автор: ZCode-сессия RimLoc UI R1. Всё локально, без push/tag/release.

## HEAD и лайны

- **RimLoc main**: `d0506fb` (Phase A docs) → кампания R1 в лайне
  **`glm/ui-r1-react`** (worktree `~/Developing/_rimloc-worktrees/wt-ui-r1`),
  HEAD `9cffee9`, 15 коммитов за ночь. main не тронут кроме Phase A-доков
  (d0506fb, c88b441..9cffee9 — см. ниже, это лайна).
- Вчерашняя кампания (soak/bridge) закрыта ранее: main несёт волны 13 + soak.

## Крупное завершённое (PROVEN)

1. **PHASE A** (d0506fb): архивы Lovable+skills распакованы и верифицированы
   (b2c6fc38, manifest 0 расхождений); CURRENT_SVELTE_BASELINE.md (frozen
   oracle, карта LIVE/PARTIAL/MOCK); LOVABLE_R1_ADAPTATION.md
   (ADOPT/ADAPT/IMPROVE/REJECT по всем зонам).
2. **PHASE B** (7ada3ba, bbf0e80): frontend-react бутстрап (React 19 + Vite 7
   + Tailwind 4 + TS strict); ВЕСЬ визуальный канон Lovable перенесён
   (tokens.css OKLCH + r1.css 41KB); framework-neutral RimLocClient с
   инъекцией транспорта; FRONTEND_UI_BOUNDARY.md + PRODUCT_CONTRACT.md.
3. **PHASE C representative** (86cdb33…0ec8e30): Home (живой project_list),
   Workspace 3 панели (file-tree из реального инвентаря + TanStack Virtual +
   редактор с persist-before-ack + инспектор source_ref), Checks (живой
   валидатор), Glossary (живой CRUD), Existing (dry-run→apply, J2),
   Build/Export (J6, validation-gate), Wizard (J1, реальный Rust-скан).
4. **Живой смоук 5/5** (wdio, реальный бэкенд, изолированные данные):
   wizard→create→инвентарь фикстуры · edit→commit→ревизия растёт ·
   валидация · glossary CRUD · existing dry-run→apply. Коммит 0ec8e30.
5. **Стресс §27** (58d3942): 10k defs → 20k строк через живой create;
   create→paint 11.9с; селекция p50=222мс; бейслайн в evidence.
6. **Визуальные раунды критики §80 ×3** (независимый критик-агент):
   Literata-бренд, 13px-плотность, eyebrow/dot-язык, wine-выделение,
   тёплая dark — внедрено и подтверждено (970a309, 7bbecbd).
7. **Лайна I**: AI-OS skill `product-ui-design` (8 файлов,
   check-skills 0/0, generic) — НА ДИСКЕ, коммит = владельцу (Layer7).

## Баги, найденные и исправленные ночью

- wdio v9: element.waitFor удалён → waitExisting (курс лайны);
- beforeBuildCommand CWD = gui/tauri-app (не src-tauri);
- pkill-гонка давала кадры со старой инстанции (проверять :4457);
- hashchange в фоновом WebKit недостоверен → route-poll 400мс;
- JsonUtility-класс уроков моста здесь не повторялись; camelCase
  build_identity wire (AppInfo-прецедент) — зафиксировано в типах;
- locale = строгая папка-форма «Russian» (семантический отказ бэка
  отловлен диагностикой, UI показал его честно);
- JSX-структура спеки дважды ломалась строчными правками — переписана
  целиком (урок: структурные файлы — целиком, не regex-патчи).

## Приёмка (честные статусы)

| Пункт | Статус |
|---|---|
| React-каркас + канон R1 (tokens/css/fonts) | PASS (build+tsc+кадры) |
| Representative Workspace на живых данных | PASS (smoke 5/5 + кадры) |
| Джорни J1/J2/J3/J4(частично)/J6 | PASS в смоуке |
| Checks/Glossary/Existing/Build экраны | PASS (live-контракты) |
| 10k стресс | PASS (бейслайн; канал-WDIO оговорка) |
| Визуальный канон ≥ Lovable R1 | PARTIAL — критик раунда 3: «база крепкая»; до полного паритета: полировка density в деталях, focus-visible проход, пустые состояния |
| Deterministic geometry-пробы §50 | **PASS** — 25/25 (faa7da6): root-scroll/overflow/blank-tail/controls/panels/dark на 5 маршрутах |
| A11y-проход §56 | NOT RUN |
| Performance-сравнение с Svelte §55 | NOT RUN (есть базовые латентности) |
| Multi-target UI | NOT RUN (одна цель ru; Language Registry — следующая лайна) |
| Settings/providers/diagnostics/selfloc в React | NOT RUN |

## Phase G — owner test packet (готово, ОБНОВЛЕНО после palette/providers/LM)

Обновлено: artifact-rel16-react-r1 v3 (sha 0e1077af, automation React UI).
Production artifact-rel16-react-r1 v2 (sha d780e275, React без automation, guard PASS).
Обе сборки из HEAD main (вся R1-лайна влита).

### Пайплайн сборки (важно!)
Production и automation пишут в ОДИН bundle path. Последняя сборка
перезаписывает предыдущую. Для параллельного использования:
копировать .app в evidence СРАЗУ после каждой сборки.

### Command palette (Cmd+K) — PASS
Cmd+K toggle, Escape close, query filter по секциям — в App.tsx.
Стили palette-overlay/box/input/list/item поверх канона R1.

### Providers экран — PASS
5 шаблонов AI-провайдеров (zai/openai/anthropic/ollama/custom),
статусы, detail-pane, честный mockNote.

### Language Manager — PASS
CRUD пользовательских языков из registry. localStorage персист.
Builtin read-only инвариант.

## Phase G — owner test packet (готово)

- Артефакт React UI v2: `RimLoc-evidence/artifact-rel16-react-r1/`
  (sha256 `b15112ba…`, source ed033e9 — вся R1-лайна + inline-bg fix,
  release-guard static+runtime PASS).
- OWNER_TEST_PACKET.md: 11 пунктов теста (Home → Wizard → Workspace →
  Checks → Glossary → Existing → BuildExport → Multi-target → Selfloc →
  Diagnostics → Settings → Dark theme).
- Известные ограничения: providers/LLM-ключи (клиентский стор Svelte,
  перенос Phase E), palette хелпер (mount в App — Phase E), топбар-кнопка
  в dark (цвет-пара уточняется), human a11y/visual review.
- Тест-инструкции: запустить .app → следовать OWNER_TEST_PACKET.md.
- Svelte fallback: не тронут, откат = запуск frontend-v2 артефакта.

## Блокеры / владельческие

- **Skill product-ui-design коммит** — файлы в
  `~/AI-OS/.agents/skills/product-ui-design/` (8 файлов, валидатор 0/0);
  Layer7 не атрибутирует файлы субагента — закоммитить из сессии владельца.
- Пуш/тег/релиз — только владелец (не трогалось).
- DMG на NFS-таргете падает (стадия дистрибуции) — .app верифицируется;
  перед публичным шагом собирать DMG на внутреннем томе.

## Дальнейшая READY-очередь (по приоритету §75)

1. ~~A11y §56~~ **PASS 8/8** (fdaa8a4): имена/табindex/контраст
   (OKLCH-математика; найден и исправлен реальный дефект канона
   4.43→≥4.5 WCAG AA — токен-девиация задокументирована)/фокус/label.
   Human a11y-review — остаётся в очереди.
2. ~~Multi-target §32~~ **PASS** (7b19b49): registry-driven переключатель
   цели в workspace (9 builtin языков), setTargetLocale ре-мапит из того
   же снапшота; проба независимости ru↔uk PASS (selectByAttribute не
   триггерит React onChange в WebKit — execute+dispatch change).
2. ~~Multi-target §32~~ **PASS** (7b19b49): registry-driven переключатель
   цели в workspace (9 builtin языков), setTargetLocale ре-мапит из того
   же снапшота; проба независимости ru↔uk PASS (selectByAttribute не
   триггерит React onChange в WebKit — execute+dispatch change).
3. Multi-target UX + Language Registry (§32) в wizard/workspace.
4. ~~Diagnostics~~ **PASS** (dc2d95b): project_diagnose → safe bundle
   (operation_id/redacted/excluded). ~~Selfloc J8~~ **PASS** (dc2d95b):
   каталог открывается как обычный проект; contribution — на Build.
   ~~Settings~~ **PASS** (1a0883e): тема/язык UI, проектные дефолты из
   Language Registry, capability-таблица из handshake.
   Providers (LLM-ключи) — остаётся (клиентский стор Svelte-мира).
5. Performance-сравнение с Svelte baseline (§55).
6. Визуальный критик раунд 4 после полировки.

## Evidence

- Кадры: `~/Developing/RimLoc-evidence/ui-r1/` (6 кадров 05:1x + стресс-лог).
- Смоук-логи: /tmp/react-smoke-*.log, стресс: /tmp/react-stress2.log.
- Канон: ~/Developing/RimLoc-reference/{lovable-r1,design-skills-review}.
- Durable чекпоинт: `docs/design/UI_R1_CHECKPOINT.md` (9cffee9).
