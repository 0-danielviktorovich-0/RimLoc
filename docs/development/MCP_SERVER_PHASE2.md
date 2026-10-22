---
title: "RimLoc external MCP server — Phase 2: GUI-session broker (design)"
status: Proposed
date: 2026-10-20
deciders: архитектурная сессия (главная); реализация — workflow-лейны; приёмка — владелец
related:
  - MCP_SERVER_PHASE1.md
  - ADR-MCP.md
  - AGENT_INTEROPERABILITY_QUALITY_STANDARD.md
tags: [topic/architecture, topic/mcp, kind/design]
---

# Phase 2: GUI-session broker — единая авторитетная сессия

Цель (issue #10, audit п.2/п.5): пока GUI запущен, ОДИН writer — процесс GUI.
Внешний rimloc-mcp подключается к брокеру и выполняет операции через
авторитетный `ProjectSessionManager` GUI; unsaved drafts пользователя
неприкосновенны; React видит внешние изменения. Headless-изоляция Phase 1
(после Lane B: fail-closed на default-профиле) остаётся вторым режимом.

## Архитектура

```
внешний агент → rimloc-mcp (stdio MCP)
                  │  режим A: GUI жив → broker client ──┐
                  │  режим B: GUI нет → headless writer │ UDS, line-JSON
                                                  ▼
        RimLoc GUI (tauri process) ── broker server thread
          ContractState { Mutex<ProjectSessionManager> }   ← единственный writer
          after-mutation: Tauri emit "broker-changed" + client notifications
          React (frontend-react): listen("broker-changed") → store refetch
```

### Broker server (в GUI-процессе)

- **Транспорт:** Unix domain socket `rimloc-broker.sock` в каталоге ПРОФИЛЯ
  (родитель managed root: `RIMLOC_DATA_DIR/` или `<os_data>/com.rimloc.gui/`),
  права 0600, backlog мал. Windows Phase 3+ (named pipe) — здесь не проектируем.
- **Жизненный цикл:** поток `std::thread` со `UnixListener`; привязка
  accept-loop к `AppHandle` НЕ нужна для data-плоскости — брокер-модуль
  принимает `Arc<ContractState>`-подобный seam и callback уведомлений,
  поэтому юнит-тестируется без окна Tauri (окно нужно только emit'у).
  Сокет-файл удаляется на выходе; stale (ECONNREFUSED у клиента) — клиент
  удаляет и работает headless.
- **Протокол v1 (line-JSON, оба направления):**
  - запрос `{id, method, params}` → ответ `{id, result}` | `{id, error{code,message,data}}`;
  - unsolicited `{event:"revision_changed", payload:{project_id, revision, session_epoch, kind}}`
    всем подключённым клиентам после КАЖДОЙ успешной мутации.
- **Методы** — ровно surface MCP Phase 1 (read + guarded writes), исполнение
  через тот же `with_manager`, те же typed ContractError → broker-коды.
  Специфичный запрет Phase 1 сохраняется: paid-инстансы не запускаются
  (проверка local — на стороне брокера, а не только MCP-клиента: доверяй,
  но проверяй).
- **Границы:** ≤4 клиента, строка ≤8 MiB, без secrets в ответах, никакой
  файловой системы вне сервисов, никакой UI-автоматизации (A4 — отдельный
  opt-in слой, НЕ часть брокера данных).

### MCP-режим BrokerClient (в rimloc-mcp)

- При старте: попытка connect к сокету профиля. Жив → режим `broker_client`:
  все project-scoped инструменты и мутации проксируются брокеру; writer-lock
  НЕ берётся (writer — GUI). `rimloc.status` честно: `app=broker_gui`,
  `writer=gui_process`. Мёртв → прежняя headless-модель (Lane B правила:
  default-профиль = read-only отказы, изолированный профиль = writer).
- Прокси-слой тонкий: те же typed-коды ошибок, тот же bounded surface.

### React sync

- `src-tauri`: после успешной мутации через брокер — `emit("broker-changed",
  {project_id, revision, session_epoch, kind})` (новое, четвёртое событие;
  сегодня React не подписан ни на одно — подписка добавляется).
- `frontend-react`: едиственный слушатель в bootstrap: on `broker-changed`
  → store.invalidate(project_id) → refetch snapshot. Черновики фронтенда
  (`draftsAll`, ключи `<locale>::<identity>`) НЕ перезатираются — они
  независимы от snapshot; при расхождении revision UI помечает
  «external change, re-check» (badge), не молча перерисовывает ввод.
- Пейдж-лонг poll (route 400 мс) остаётся как есть.

## Single-writer инвариант (формально)

- GUI жив → все мутации любого клиента идут через менеджер GUI-процесса
  (брокер). Конкурирующий второй GUI-процесс того же профиля не
  поддерживается (нет single-instance; вне скоупа — задокументировано).
- GUI мёртв → мутации только при изолированном RIMLOC_DATA_DIR + writer-lock
  (Phase 1/Lane B). Default-профиль headless = read-only (fail-closed).
- Переходные состояния (GUI стартует при живом headless writer): GUI при
  старте проверяет `rimloc-mcp.lock`; занят и владелец жив (pid+boot
  валиден) → брокер НЕ стартует, GUI предупреждает; mcp-клиенты при
  reconnect падают в headless-правила. Атомарность обеспечивается
  порядком lock→socket-bind.

## Тесты (реальное приложение, изолированный профиль /tmp/rimloc-broker)

1. **Unit (без окна):** брокер-модуль с tempdir-профилем — протокол,
   отказы, bounded, revision_changed после apply.
2. **Integration (GUIless-брокер + mcp client):** поднять брокер над
   tempdir-менеджером, rimloc-mcp в broker_client-режиме, полный сценарий
   A1/A2 из Phase 1 через прокси; paid-guard и stale-refusal негативы.
3. **Живой GUI E2E (WDIO, automation-bridge):** spawn-wrapper с
   RIMLOC_DATA_DIR=/tmp/rimloc-broker; открыть проект в UI; внешний клиент
   применяет перевод; ассерты: ячейка обновилась (React repaint), unsaved
   draft пользователя не потерян, revision badge виден, второй клиент
   со stale revision получает typed refusal.

## DoD

- A3-часть issue #10: GUI+MCP конкурентность = один writer, drafts целы,
  stale — typed отказ; проверено на живом приложении (WDIO), не моком.
- rimloc.status во всех трёх режимах честен (broker_gui / headless_writer /
  other_writer / shared_profile).
- Ни одного network-listener; сокет 0600 в профиле пользователя.
- Phase 1 гарантии не ослаблены (paid-guard дублируется на брокере).

## Вне скоупа Phase 2a

- Semantic UI automation (A4) — следующим слоем (opt-in, поверх
  automation-bridge), НЕ в брокере данных.
- Windows named pipe, Streamable HTTP — Phase 3.
- Второй GUI-процесс одного профиля — не поддерживается, задокументировано.
