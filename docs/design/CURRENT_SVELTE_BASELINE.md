# CURRENT SVELTE BASELINE — FROZEN LEGACY REFERENCE (R1)

Статус: **FROZEN LEGACY / FALLBACK / REGRESSION ORACLE** (мандат владельца §3-4).
Svelte 5 frontend (`gui/tauri-app/frontend-v2`) НЕ удаляется и НЕ редизайнится;
критические фиксы допустимы только для сохранения operability fallback'а.
Удаление — отдельная задача после React acceptance.

## Build identity

- Репозиторий HEAD на момент заморозки: `f2bba86` (main; волна 13 + soak #3 gate PASS).
- Бинари с identity (`RIMLOC_SOURCE_COMMIT`): артефакты rel15
  - production: sha256 `24e87ffd…` (release-guard static+runtime PASS)
  - automation: sha256 `bdbb128c…` (preflight gate PASS)
- Build-скрипты: `testlab/build/build-production.sh`, `build-automation.sh`;
  acceptance-инфраструктура: WDIO embedded (`e2e/wdio-spike/`), soak v3
  (`soak-runner.sh` + `soak-preflight.sh` identity-гейт), T5-сцены, semantic snapshots.

## Стек и структура

- Svelte 5 (runes) + Vite + TS; хэш-роутер (`src/lib/router.svelte.ts`).
- Один transport-шов: `src/lib/client/{transport,client,mock,types,messages,capability,instance}.ts`
  — typed contract surface над Tauri invoke; режим tauri|mock выбирается ЯВНО
  (`instance.svelte.ts`: Tauri bridge → devMode → mode 'none' с честной ошибкой).
- Данные проекта: `stores/project.svelte.ts` (`source: 'fixture' | 'contract'`),
  persist-before-ack, dirty/acked_revision семантика.
- i18n: `src/i18n/{en,ru}.ts` → `npm run export:catalog` (ONE AUTHORITY,
  generated catalogs 1310/1311 ключей, parity-тест).

## Маршруты (13)

home · wizard · workspace (вкладки editor/review/tm/glossary/project) · review ·
build · existing · chat · glossary · tm · diagnostics · settings · providers · help.

## Карта возможностей (§20: фактическое состояние на заморозку)

| Возможность | Статус | Носитель |
|---|---|---|
| Новый перевод (wizard → project → workspace) | LIVE | Wizard → project.createContractProject → Workspace |
| Открытие/обновление существующего перевода | LIVE | Existing (dry-run import_existing + apply_existing) |
| Persistence управляемых проектов | LIVE | project_store v2 envelope; reopen/restart |
| Editor (select/edit/commit/next, dirty) | LIVE | Workspace editor → apply intents |
| Валидация (026 severity) | LIVE | project_validate |
| Сборка мода / экспорт (PO/…) | LIVE | project_build_mod / project_export (ContractOps) |
| Diagnostics bundle | LIVE | project_diagnose |
| Существующие пакеты: классификация new/changed/orphan/ambiguous | LIVE | import_existing (reusable/conflict/orphan/ambiguous) |
| Глоссарий проекта (CRUD, persist) | LIVE | project_glossary* (волна 13) |
| Self-localization (RimLoc UI catalog) | LIVE | selfloc_catalog_dir + selfloc_build_contribution (β) |
| Multi-target | PARTIAL | один активный проект-цель; языковой реестр есть, независимые цели нескольких локалей в одном проекте — не в UI |
| Review-вкладка | MOCK | ReviewStub (fixture findings) |
| TM-вкладка | MOCK | TMEditor/TMStub (mockTm) |
| AI-провайдеры (LLM) | PARTIAL | ProviderManager: профили/ключи, запуск батчей — вне контракта (chatbatch store, no-API flow) |
| No-API chat batches | PARTIAL | chatbatch store (экспорт пакетов), UI chat |
| Language Registry | PARTIAL | languages/registry.ts (справочник), выбор цели в wizard |
| Source Inspector (per-entry source_ref) | LIVE | snapshot source_ref (wave 12) |
| Command palette | LIVE | CommandPalette (Cmd/Ctrl+K) |
| Настройки | LIVE (клиентские) | settings store (тема/плотность/шрифт…), проектные — через контракты |
| Help | LIVE | help-маршрут |

Правило честности действует: MOCK никогда не выглядит как LIVE
(`transport-mock-badge` / `mock-badge`), e2e honesty-спек закрепляет.

## Acceptance state на заморозку

- Rust: services+domain 231/0, clippy 0; vitest 447+3 skipped (47 файлов);
  Playwright e2e 17 passed + 1 skipped; svelte-check 0/0.
- Soak: 60-мин post-fix official gate = PASS (840 циклов, все нули) —
  `RimLoc-evidence/soak-60-main-v3-official-20261003/`.
- Real-game acceptance: T6 PRODUCT + BACKGROUND AUTONOMOUS = PASS (golden);
  Runtime Bridge v1.0.0 conformance 30/30.

## Известные ограничения (наследуются осознанно)

1. Review/TM вкладки — моки (живые домены не завершены; TM live ждёт брифа).
2. Multi-target UI ограничен одной целью на проект.
3. LLM-батчи живут вне binding-контракта (chatbatch), провайдер-ключи —
   клиентский стор.
4. Проприетарная density-система v1 (не OKLCH-токены Lovable R1).
5. design pass 1.0: элементы реактивности/скролла закрыты гейтами
   (blank-tail/overlap), но визуальная система ниже планки Lovable R1 —
   что и запускает кампанию R1.
