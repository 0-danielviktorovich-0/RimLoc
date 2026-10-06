Обновлён: 2026-10-05 ~08:15 JST. Ветка: `glm/ui-r1-react`, HEAD `58765ec`
(worktree `wt-ui-r1`). Основной main: `d0506fb` (Phase A docs; R1-лайна
мержится после owner-test). Лайна содержит весь main.

## Раунд 6 критики (08:08): dark-фикс применён

- var(--accent) в виртуализированных строках этого WebKit резолвил
  LIGHT-значения несмотря на html.dark (осколочный каскад/кеш);
  литеральные oklch + html.dark + !important — не могут проиграть;
- кадры 08:08: selected = тёмный wine-тинт + винная полоска + читаемый
  текст; топбар-кнопка wine с светлым текстом — dark-консистентно;
- смоук 5/5 зелёный.

## Palette mount подтверждён (4c911f9)

- Cmd+K toggle + Escape + query фильтр — в App.tsx;
- полная регрессия: смоук 5/5 · geometry 25/25 · a11y 8/8 — все зелёные;
- §66 обновлён: palette LIVE.

## READY-очередь для следующей сессии

1. Providers/LLM экран — порт клиентского стора (138 строк runes,
  localStorage-персист, шаблоны zai/openai/anthropic/ollama);
2. Command palette (Cmd+K);
3. Language Manager (CRUD пользовательских языков);
4. Human a11y/visual review (§52/§56);
5. Perf-сравнение с Svelte baseline (§55);
6. Selfloc contribution-экран (build_contribution);
7. Wizard polish: «Открыть пример» кнопка, multilingual стрессы.
# UI R1 CAMPAIGN CHECKPOINT (durable, обновлять по ходу)

Обновлён: 2026-10-05 ~07:45 JST. Ветка: `glm/ui-r1-react`, HEAD `1083d13`
(worktree `wt-ui-r1`). Основной main: `d0506fb`+ (Phase A docs; R1-лайна
мержится после representative-acceptance).

## Раунд 4 критики + финал ночи (1083d13)

- dark-selection (цвет текста от foreground), компакт entry-строк,
  file-tree 176px зафиксирован, editor body scroll, nav ellipsis;
- кадры 05:5x в evidence — тёмная тема канона подтверждена кадром:
  Literata-h1, eyebrow, дерево def_injected/ThingDef с count, selected
  wine-тинт, sidebar-progress 25%, multi-target селектор;
- остающиеся полировки: кнопка «Новый проект» в dark (цвет-пара),
  высота карточек Home, human a11y/visual review, perf-сравнение с
  Svelte, providers/LLM экран.

## Ночь 05/06.10 (автономное окно): завершено

- Стресс §27: синтетический мод 10k defs → 20k строк через живой
  wizard-create; create→first-paint 11.9с; селекция p50=222мс;
  прыжки/поиск — канал WDIO доминирует (честная оговорка). Бейслайн
  commit 58d3942, лог в RimLoc-evidence/ui-r1/react-stress-baseline.log.

- Representative Workspace: 3 панели (file-tree из реального инвентаря +
  виртуализированный список + редактор/инспектор), смоук-джорни live.
- Живые маршруты: Home (project_list + wizard J1 + open) · Workspace ·
  Checks (project_validate) · Glossary (project_glossary CRUD) ·
  Existing (import/apply_existing, J2) · Build/Export (J6, gate валидации).
- Смоук 5/5: wizard→create→инвентарь фикстуры · редактор-коммит→ревизия ·
  живая валидация · glossary CRUD · existing dry-run→apply.
- Визуальные раунды критики 1-3 (§80): фикс Literata/13px/топбар/
  heading/63px-строки/wine-выделение/dark/табы/CTA — подтверждено
  критиком; кадры в RimLoc-evidence/ui-r1/.
- Skill product-ui-design (лайна I): 8 файлов на диске в AI-OS,
  check-skills 0/0 — коммит ожидает сессию владельца (Layer7-атрибуция).

