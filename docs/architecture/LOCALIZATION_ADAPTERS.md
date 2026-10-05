# Localization Adapters — архитектурная граница

Статус: **seam заложен, вторая реализация живая** (selfloc dogfooding). Спекулятивные
адаптеры (Minecraft/Terraria/…) НЕ реализуются — документ проверяет только то, что
граница не потребует их знания сегодня.

## Разделение ответственности

| Слой | Владеет | Примеры |
|---|---|---|
| **Каноническое ядро** (`rimloc-domain/canonical.rs`, `rimloc-services/project*`) | source entry identity (locale-независимая), source value, target locale, контексты, provenance, revision, sourceChanged/new/obsolete/orphan, review state, TM, glossary, AI drafts, persistence, редактор | `Project`, `SourceEntry`, `Translation`, project store, session |
| **Адаптер платформы** | discovery/parse, build/export, target-специфичная валидация, runtime-приёмка | RimWorld: DefInjected/Keyed/TKey/LoadFolders/About.xml/load order; RimLocApplication: UI-catalog schema, плейсхолдеры, plural/select |
| **App Control Fabric** (опционально) | runtime-acceptance поверх живого приложения | RimWorld Adapter → Runtime Bridge; RimLocApplicationAdapter → WDIO |

Правило: ядро не знает RimWorld-типов; RimWorld-знание не размазывается по generic-коду.
Generic core + rich adapter предпочитается слабому «универсальному» формату.

## Идентичность проекта (§F9)

`Project.adapter: AdapterIdentity { adapter_id, adapter_api_version,
adapter_project_schema_version }` (rimloc-domain `canonical.rs`).

- Встроенные first-party id: `adapter_ids::RIMWORLD` ("rimworld"),
  `adapter_ids::RIMLOC_APPLICATION` ("rimloc-application").
- Legacy-конверты без поля грузятся как `rimworld` (serde default) — они и были
  RimWorld-проектами; обратная совместимость не сломана.
- Проект неизвестного адаптера **отказывает чисто**:
  `ProjectLoadDiagnostic::UnknownAdapter { adapter_id, known }` — сообщение называет
  чужой id, список известных и говорит, что файл цел, а семантика чужая. Никакой
  интерпретации чужой семантики как RimWorld, никакой коррупции.
- Версии: новая версия контракта адаптера = additive-minor в рамках
  `adapter_project_schema_version`; смена семантики = bump версии, старые файлы
  читаются миграцией или честным отказом, никогда «как получится».

## Adapter-specific метаданные

Типизированные, не «bag of random JSON»: RimWorld-специфика живёт в типизированных
полях канона, которые СЕЛЕКТИРУЮТСЯ identity адаптера — `EntryKind`
(keyed/def_injected/t_key/strings/backstories/patch_derived — семейства механизмов;
selfloc-сообщения сознательно идут как `Keyed`), `SourceEntryId.def_type` (Option),
`InventoryContext` (target_version/DLC/load order), `SourceProvenance`
(version_selected/conditional_branch/patch_stage/selected_by — словарь
`winner_reason`). Будущий адаптер расширяет набор СВОИМИ типизированными структурами
за своим identity; generic-ядро не обязано их понимать.

## Capability discovery (§F3/§F4)

Адаптер объявляет, что умеет; UI/сервисный слой подстраивается и НЕ рендерит
рабоче выглядящие контролы для неумелых возможностей. Сегодня возможности выводятся
из пути проекта (mod-дерево → RimWorld-экспорт; catalog-дерево → contribution
bundle); machine-readable манифест возможностей — направление (первый живой образец —
мост: `testlab/runtime_bridge/capability-manifest.json` в схеме Computer Fabric).

## Runtime-acceptance интеграция (§F13)

Локализационный адаптер ОПЦИОНАЛЬНО несёт runtime-приёмку:

```
RimWorldAdapter → RimWorld Runtime Bridge (test-only, JSON v1, frame freshness)
RimLocApplicationAdapter → WDIO semantic app control (embedded, P0-гейты)
<future> → свой нативный рантайм-мост или generic CuaDriver fallback
```

Generic-ядро локализации от App Control Fabric НЕ зависит: связь — через адаптер.

## Соблюдённые conformance-инварианты (тесты)

- `project_store::tests::legacy_envelope_without_adapter_defaults_to_rimworld` —
  legacy грузится, identity честный;
- `…::adapter_identity_round_trips` — версии проходят серализацию без потерь;
- `…::unknown_adapter_fails_cleanly_with_diagnostic` — чужой адаптер = чистый отказ;
- `…::rimworld_and_selfloc_adapter_projects_coexist` — два идентити в одном ядре;
- `ui_catalog::tests::selfloc_project_carries_rimloc_application_adapter_identity` —
  selfloc dogfooding;
- редактор/TM/review работают над любым адаптером (один write-path
  `update_translation`) — покрыто общим suite'ом (211/211).

## Направление третьих сторон (только дизайн-шов, §F10/F11)

НЕТ коммитмента на загрузку нативных DLL/dylib в процесс RimLoc. Для будущих
непроверенных/коммьюнити-адаптеров оценивать: отдельный процесс-хост + версионированный
RPC · WASM/sandbox · filesystem capability grants. Встроенные доверенные адаптеры
остаются нативными crates. UI-вкладка адаптеров «на вырост» не заводится: product UX
вернётся к platform-выбору, когда появится второй РЕАЛЬНЫЙ внешний адаптер (§F16).
