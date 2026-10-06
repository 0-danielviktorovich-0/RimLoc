---
title: "ADR-MCP: MCP-поверхность — отложена, CLI является агентным API"
status: Proposed
date: 2026-10-07
deciders: архитектурный decision pass (агентская кампания ba-main), владелец — финальное утверждение
related:
  - AUTONOMOUS_PLAN.md
  - AUTONOMOUS_STATUS.md
  - CANONICAL_PROJECT_MODEL.md
  - "../en/cli/index.md"
tags: [topic/architecture, kind/adr]
---

# ADR-MCP: MCP-поверхность — отложена, CLI является агентным API

**Статус:** Proposed (не коммитится; на утверждение владельца)
**Дата:** 2026-10-07
**Мандат:** §X AUTONOMOUS_STATUS («MCP thin adapter, если останется scope») / §8.5 кампании: реализовать thin MCP adapter **или** оформить осознанный ADR об отложении. Этот документ — второй путь.

---

## 1. Контекст

RimLoc — local-first desktop (Tauri) + CLI продукт для перевода модов RimWorld. Canonical surface по CANONICAL_PROJECT_MODEL:

```
Rust domain/services
  → contract (typed DTO, Capability enum, revision/epoch, UI_CONTRACT_VERSION)
      → Tauri commands (contract_adapter.rs) → GUI (RimLocClient)
      → (нет) MCP
  → CLI (rimloc-cli): детерминированный --format json, schema_version, exit codes
```

Мандат предусматривает MCP tool-минимум: capabilities / list project / scan / entries /
get-set-apply translation / validate / glossary / TM / build / diagnostics.
Вопрос pass: покрыт ли этот минимум уже существующими поверхностями, и если нет —
когда и в какой форме строить MCP.

Факты о текущем состоянии (проверено по коду в этом worktree):

- **CLI уже агентный.** `scan`, `validate`, `validate-po`, `xml-health`, `import-po`,
  `diff-xml`, `coverage`, `doctor`, `compare`, `wordinfo`, `version-diff` отдают
  `--format json` в stdout; JSON-вывод объявлен контрактом («with --format json the
  whole stdout must parse», комментарий в `validate.rs`), несёт `schema_version`
  (`OUTPUT_SCHEMA_VERSION`), баннер уходит в stderr, есть `--quiet` и `--non-interactive`
  поведение. Схемы доменных типов дампятся командой `schema`. Документация
  (`docs/en/cli/index.md`) прямо называет CLI «deterministic/headless surface».
- **Сессионные операции есть только на contract/GUI.** `ProjectSessionManager`
  даёт: managed-проекты (create/open/list/snapshot/refresh), intents
  (`project_apply_intents` с `expected_revision`/`session_epoch` и lost-update
  защитой), glossary CRUD, TM CRUD/lookup/import, provider instances (секрет —
  в OS keychain, на проводе только `has_key`), validate/build/diagnose над
  открытым проектом. Полный список — `Capability` в
  `crates/rimloc-services/src/contract.rs`, регистрация — `CONTRACT_COMMANDS` в
  `gui/tauri-app/src-tauri/src/contract_adapter.rs`.
- **Продакшн-MCP отсутствует.** Никакого `rimloc-mcp` бинарника/крейта нет.
- **Эмпирика кампании:** агентные сценарии в этой и предыдущих кампаниях
  исполняются через CLI как API (scan → LLM переводит → import-po → validate →
  build-mod). Ни один шаг кампании не потребовал MCP.

## 2. Покрытие MCP tool-минимума поверхностями 2026-10-07

| MCP tool-минимум | CLI сегодня | Contract/GUI сегодня |
|---|---|---|
| capabilities | частично: `--help`, `schema` (JSON-схемы доменных типов); единого capability-отчёта нет | да: `capability_report()`, supported/unsupported с причинами |
| list project | по дизайну N/A: CLI безсостоянийный, работает путями; «список проектов» = файловая система | да: `project_list` (+ unloadable-диагностика) |
| scan | **да**: `scan --format json` (детерминированный, `schema_version`) | `project_create` строит инвентарь в сессии |
| entries | **да**: тот же JSON (path/line/key/value/tkey) | `project_snapshot` (read-only зеркала) |
| get/set/apply translation | **косвенно, батчево**: export-po → агент правит PO → `import-po --format json --report/--dry-run` (+ `--incremental`, `--only-diff`, `--backup`) | **да, транзакционно**: `project_apply_intents` (revision/epoch, per-intent skips) |
| validate | **да**: `validate --format json`, `validate-po --format json`, `xml-health --format json` | `project_validate` над сессией |
| glossary | **read-only**: `--glossary` JSON-файл для `translate`/`compare`; CRUD нет | **да**: glossary upsert/delete (wave 13) |
| TM | **read**: `export-po --tm_root`; CRUD/lookup нет | **да**: list/upsert/delete/import/lookup (A+B+C) |
| build | **да**: `build-mod` (+ `--dry-run`, `--skip-empty`, `--dedupe`) | `project_build_mod` / `project_export` |
| diagnostics | **да**: `doctor --format json`; `validate --support-bundle` | `project_diagnose` |

