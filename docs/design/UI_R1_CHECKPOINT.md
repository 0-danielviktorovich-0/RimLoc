# UI R1 CAMPAIGN CHECKPOINT (durable, обновлять по ходу)

Обновлён: 2026-10-05 ~05:35 JST. Ветка: `glm/ui-r1-react`, HEAD `970a309`
(worktree `wt-ui-r1`). Основной main: `d0506fb`+ (Phase A docs; R1-лайна
мержится после representative-acceptance).

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