## Выполнено (PROVEN, живые прогоны)

- PHASE A: архивы распакованы+identity (b2c6fc38, manifest 0 расхождений);
  `docs/design/CURRENT_SVELTE_BASELINE.md` + `docs/design/LOVABLE_R1_ADAPTATION.md`
  (коммит d0506fb на main); live skill inventory (13 шт, карта
  редактируемости); capability-матрица внутри baseline.
- PHASE B: `frontend-react/` (React 19 + Vite 7 + Tailwind 4 + TS strict);
  tokens.css (:root/.dark/@theme OKLCH) + r1.css (41KB канона) +
  r1-react.css (аддитивный мост); framework-neutral client с инъекцией
  транспорта (7ada3ba); FRONTEND_UI_BOUNDARY.md + PRODUCT_CONTRACT (bbf0e80);
  tauri.react.conf.json (devUrl 5175).
- PHASE C начало: project store (снапшот→entries, drafts, persist-before-ack)
  + Home (живой project_list) + Workspace (resizable-панели, TanStack
  Virtual, редактор core-loop, инспектор source_ref) — 86cdb33.
- **ЖИВОЕ ДОКАЗАТЕЛЬСТВО (§68)**: react-smoke PASS 317мс — boot → живой
  project_list → open → виртуализированные реальные строки → правка →
  save&next → durable revision bump → персистентность (802526a). Артефакт:
  automation-сборка react-конфига; харнесс frontend-v2/e2e/wdio-spike/react-*.
- Визуальные раунды критики §80 (2 раунда) + фиксы (970a309): Literata/13px/
  топбар/63px-строки/wine-выделение/dark — подтверждено критиком; heading
  зона, human-метки (display_name backfill в бэке 216/0), active-nav,
  save&next primary, ellipsis.
- Лайна I (параллельный воркер): AI-OS skill `product-ui-design` создан
  (8 файлов, check-skills 0/0, generic) — **НА ДИСКЕ, НЕ ЗАКОММИЧЕН**:
  AI-OS Layer7 session-ownership блокирует файлы субагента (zcode-nosid).
  Закоммитить из сессии владельца или recovery-тегом.

## В процессе / следующие READY

1. Раунд 3 критики по свежим кадрам 05:17 (localStorage-персист панелей уже
   включён autoSaveId; проверить heading на живом кадре — в DOM есть 107px,
   кадр 05:17 ожидался с ним — ПЕРЕПРОВЕРИТЬ кадр глазами).
2. LEFT context-панель Workspace (§24): file-tree из реального инвентаря
   (def_type группировка) — сейчас 2 панели.
3. Ревью-вкладка/проверки: project_validate живьём в React (metrics-band).
4. Глоссарий: project_glossary CRUD в React (контракт готов, волна 13).
5. Экспорт/сборка: ContractOps-эквивалент (build_mod/export).
6. Wizard новый перевод (§22) + Existing (§23) — живые флоу.
7. i18n: ключи React-лайны → общий каталог (Phase E), снапшот-тест parity.
8. Deterministic QA: портировать geometry-пробы на React (§50), WDIO-фон.
9. Стресс 10k+ (§27): синтетический большой проект + латентности.
10. Teacher-уроки в AUTONOMOUS_ACCEPTANCE_ARCHITECTURE.md при повторах.

## Известные грабли (НЕ повторять)

- `element.waitFor` удалён в wdio v9 → waitUntil(isExisting).
- beforeBuildCommand CWD = gui/tauri-app (пути БЕЗ ../).
- cargo-tauri build exit=1 при упавшем bundle_dmg НА NFS — .app уже собран,
  это не провал сборки; пайп `| tail` маскирует код — писать в лог-файл.
- pkill-гонка: старая инстанция держит 4457 → wdio цепляется к СТАРОМУ
  приложению → фантомные «регрессии» на свежих кадрах. Перед съёмкой:
  pkill + sleep + проверить lsof :4457 пуст.
- Обе сборки (prod/automation) пишут один bundle-путь — копировать артефакт
  в evidence СРАЗУ.
