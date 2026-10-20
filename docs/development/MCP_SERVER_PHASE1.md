---
title: "RimLoc external MCP server — Phase 0/1 proposal"
status: Proposed
date: 2026-10-20
deciders: архитектурная сессия; финальные гейты — владелец (issue #10)
related:
  - ADR-MCP.md
  - AGENT_INTEROPERABILITY_QUALITY_STANDARD.md
  - CLI_PARITY_TRACEABILITY.md
  - ../architecture/LOCALIZATION_ADAPTERS.md
tags: [topic/architecture, topic/mcp, kind/proposal]
---

# RimLoc external MCP server — Phase 0/1 proposal

Owner directive: private issue #10 (2026-10-20). ADR-MCP пересмотрен: thin stdio
adapter принят (revisit-условие №2). Это предложение по форме Phase 1 MVP и
границам безопасности; никакой код не считается сделанным без живых
клиентских тестов (A1–A7 issue #10).

## Phase 0 — что уже есть (аудит, nonblocking)

- Канонические операции: `rimloc-services::contract` + `ProjectSessionManager`
  (session_epoch / expected_revision, read-only source, guarded export).
  Второй движок перевода/экспорта не строится — MCP зовёт те же методы.
- Протокольный референс: `testlab/rimworld/gabs_mcp.py` — живой MCP 2026-07-28
  stdio-клиент (initialize → tools/list → tools/call, stateless `_meta`).
  Он же — первый conformance-клиент для тестов Phase 1.
- P0/P1-дефекты, названные issue #10 блокерами (job-ID/checkpoint reuse,
  implicit auto-rejection в accept selected at `9f9a439`), исправлены на
  интеграционной ветке: collision-proof job IDs + durable records +
  `accept_selected` с явным rejected-ledger. MCP потребляет исправленные сервисы.
- Чего нет: крейта `rimloc-mcp`, GUI-брокера сессий, UI-семантического слоя —
  это Phase 1/2 по порядку ниже.

## Phase 1 — MVP (этот документ)

**Процесс:** `crates/rimloc-mcp` → бинарь `rimloc-mcp`. stdio JSON-RPC,
MCP 2026-07-28 (stateless per-request `_meta`; initialize с negotiation
protocolVersion — не жёстко зашитая единственная версия; отвечаем версией
клиента, если она нам известна, иначе своей). Запускается ТОЛЬКО внешним
MCP-клиентом; установка GUI не стартует никаких служб. Логи — stderr,
stdout — только протокол. `rimloc-mcp doctor` — диагностика окружения.

**Headless-модель v1 (одиночный writer):** процесс поднимает собственный
`ProjectSessionManager` (тот же `RIMLOC_DATA_DIR`-профиль) под эксклюзивным
file-lock на managed root. Если lock занят (живой GUI с тем же профилем) —
`rimloc.status` честно отвечает `app: other_writer`, write-инструменты
отказывают typed-ошибкой; read-only остаётся доступным без lock только если
данные консистентны на диске (job-записи/конверты атомарны). Live-GUI-брокер
и event-sync — Phase 2, в v1 не эмулируются.

**Поверхность инструментов v1** (capabilities, не зеркало CLI-флагов;
JSON Schema 2020-12 на вход, structured output на выход):

Read-only (первый слой):
| Инструмент | Источник (сервис) | Примечание |
|---|---|---|
| `rimloc.status` | локально | protocol+schema версии, headless/other_writer, lock-владелец |
| `rimloc.capabilities` | `capability_report()` | supported/unsupported с причинами |
| `rimloc.projects.list` | `project_list` | + unloadable-диагностика |
| `rimloc.project.snapshot` | `project_snapshot` | read-only зеркало, bounded |
| `rimloc.entries.search` | `project_source_search` | пагинация offset/max_results, HARD_MAX_RESULTS=1000 |
| `rimloc.entry.context` | `project_source_xml_preview` | MAX_FILES=60 / 384KB на файл |
| `rimloc.validation.run` | `project_validate` | |
| `rimloc.export.preview` | `project_output_xml_preview` | truncated-признак наверх |
| `rimloc.jobs.status` | `job_status_with_record` | bounded; восстановленные Interrupted видны |

Контролируемые writes (второй слой, каждый с session_epoch + expected_revision):
`rimloc.project.open`, `rimloc.translation.propose` (черновик, не apply),
`rimloc.translation.apply` (явные identities; stale-refusal по fingerprint),
`rimloc.ai_job.start/status/cancel`, `rimloc.ai_job.accept_selected`
(rejected — явный список), `rimloc.export.build` (только явный guarded
output-dir). Платный провайдер — только через существующий consent/budget
механизм (provider_instance + keychain); сервер не хранит и не отдаёт секреты.

Аннотации честные (`readOnlyHint`, `destructiveHint: false`, `idempotentHint`),
но auth-решения по аннотациям не принимаются.

**Запрещено в v1 (навсегда для этой поверхности):** `run_shell`,
произвольные file read/write, доступ к keychain/секретам, сетевой listener,
запись в source-mod-дерево, apply без pinned revision/epoch, generic
«execute any contract method».

## Phase 2 (не в этом MVP)

GUI-session broker (UDS/named pipe, capability-scoped, one writer),
event-уведомления в React, семантическая UI-автоматизация через нативный
WDIO-мост, `requires_foreground`/`ui_unavailable` честность.

## DoD Phase 1 (сокращённо из issue #10; полная таблица — при реализации)

- A1: живой внешний клиент (gabs_mcp.py-стиль) открывает stdio, получает
  tools/list, листит проекты, берёт snapshot, страницами ищет, валидирует
  fixture-мод; stdout без лишних байтов; schema стабильна.
- A2: изолированный профиль: propose-draft → apply выбранной строки с
  epoch+revision → validate → export; исходник мода байт-в-байт не тронут;
  результат перепарсен и совпадает.
- A5-негативы: stale epoch/revision → typed отказ; запись в source-дерево
  невозможна; секреты не покидают keychain; малформированные запросы —
  typed ошибки, стор не портится.
- Evidence-таблица `MODE|TRANSPORT|VERSION|APP_STATUS|TOOL|…|PASS/FAIL`
  заполняется только живыми вызовами.

## Открытые вопросы к владельцу (не блокируют Phase 1)

1. SDK: официальный Rust MCP SDK (`rmcp`) против ручного stdio JSON-RPC
   (~500 строк, ноль новых deps, полный контроль версий протокола).
   Рекомендация: ручной цикл + conformance-прогон официальным референсным
   клиентом; SDK добавить, если rmcp подтвердит 2026-07-28.
2. Распространение: отдельный cargo-бинарь в crates.io publish-порядке
   (уже есть publish-crates.yml) против bundled-артефакта GUI.
