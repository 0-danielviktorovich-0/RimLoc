# RimWorld Runtime Bridge (test-only)

**Путь в App Control Fabric:** AI-OS App Control Fabric → RimWorld Adapter →
**RimWorld Runtime Bridge**. Generic computer-use слой (CuaDriver и пр.)
владеет параллельной сессией; этот мост — application-native semantic adapter
для RimWorld и НЕ является заменой CuaDriver (селекция механизмов — задача
Fabric: app-native bridge → window/background capture → CuaDriver/accessibility →
vision/global input только как последний рубеж).

Статус: **v1.0.0, PROTOCOL v1 + FRAME FRESHNESS = PASS (live 03.10, 30/30)**.
Golden acceptance (T6 REAL GAME + T6 BACKGROUND AUTONOMOUS) не перезапускался и
не мутировал — маркеры `VWE_HeavyWeapons` / `VWE_ShotRemaining` есть identity
якоря и менять их под изменения моста запрещено.

## Файлы

| Путь | Что |
|---|---|
| `src/RimWorldRuntimeBridge.cs` | источник моста (ONE AUTHORITY для манифеста возможностей) |
| `RuntimeBridge.csproj` + `build.sh` | сборка net48 против Managed-ссылок клона, `--install` ставит DLL в клон-мод |
| `capability-manifest.json` | опубликованный артефакт (экспорт `op=capabilities`; правится только через код) |
| `conformance/v1-conformance.sh` | живой конформаns-прогон — он же минимальный conformance-тест для Fabric (§33) |

## Протокол v1 (JSON envelope, файловый канал)

Запрос: `<savedata>/QA/commands/<id>.json`:

```json
{"protocol_version":1,"request_id":"c1","run_id":"run-...","op":"read_def","args":{"def":"VWE_HeavyWeapons"}}
```

Ответ: `<savedata>/QA/results/<id>.json`:

```json
{"protocol_version":1,"request_id":"c1","run_id":"run-...","ok":true,
 "result":{"def":"VWE_HeavyWeapons","label":"...","description":"..."},
 "error":null,
 "telemetry":{"frame":1916,"main_thread":true,"state_revision":0,"wall_utc":"..."}}
```

Ошибка: `ok:false` + `"error":{"code":"...","message":"...","detail":{...}}`.
Семантические коды (сырые исключения НИКОГДА не контракт; их детали — только
в game log): `BAD_REQUEST` · `PROTOCOL_VERSION_UNSUPPORTED` · `RUN_ID_MISMATCH`
· `CAPABILITY_UNAVAILABLE` · `WRONG_GAME_STATE` · `NOT_IN_COLONY` ·
`TARGET_DEF_NOT_FOUND` · `LANGUAGE_NOT_READY` · `EVIDENCE_ROOT_ESCAPE` ·
`CAPTURE_TIMEOUT` · `INTERNAL`.

Legacy line-format `commands/*.txt` v0.3 принимается (deprecated, без run_id-гейта).

## Опы (узкий whitelist)

`capabilities` · `state` · `select_language{folder}` · `research_tab` ·
`read_def{def}` · `keyed{key}` · `screenshot{path}` · `quit`.
Без shell, без сети, без рефлексии, без произвольного доступа к файловой системе.

## Frame freshness (§5)

`screenshot` возвращает `freshness{frame_at_request, frame_at_present,
frame_advanced, state_revision, stale}`. Наименее инвазивный тест-метод
запроса рендера — сам запрос капчи: `ScreenCapture.CaptureScreenshot`
планируется на конец текущего кадра, т.е. просит ровно один презентованный
кадр — без фокуса и манипуляций окном. Потребитель обязан гейтить на
`stale == false`.

## Модель языка (§6)

- **Identity** = имя папки языка (`folder`), канон; **display** =
  `friendlyNameNative` / `friendlyNameEnglish` — отдельные поля, никогда не
  выводить имя папки из локализованного текста.
- Claim «неактивные RU-паки регистрируют язык» = **UNCONFIRMED** (нужен
  контролируемый A/B или пруф по коду загрузчика); экспортёр от него
  НЕ зависит.
- LanguageInfo.xml для contributor/build-мода = **NOT REQUIRED** (A/B 03.10).

## Модель диспатча (уроки живых прогонов)

- Read-опы (`state`/`keyed`/`read_def`/`capabilities`, precondition-ветка
  `research_tab`) исполняются в bridge-треде мгновенно — long-event очередь
  окклюдированного меню может не подтаивать минутами; честный
  `telemetry.main_thread=false`.
- Unity-API-опы (`select_language`/`screenshot`/`quit`, `research_tab` в
  колонии) — через `SynchronizationContext.Post` (захвачен в статик-кторе,
  исполняется каждый player-loop update).
  ЗАПРЕЩЁН `LongEventHandler.ExecuteWhenFinished` как диспатч: при
  `ProgramState==Entry` он исполняет экшн inline в вызывающем потоке (SIGTRAP).
- `Application.runInBackground = true` форсируется мостом (тест-мод); иначе
  player loop замирает без фокуса и диспатчи голодают.

## Граница угроз (threat boundary, §9)

- Тест-инфраструктура: ставится ТОЛЬКО в изолированный клон-профиль
  (`-savedatafolder`), никогда не в прод-артефакт RimLoc (compile-time
  feature-гейт `automation-bridge` + release-guard).
- Локальный файловый канал внутри песочницы; никаких листенеров/сети.
- `run_id` гейт: команды чужого запуска отклоняются; протухшие файлы
  карантина при старте.
- `screenshot` пишет только внутрь evidence-root (QA-каталог песочницы;
  override `RIMLOC_BRIDGE_EVIDENCE_ROOT`), иначе `EVIDENCE_ROOT_ESCAPE`.
- Захваченный кадр и state.json — evidence-материал; прод-данные RimWorld и
  сейвы владельца не читаются и не пишутся.

## Conformance (§33) — минимальный сценарий для Fabric

`bash conformance/v1-conformance.sh` — полный живой цикл в фоне, без
foreground takeover, без pointer/keyboard, без Space-переключений:
capabilities → state → keyed → read_def → research_tab (структурная ошибка в
меню) → screenshot+freshness → fence-отказ → graceful quit. 30/30 PASS.

## Сборка

```bash
bash build.sh            # dotnet build net48 против Managed клона
bash build.sh --install  # + замена DLL в клон-моде (packageId не меняется)
```