- AI-OS Layer7: файлы субагентов не атрибутированы — коммит из сессии
  владельца; чужие staged-файлы снимать git restore --staged.
- Хуки ZCode-репо RimLoc: subject ≤72, conventional type, body с «- ».
- cargo-хук требует CARGO_TARGET_DIR В ТЕКСТЕ КОМАНДЫ (SSD) +
  CARGO_INCREMENTAL=0 (NFS без flock).

## Блокеры/владельческие

- Skill product-ui-design: коммит (см. выше).
- Пуш/тег/релиз — только владелец.
- DMG-стадия на NFS падает (дистрибуция — на внутреннем томе, перед пабликом).

## Evidence пути

- Кадры: /tmp/rimloc-r1-visual/ (4 кадра, 05:17) → копировать в
  RimLoc-evidence/ui-r1/ при коммитах визуальных волн.
- Смоук-логи: /tmp/react-smoke-*.log; билды: /tmp/react-auto-build*.log.
- Канон Lovable: ~/Developing/RimLoc-reference/lovable-r1/…
- Скилл-ревью: ~/Developing/RimLoc-reference/design-skills-review/…

## §87. Волна 1+2 P0-сходимости (2026-10-05, вечер)

**HEAD feature/ui-r1-convergence = fee1e35** (плавает; ветка = PR #60 → main).

### Решения владельца — СТАРЫЕ НЕ СПРАШИВАТЬ
TM=A+B+C (DRAFT/ACCEPTED/REVIEWED, provenance) · DMG deferred · git: feature→push→Draft PR→merge
разрешён, force/tag/release/notarization запрещены · Svelte frozen legacy · React = прод.
Канон: docs/campaign/STATE_CORRECTION_2026-10-05.md.

### Волна 1 — ЗАВЕРШЕНА И ВЛИТА
- Competitive: 16 конкурентов (13× level-2), Remis=угроза HIGH · TEXT_GRABBER_VS_RIMLOC (ev.5) +
  RIMLANGKIT_VS_RIMLOC (ev.6) на корпусе 4 модов; находки: RimLoc dict-gap (49 типов),
  entity-разметка Keyed, is_version_directory ловит workshop-id (фикс в волне 2).
- Security: guard path-injection (ensure_writable_output_path и др., 314/0) · F-1 закрыт ·
  CODE_SCANNING_RECONCILIATION (130 алертов: 80 FP/43 DEV_ONLY/0 PRODUCTION_REACHABLE) ·
  DEPENDABOT_RECONCILIATION (15 PR: 0 security).
- Docs: SECURITY.md/README/CONTRIBUTING/AGENTS.md переписаны · DOCUMENTATION_AUDIT (168 файлов) ·
  mkdocs strict зелёный · wiki пуста → deprecate.
- TM A+B+C: ветка glm/tm-live ВЛИТА (b0bed86→merge 92b28e6); 248/0 независимо верифицировано;
  React экран #/tm; docs/campaign/TM_LIVE_IMPLEMENTATION.md.

### Волна 2 — идёт (workflow dwfrun-83e9)
fs-adversarial (wt-adversarial) · version-dir-fix (wt-versionfix) · artifact rel18+identity ·
perf React-vs-Svelte · keychain proof (wt-providers) · palette/LM acceptance.
Деливераблы: FRONTEND_PERFORMANCE_EVIDENCE.md, PALETTE_LM_ACCEPTANCE.md, artifact-rel18-react-tm.

### CI PR #60 — почти зелёный
Исправлены: fmt (дважды — TM дрейф), clippy -D warnings (+vendor allow), yoke-derive/deny,
actionlint PATH, public-api (--diff-git-branch не существует → inventory), test --exclude
rimloc-gui + dist в gui-джобе, +frontend-react джоба. Остались: CodeQL summary (default-setup
rust config — owner-настройка), deploy preview (BlobNotFound — контрольный rerun запрошен).

### Среда (важно для свежих сессий)
- Cargo: SSD таргеты ПЕР-WORKTREE (collision last-build-wins на одинаковых пакетах!):
  rimloc (ba-main), rimloc-tm (wt-tm-live), fs-adversarial, versionfix, providers...
  mkdir таргет-каталога ОТДЕЛЬНОЙ командой до cargo (иначе storage-guard unrouted 20+10).
  Всегда CARGO_INCREMENTAL=0 (btrfs). SSD может уснуть (os error 60) — ретрай.
- Диск internal ~14 GiB; npm vite build блокирован сторожем (не heavy-роут), tsc/vitest — нет.
- RimLoc commit-хук: CC type(scope): ≤72 + '- ' буллеты.

### §87.1 Волна 3 competitive (2026-10-05, ночь)

**Tier A закрыт на практике**: 8 конкурентов PRACTICALLY_RUN с same-corpus диффами
(Text Grabber ev.5, RimLangKit ev.6, RimTrans-zh ev.6, RimTranslate ev.5,
Translation Forge ev.5, laskinss27 ev.5, NicoriciN89 ev.5, RimWorldAiTranslator ev.6).
Матрица 24 строки: docs/competitive/TIER_A_COMPLETION_MATRIX_2026-10.md,
0 IDENTITY_UNRESOLVED; 16 NOT DONE (7 feasible → следующая practical-волна:
Remis, Grabber GUI, RimTransAI, RimTrans_PY, etejasdgjjjj532, TokcDK, rtl-tools).

**MUST_FIX_BEFORE_BETA ×2** (подтверждены независимо 3 лейнами):
1. Патч-слой RimLoc пуст: на 3170653412 конкуренты извлекают 54-163 игроку-видимых
   строк (title/description/baseDesc/titleShort), RimLoc 0; обязательный
   blacklist шума (bodyType*/spawnCategories/requiredWorkTags).
2. Порча entity-разметки в msgid export-po: «&lt;b&gt;…» выгружается как «b…b».

Также: scan по умолчанию пропускает корневые Defs при наличии v1.x-папок
(workaround --defs-dir); learn-defs словарь — 49 типов (dict-gap); fill-баг
is_version_directory починен (glm/version-dir-fix, влит).

**Tier B/C/D/E**: docs/competitive/roadmap/ — требования адаптеров (Remis
Protocol-контракт 7 методов = ориентир шва rimloc-plugin-api) + 32 тулзы
north-star (Weblate QA-каталог, Trados TM-бэнды, memoQ concordance, git-native
continuous, PO/XLIFF статусы сегментов).

**Palette/LM acceptance** (волна 2, WDIO живым артефактом, порт 4469):
6/6 команд палитры работают, НО стрелки ↑/↓/Enter отсутствуют (MUST-FIX);
LM CRUD/персистентность/рестарт 14/14+1/1. docs/design/PALETTE_LM_ACCEPTANCE.md.

**WINDOWS_BETA_BLOCKER**: CLI на windows падает STATUS_STACK_OVERFLOW на любой
команде (вплоть до --help; 43/49 тестов). Subprocess-тесты гейтнуты
cfg(not(windows)); чинить стартовую рекурсию (config/i18n?).

### §88. PR #60 СМЕРЖЕН — main снова актуален (2026-10-06)

**origin/main = 4249ba8** (merge PR #60; 279+ коммитов кампании). Локальный ba-main
переключён на main. Feature-ветка feature/ui-r1-convergence удалена (заменяет старый
§87-контекст «PR открыт»).

Финальный CI мержа: 18 pass / 1 fail (CodeQL summary — settings-настройка, задокументированное
исключение) / 2 skip (pages-гейты за PAGES_PREVIEW_ENABLED). **Windows cargo test ЗЕЛЁНЫЙ** —
впервые: clap-flatten снял stack overflow, гейты убраны, юниты+интеграция прошли все 3 ОС.

Волна 2 интегрирована полностью: keychain РЕАЛЬНЫЙ бэкенд (keyring без платформенных
backend'ов молча писал в мок — включены apple/windows-native, restart-proof двумя
процессами), FRONTEND_PERFORMANCE_EVIDENCE (React≈Svelte на бытовых операциях; 222ms =
driver-канал), rel18 artifact+identity (72259e0b), palette/LM acceptance (8 багов
зафиксированы, MUST-FIX список в PALETTE_LM_ACCEPTANCE.md), adversarial 13 проб.

Dependabot: #58 tauri-build, #55 markdown, #56→rebase pymdown, #59→rebase dialog — 3 влито,
2 на ребейзе (@dependabot rebase отправлен). Parallel-session PRы #61 (docs-refresh) и #62
(Codecov) — НЕ мои, ждают своей сессии/ревью.

Дальше: rel19 артефакт от 4249ba8+ (identity + palette WDIO приёмка стрелок) →
OWNER_TEST_PACKET v2. Backlog: PO-optional (docs/development/BACKLOG_PO_OPTIONAL_ARCHITECTURE.md),
7 feasible конкурентов, palette MUST-FIX хвосты.

### §89. P0-инцидент: bump tauri 2.12.1 убил noactivate-патч (2026-10-06)

**Обнаружено rel20-конвейером** (остановлен на блокере сознательно — артефакт без форков
маскировал бы регресс). Bump `6ccedaf` (parallel session, tauri 2.11.6→2.12.1) оставил
`[patch.crates-io]` неиспользуемым: `tauri-runtime-wry 2.12.1` пиннит wry = "0.57.0"
(→ tao 0.37.1), форки 0.55.1/0.35.3 несовместимы. Cargo.lock нёс `[[patch.unused]]` —
main собирал артефакты с безусловной `activateIgnoringOtherApps` (регресс «кражи фокуса»).

**Фикс: PR #67** (fix/noactivate-0.57, a688996): форки пересажены — vendor/
{wry-0.57.0-noactivate, tao-0.37.1-noactivate}, диффы восстановлены ТОЧНО (апстримы
нашлись в registry-кэше, семантика 1-в-1, места отмечены комментариями noactivate).
Runtime-проба: frontmost при запуске GUI остаётся на чужом приложении (PASS ×2).
После мержа #67 старые форки 0.55.1/0.35.3 — кандидат на удаление.

Урок: bump семейства tauri требует проверки `[patch.unused]` в lock (release gate —
добавить в soak-preflight: cargo metadata | grep "patch.*unused" = FAIL).

**§89 финал**: PR #67 СМЕРЖЕН (d964dff) — форки wry 0.57.0/tao 0.37.1 noactivate в main,
runtime-проба фокуса PASS ×2, в soak-preflight новый гейт `--repo` (cargo metadata
[[patch.unused]] = FAIL; проверен живьём: сломанный main FAIL, фикс PASS). §56/#57/#45/#39/#36/#47
Dependabot закрыты по классификации reconciliation; #56 — stale-closed (rebase не состоялся,
Dependabot переоткроет). MUST-FIX palette/LM волна смержена PR #63 (f4bf02f). rel20-конвейер
возобновлён от d964dff: сборка+identity+живой WDIO переведённых спек.

**§89.1**: rel20 принят (358e17a, identity PASS c §89-гейтом --repo, WDIO живьём: palette 30/34 + LM 13/15 — ВСЕ MUST-FIX пробы зелёные; 6 красных = спек-дефекты волны, исправлены PR #69 (2500eae): «перев»=4, compare/tools = home-fallback маркеры, LM-фикстура base36-сабтаг). Реран спек против rel20-артефакта — финальное подтверждение.

**§89.2 ФИНАЛ**: WDIO реран против rel20-артефакта — **palette 34/34 + LM 15/15 = 49/49**,
ноль красных. Все 5 MUST-FIX закрыты с живыми доказательствами; маркеры compare/tools
честно указывают на placeholder (.section-heading) — экраны в parity-бэклоге. rel20 —
актуальный владельческий артефакт (OWNER_TEST_PACKET в evidence-папке).