Вывод: **детерминированный батч-конвейер агента покрыт CLI полностью.** Не
покрыты CLI только сессионные CRUD (glossary/TM/intents) — и это осознанный
разделитель: contract обслуживает интерактивную GUI-сессию, CLI — батчевый
конвейер.

## 3. Целевой пользователь MCP и непокрытые сценарии

Целевой потребитель MCP — LLM-агент (как в этой кампании) и автоматизация.
Честный разбор, что агенту реально нужно:

**Покрыто CLI уже сегодня** (и доказано кампаниями): обнаружение единиц
перевода, валидация, батчевое применение переводов через PO, сборка мода,
диагностика окружения. Процесс-spawn на операцию — не недостаток, а желаемая
изоляция: агент получает stateless-операцию с детерминированным выводом и
exit-кодом, не управляя ничьим жизненным циклом.

**НЕ покрывается CLI и не покрылось бы без MCP/сессии:**

1. **Интерактивная многоточечная сессия над одним managed-проектом** — агент и
   человек правят ОДИН проект параллельно, агенту нужны промежуточные
   `revision/epoch`-проверки между правками (сегодня это только GUI).
2. **Incremental state между вызовами инструментов** — открытая сессия с
   dirty-флагом, refresh-семантикой, acked_revision. CLI бессостоянийный по
   дизайну; компенсация — файлы (PO/JSON) как переносимое состояние.
3. **Подписка на изменения** (server→client notifications при изменении
   проекта). Не существует ни на одной поверхности; для batch-агента не нужно.
4. **Glossary/TM CRUD** для агента — сегодня агенту доступен glossary только
   как входной JSON-файл; запись терминов в managed-проект — только через GUI.

Из четырёх пунктов только (1)+(4) имеют наблюдаемую ценность, и оба требуют
не MCP как такового, а **CLI-экспозиции сессионных сервисов** — MCP лишь одна
из возможных транспортных обёрток над ней.

## 4. Рассмотренные варианты

### A. Thin stdio MCP adapter сейчас

Реализовать `rimloc-mcp` (stdio, local-only) поверх contract.

- *За:* протокольная discoverability (`tools/list`); мандат §X упомянут;
  модно для MCP-клиентов.
- *Против:*
  - **Третий исполняемый файл с состоянием.** Adapter поверх contract должен
    хостить `ProjectSessionManager` (managed-root, keychain-sink) — это не GUI
    и не CLI, а новая среда исполнения со своей изоляцией данных
    (`RIMLOC_DATA_DIR`?) и своими багами.
  - **Вторая синхронная поверхность.** Архитектурный гейт требует
    fidelity-матрицы consumer × operation. Сегодня синхронизируются CLI и
    Tauri-adapter; MCP добавляет третью точку к каждому изменению contract
    (правило «wire name appends, never renames» пришлось бы доказывать и там).
  - **Три оси версионирования** вместо двух: MCP protocol version ×
    UI_CONTRACT_VERSION × OUTPUT_SCHEMA_VERSION.
  - **Стоимость**: новый крейт + handshake + маппинг ошибок на каждый инструмент
    + тесты + документация; реальная цена не в LOC (~300–600), а в постоянной
    обязанности синхронизации. При **нулевом доказанном спросе**: ни одного
    запроса, ни одного сценария, который CLI не закрывает.
  - Adapter поверх CLI (обёртка, спавнящая `rimloc-cli --format json`) —
    бессмысленна: добавляет процесс-посредник между агентом и тем же самым CLI.

### B. CLI как agent-API сейчас + MCP после доказанного спроса — **ПРИНЯТ**

- *За:*
  - **Уже реализовано и проверено боем.** Детерминированный JSON, schema_version,
    exit-коды, `doctor`, `schema` — агентные кампании работают через CLI как API
    прямо сейчас. ADR ничего не строит — он фиксирует, что агентным API уже
    является CLI.
  - **Ноль новой поверхности.** Ни fidelity-матрица, ни версионирование, ни
    безопасность не расширяются.
  - **Честное закрытие мандата.** «Не успели» и «не нужно» — разные решения;
    этот документ делает выбор осознанным и обратимым.
- *Против:* агент без MCP-клиента должен уметь читать docs/скилл — на 2026 год
  это не барьер; discoverability решается документацией
  (RimLoc AI USER SKILL, §U — уже в плане), а не протоколом.

### C. Полноценный MCP daemon

Постоянный процесс с сессиями, подписками, полным contract-зеркалом.

- *Против:* для desktop+CLI продукта — максимум состояния и площади атаки
  (lifecycle, managed-root конкуренция с GUI, keychain-доступ из демона) при
  тех же непокрытых сценариях, что и у A. Отклоняется без revisit-шансов до
  смены формы продукта (например, server-режим).

