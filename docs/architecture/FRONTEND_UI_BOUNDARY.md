# FRONTEND UI BOUNDARY (R1)

Мандат §7/§47/§62. Цель: смена UI-фреймворка не требует переписывания
RimLoc domain/services/backend. Сейм уже существует и проверен кампанией —
этот документ фиксирует его канон для React-лайны.

## Слои

```
React components (frontend-react/src/components, src/routes)
        ↓  hooks/state (React)
RimLocClient (frontend-react/src/lib/client/)   ← framework-neutral контракт
        ↓  typed ops
Tauri IPC (invoke, snake_case-аргументы)
        ↓
rimloc-services (Rust) → rimloc-domain / rimloc-core
```

## Канон контракта

- **Модуль**: `gui/tauri-app/frontend-react/src/lib/client/{types,transport,client}.ts`
  (скопирован из замороженного Svelte-клиента при бутстрапе лайны; wire-DTO и
  поверхность операций ИДЕНТИЧНЫ — Svelte-копия заморожена как legacy oracle).
- **Поверхность**: handshake + capability report; create/open/list/snapshot/
  apply/refresh/cancel/validate/export/build_mod/diagnose/import_existing/
  apply_existing/pick_directory/selfloc_catalog_dir/selfloc_build_contribution/
  glossary (list/upsert/delete)/build_identity. Append-only: коды ошибок и
  команды никогда не переименовываются.
- **Транспорт инъекционный**: `createRimLocClient({mode:'tauri', invoke})` или
  `{transport}` (тесты/будущие лайны). Мок НЕ бандлится в React-лайну —
  отсутствие моста = честная ошибка конфигурации (никогда не молчаливый мок).
- **Семантика**: persist-before-ack (dirty/acked_revision), stale_epoch/stale_revision
  — типированные отказы, source-drift, honesty (LIVE/PARTIAL/DEMO/UNSUPPORTED
  виден пользователю).

## Правила

1. **Домен в React не переизобретается**: парсинг, effective source resolution,
   DefInjected/Keyed/TKey/LoadFolders, валидатор, TM, глоссарий-семантика,
   persistence, build, filesystem safety, adapter-семантика — только Rust.
2. **React state трёх видов**: domain state (зеркало снапшота/контракта),
   application state (открытый проект, эпоха, busy), view state (layout,
   фильтры, выделение). Не смешивать.
3. **Capability-driven UI**: unsupported-возможности не рендерят рабочие
   контролы (handshake capability report + adapter identity, §8).
4. **i18n**: EN+RU first-class; новые ключи React-лайны уходят в общий
   self-localization каталог (Phase E — унификация экспортёра каталога).
5. **Замороженный Svelte**: frontend-v2 не импортируется из React-лайны
   (кроме сгенерированных данных, если понадобятся) и не изменяется.

## Миграция/зачистка

Cutover (Phase G, только после acceptance и отдельно): frontend-v2 удаляется
или архивируется; shared-данные (i18n каталоги, клиент) остаются жить в
React-лайне или переезжают в общий пакет — решение фиксируется отдельным
коммитом с evidence.
