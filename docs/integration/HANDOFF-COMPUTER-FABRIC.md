# Handoff → AI-OS Computer Fabric session: RimWorld как application adapter

От: RimLoc-сессия (владелец RimWorld Adapter / Runtime Bridge)
Кому: параллельная computer-use сессия (владелец generic App Control Fabric)
Дата: 2026-10-03. Полная версия: RimLoc repo `testlab/runtime_bridge/README.md`,
evidence `RimLoc-evidence/p0-incident-20261001/{t6-background-acceptance,runtime-bridge-v1}/`.

## Что доказано (всё — live)

- **RimWorld Runtime Bridge v1.0.0** — application-native semantic adapter для
  RimWorld 1.6 (Unity Mono player). Протокол v1: JSON envelope
  `{protocol_version, request_id, run_id, op, args}` →
  `{ok, result, error{code,message,detail}, telemetry}` по файловому каналу
  внутри песочницы (`savedata/QA/commands → results`).
- Конформанс 30/30 (35с, полностью фон: без foreground takeover, pointer,
  keyboard, Space): discovery → state → keyed → def_injected → semantic
  navigation → background frame capture + freshness → structured errors →
  graceful quit.
- Golden acceptance (T6 REAL GAME / T6 BACKGROUND AUTONOMOUS = PASS 03.10) —
  не перезапускался, маркеры-фикстуры не мутировали.

## Capability manifest (machine-readable)

`op=capabilities` + артефакт `testlab/runtime_bridge/capability-manifest.json`
(записи в schema `aios-computer-fabric/contracts/schemas/capability.schema.json`:
dotted-lowercase name, class observe/app/recording, proven, evidence):

runtime.ready · runtime.state · localization.keyed · localization.def_injected ·
ui.research_navigation · capture.game_frame · capture.frame_freshness ·
lifecycle.quit — все proven. Constraints: `accessibility.semantic_ui = limited`,
`global_input.required = false`, `foreground_required = false`.

## Security model (граница)

Test-only изолированный клон-профиль (`-savedatafolder`), локальный файловый
канал, узкий whitelist из 8 опов, БЕЗ shell/сети/рефлексии/произвольного fs;
run_id-гейт + карантин чужих команд; capture-fence внутрь evidence-root;
семантические коды ошибок (сырые исключения никогда не контракт); прод-данные
RimWorld и сейвы владельца не читаются/не пишутся.

## Требования сессии (что Fabric должен уметь представлять)

1. explicit target application/PID (здесь: процесс игры + QA-каталог сессии);
2. explicit control session с run-identity (run_id гейт отклоняет чужие команды);
3. capability discovery до вызовов (`op=capabilities`, proven-флаги);
4. attach semantic runtime bridge (не эмуляция UI — семантика приложения);
5. invoke semantic op / observe semantic state;
6. background window/frame capture с freshness-телеметрией;
7. evidence correlation (request_id/run_id/state_revision/frame в каждом ответе);
8. non-interference telemetry (harness не выдаёт глобальный ввод; frontmost
   владельца не трогается);
9. graceful target lifecycle (op=quit, штатный выход).

## Поведение фонового капчи-канала

`ScreenCapture.CaptureScreenshot` в фоне захватывает презентованный кадр; сам
запрос — наименее инвазивный «попроси один кадр» (EndOfFrame), без фокуса.
Freshness: `frame_at_request/at_present/advanced/stale`; consumer обязан гейтить
на `stale == false`. Обязательное условие живости — Run-In-Background
(игровая настройка песочницы + `Application.runInBackground = true` в мосту):
без него player loop замирает без фокуса и все main-thread диспатчи голодают.

## Чего не хватает в generic Fabric (наш список требований, не реализация)

- модель «application-native semantic adapter» рядом с generic drivers
  (селекция: app-native bridge → background capture → CuaDriver/accessibility →
  vision/global input — последний рубеж);
- сессии с run-identity и карантином чужих команд;
- freshness-телеметрия как первоклассное поле observation;
- evidence-корреляция (id-цепочка запрос-ответ-кадр) на уровне broker audit.

## Минимальный conformance-тест (для Fabric, готов сегодня)

`bash testlab/runtime_bridge/conformance/v1-conformance.sh` (в RimLoc repo,
worktree ba-main) — полный живой цикл против клона RimWorld, 30/30 PASS.
Fabric-уровневая версия того же сценария: discover → session → capabilities →
state → localization lookup → semantic navigation → background capture →
structured evidence → close — БЕЗ foreground/pointer/keyboard/Spaces.

## Граница владения (§28/§30/§37)

Generic Fabric/CuaDriver/broker — ваш. RimWorld Adapter/Runtime Bridge/RimLoc
WDIO-адаптер — наш. Мы НЕ строим второй generic Fabric; ждём от Fabric модель
адаптеров — интеграция без переписывания моста (thin local adapter уже стоит).
