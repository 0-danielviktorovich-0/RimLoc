# rimloc-broker

GUI-session broker RimLoc (Phase 2, дизайн: `docs/development/MCP_SERVER_PHASE2.md`).
Пока GUI запущен — ОДИН writer: процесс GUI. Этот крейт — его data-plane:
Unix-сокет, line-JSON протокол v1, все операции внешних клиентов идут через
авторитетный `ProjectSessionManager` хоста. Никаких window/Tauri-зависимостей:
брокер принимает seam (`BrokerOps`) и callback уведомлений, поэтому
юнит-тестируется без окна.

## Протокол v1 (line-delimited JSON, оба направления)

- запрос `{id, method, params}` → ответ `{id, result}` | `{id, error{code,message,data}}`
  (одна из веток на проводе, вторая опускается, не `null`);
- unsolicited `{event, payload}` всем подключённым клиентам после КАЖДОЙ
  успешной мутации: `{"event":"revision_changed","payload":{project_id,
  revision, session_epoch, kind}}`;
- каждый кадр — один компактный JSON + `\n` (компактность гарантирует, что
  внутри кадра нет сырого перевода строки);
- клиент различает кадры по форме: есть `id` — ответ, есть `event` — событие.

Коды ошибок: зарезервированные JSON-RPC 2.0 (-32700..-32603) + серверные
-32002 `contract_error` (стабильный snake_case код контракта в `data.code`),
-32004 `paid_providers_disabled`, -32005 `too_many_clients`, -32006
`line_too_long` — зеркала кодов `rimloc-mcp` (`crates/rimloc-mcp/src/error.rs`).

## Методы (ровно surface MCP Phase 1, 1:1 к инструментам `rimloc.*`)

`projects.list` · `project.snapshot` · `entries.search` · `entry.context` ·
`validation.run` · `export.preview` · `jobs.status` · `project.open` ·
`translation.propose` · `translation.apply` · `ai_job.start` · `ai_job.status` ·
`ai_job.cancel` · `ai_job.accept_selected` · `export.build`.

`rimloc.status` / `rimloc.capabilities` — поверхности MCP-рантайма, брокеру не
нужны: брокер и есть статус. Два режима источника менеджера
([`OpsImpl::new`] / [`OpsImpl::over_shared_manager`]):

- **Root (headless/тесты)** — паттерн `crates/rimloc-mcp/src/tools.rs`:
  свежий `ProjectSessionManager` на вызов, `open`, одна типизированная
  операция, дроп менеджера;
- **Shared (хост-GUI, Phase 2)** — один менеджер-инстанс, клон ЖИВОГО
  менеджера хоста (`ProjectSessionManager` — `Arc`-Clone,
  `rimloc-services/src/session.rs:101`: общий реестр сессий, provider-стейт
  и keychain-sink). Живые сессии читаются через `snapshot()` (без бампа
  epoch); `open()` — только при отсутствии живой сессии. Единственный
  метод, бампающий epoch, — `project.open` (это и есть его смысл: mint
  новой генерации сессии). Неявный бамп на каждом вызове инвалилировал бы
  сессии самого GUI — ровно то, что Phase 2 запрещает.

Все коллекции на проводе ограничены (snapshot 200 записей, findings 200,
поиск — пагинация сервиса), усечение помечается флагами, не молчит.

## Границы и гарантии

- **Сокет**: `rimloc-broker.sock` в КАТАЛОГЕ ПРОФИЛЯ (родитель managed root:
  `RIMLOC_DATA_DIR/` или `<os_data>/com.rimloc.gui/`), права **0600** сразу
  после bind. Никакого network-listener.
- **Жизненный цикл**: при старте, если файл сокета существует —
  `UnixStream::connect`-проба: соединилось → живой брокер, старт отказывает
  (`AlreadyRunning`); не соединилось → stale-файл, unlink и bind. Drop
  `BrokerServer` останавливает accept-loop и удаляет файл.
- **Лимиты**: ≤4 клиентов одновременно (5-й получает typed `too_many_clients`
  и закрывается); строка ≤8 MiB (`line_too_long`, соединение закрывается —
  фрейминг после oversized-строки недостоверен); ≤8 исполняемых строк
  одновременно на соединение (readер паркуется — пайплайн не плодит потоки).
- **Паники**: каждая строка исполняется в своём потоке под `catch_unwind`;
  паника обработчика = один typed internal error (`handler_panicked`),
  сервер живёт.
- **Paid-guard дубль**: `ai_job.start` по НЕ-local инстансу (поле `local`
  записи провайдера, `crates/rimloc-services/src/providers.rs:146`) → typed
  отказ `paid_providers_disabled` ДО любого запуска — независимо от того,
  что проверил клиент («доверяй, но проверяй»).
- **Секреты**: никогда не пересекают провод в обе стороны; `ai_job.start`
  принимает только ID провайдер-инстанса; CRUD провайдеров на поверхности
  брокера отсутствует вовсе.
- **Stale-защита**: epoch/revision-гвэрды сервисного контракта без изменений;
  stale epoch/revision — typed `stale_epoch`/`stale_revision`, никогда не
  слепая перезапись.

## События `revision_changed` (kind → источник координат)

| kind | когда | revision / session_epoch |
|---|---|---|
| `project_open` | успешный `project.open` | ack открытия (новый epoch) |
| `propose` | успешный `translation.propose` | `project_revision` черновиков (ревизия НЕ растёт) |
| `apply` | успешный `translation.apply` | НОВАЯ ревизия из ack |
| `ai_job_start` | успешный `ai_job.start` | координаты сессии этого вызова |
| `ai_job_cancel` | успешный `ai_job.cancel` | `base_revision` задачи; epoch-сентинел `0` (у cancel нет сессии) |
| `accept_selected` | успешный `ai_job.accept_selected` | пост-apply `manager.snapshot()` (без bump epoch) |
| `export_build` | успешный `export.build` | координаты сессии (экспорт ревизию не меняет) |

## Откуда запускается

Хост — GUI-процесс (`gui/tauri-app/src-tauri`): на старте он создаёт
`OpsImpl` (или свою реализацию `BrokerOps` над живым `ContractState`) и зовёт

```rust
let server = rimloc_broker::server::BrokerServer::start(
    &managed_root,                    // <profile>/managed — сокет упадёт в <profile>/
    ops,                              // Arc<dyn BrokerOps>
    notify,                           // Arc<dyn Fn(BrokerEvent) + Send + Sync> → Tauri emit("broker-changed")
)?;
```

События fan-out'ятся всем подключённым сокет-клиентам И уходят в `notify`
(Tauri-проводка живёт в хосте, не здесь). React-подписка на
`broker-changed` — часть Phase 2 (см. дизайн-док). MCP-клиентский режим
(`broker_client` в `rimloc-mcp`) — вторая половина фазы, живёт в своём
крейте; этот крейт от `rimloc-mcp` не зависит.

Windows (named pipe) — Phase 3, здесь не проектируется; второй GUI-процесс
того же профиля не поддерживается (задокументировано в дизайне).