## 5. Решение

**Принят вариант B: production MCP-поверхность не строится в этом цикле; агентным
API объявляется CLI (`rimloc-cli`, детерминированный `--format json` + exit codes).
Thin stdio adapter возвращается к рассмотрению строго по Revisit-условиям (§7).**

Обоснование тремя аргументами:

1. **MCP-минимум для агента уже покрыт CLI детерминированно** (таблица §2):
   scan/entries/validate/build/diagnostics — да, get-set-apply — батчево через
   PO-round-trip, который и есть рабочий агентный сценарий кампаний. Единственный
   непокрытый кластер — сессионные CRUD — требует не MCP, а CLI-экспозиции
   `ProjectSessionManager`, у которой нет ни одного запроса.
2. **Стоимость A — не код, а вечная вторая поверхность**: третий бинарник
   с состоянием, третья точка синхронизации fidelity-матрицы на каждое изменение
   contract, третья ось версионирования, повторная верификация path-гардов и
   секрет-редакции на новом транспорте — при нулевом спросе. Правило reuse-before-create:
   поверхность без потребителя не строим.
3. **Эмпирика сильнее гипотезы**: агенты (включая исполнителя этого ADR) уже
   используют CLI как API в реальной работе; ни один шаг ни одной кампании не
   упёрся в отсутствие MCP. Отсутствие спроса — не предположение, а измеренный факт.

## 6. Последствия (Consequences)

**Положительные:**
- Ноль кода сейчас; весь бюджет остаётся на функциональных pass и релизе.
- Две поверхности вместо трёх: contract → (Tauri | CLI), синхронизация дешевле.
- Агентная discoverability становится задачей документации: RimLoc AI USER SKILL
  (§U плана) должен зафиксировать CLI-конвейер агента (scan → translate →
  import-po → validate → build-mod) как первичный agent-API. Это единственный
  новый deliverable, вытекающий из ADR.

**Отрицательные / принятые риски:**
- Нет `tools/list`-discoverability в MCP-клиентах — компенсируется docs/скиллом.
- Glossary/TM CRUD недоступны агенту — компенсации: glossary как входной JSON;
  TM в батчевом сценарии не критична. Осознанно принимаем до revisit.
- Мандат §X закрыт не кодом, а решением — допустимо, потому что решение
  зафиксировано с условиями пересмотра, а не забыто.

**Нейтральные:**
- Настоящий мост к сессиям, когда спрос появится, — CLI-команды над
  `ProjectSessionManager` (project-create/open/apply-intents/--format json),
  а MCP тогда станет тонкой обёрткой уже над ними. Этот порядок (CLI-сессии →
  MCP-обёртка) дешевле и полезнее, чем MCP поверх contract напрямую: CLI-сессии
  полезны даже без MCP.

## 7. Условия пересмотра (Revisit)

Решение пересматривается при **любом** из:

1. **≥3 независимых запроса агент-интеграции через MCP именно** (issues/discussions
   от сообщества или владельца) — «было бы неплохо» не считается; запрос должен
   называть сценарий, который PO-round-trip не закрывает.
2. **Прямой запрос владельца** подключить RimLoc как MCP-сервер к его клиентам
   (Claude Code / Cursor / Zed) — исполняется в форме thin stdio adapter над
   CLI/сессиями, не daemon.
3. **Доказанный интерактивный сценарий**: агент и человек параллельно правят
   один managed-проект с промежуточными revision-проверками — единственный
   случай, где revision/epoch контракта реально бьёт батчевый CLI.
4. **Стабилизация contract**: `UI_CONTRACT_VERSION = 1` прожил релизный цикл без
   breaking-изменений — тогда thin MCP adapter становится механическим зеркалом
   `CONTRACT_COMMANDS`, и его цена падает до приемлемой.

Вне условий: daemon (C) не рассматривается; revisit возможен только на вариант A.

---

## Приложение: доказательная база

- `crates/rimloc-services/src/contract.rs` — Capability enum (17 supported),
  ContractErrorCode, intents/TM/glossary/providers DTO, UI_CONTRACT_VERSION.
- `gui/tauri-app/src-tauri/src/contract_adapter.rs` — CONTRACT_COMMANDS,
  pass-through правило адаптера, RIMLOC_DATA_DIR изоляция.
- `crates/rimloc-cli/src/lib.rs`, `commands/*.rs` — `--format json` +
  `schema_version` в scan/validate/validate-po/import-po/doctor/compare/
  wordinfo/version-diff/xml-health; комментарий «Machine-readable stdout is
  a contract» (validate.rs:316).
- `docs/en/cli/index.md` — «RimLoc CLI is the deterministic/headless surface».
- `docs/development/AUTONOMOUS_PLAN.md` (L77, L101, L120) — MCP как consumer
  канонической модели, не отдельного состояния.
- `docs/development/AUTONOMOUS_STATUS.md` (L74–75) — мандат §X: «MCP thin
  adapter если останется scope».
