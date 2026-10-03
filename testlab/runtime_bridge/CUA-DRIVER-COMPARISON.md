# §10 — CuaDriver capability inventory vs RimWorld Runtime Bridge

Директива POST-PASS HARDENING §10: полная inventory, БЕЗ замены работающего
моста generic computer-use. Источник CuaDriver: матрица параллельной сессии
`aios-computer-fabric/docs/providers/cua.md` (driver 0.31.0, живой list-tools
2026-10-01/03) — их файлы read-only, интеграция только через handoff.

## Сводка по способностям (доказанное)

| Способность | Runtime Bridge v1.0.0 | CuaDriver 0.31.0 (generic) |
|---|---|---|
| Семантическое состояние приложения (версия игры, активный язык, моды, program state) | **native** — `op=state` из живого DefDatabase/LanguageDatabase | нет эквивалента: AX-дерево не несёт языковую/модовую семантику |
| Локализационные lookup (keyed / def-injected, live Translator) | **native** — `op=keyed`, `op=read_def` | нет |
| Семантическая навигация UI (research_tab) | **native** — через игровой API, ноль ввода | приближенно: AX-клики; у Unity-игр AX-экспозиция бедная/отсутствует |
| Фоновый capture кадра | **native** — in-process ScreenCapture + freshness (frame counters, stale-флаг) | `get_desktop_state`/`zoom` — window-bounded скриншот, работает и без AX; freshness = capture_id + TTL 30с (анти-дрейф снапшота, не frame-счётчик) |
| Фоновый ВВОД (клики/печать без фокуса) | не нужен by design (ноль ввода) | **сильная сторона**: click/type «against a target pid», type через AXSetAttribute |
| Точный таргетинг pid+window | QA-каталог + run_id (файловый канал) | **сильная сторона**: pid + window_id + element_token (`s<N>:M`), snapshot-инвариант re-snapshot-per-turn |
| Generic поверх ЛЮБОГО приложения | нет — только RimWorld с установленным тест-модом | **сильная сторона**: нативные приложения и браузеры (exact-bound tab) |
| Lifecycle | `op=quit` — штатный выход | launch_app (background), kill_app (destructive), invoke_menu |
| Identity/evidence | request_id/run_id/state_revision в каждом конверте | snapshot_id/capture_id; trajectory recording из коробки |
| Требуемая подготовка цели | установка тест-мода в клон-профиль (compile-time гейт) | TCC-гранты Accessibility+Screen Recording по app identity |

## Вердикт (§10)

1. **Мост НЕ заменяется CuaDriver** — для RimWorld мост даёт класс истин, которых
   generic-канал принципиально не имеет (DefDatabase/Translator/исследовательский
   UI через игровой API) при нулевом вводе.
2. **CuaDriver genuinely сильнее** там, где моста нет: любое приложение без
   semantic-моста; фоновый ввод (если когда-нибудь понадобится); управление окном
   (set_window_frame с readback); браузерные табы; trajectory-запись.
3. **Иерархия селекции** (решает Fabric, мост только декларирует):
   app-native semantic bridge → background capture → CuaDriver/accessibility →
   vision/global input — последний рубеж.
4. **Честный случай**: релизная сборка RimLoc-приложения НЕ несёт моста
   (compile-time exclusion + release-guard) — для приёмки прод-артефакта
   единственным фоновым каналом остаётся generic-уровень (сейчас это встроенный
   WDIO application adapter RimLoc'а; CuaDriver — кандидат на fallback-уровень
   после owner-gate установки).

## Обмен с Fabric (не реализация за них)

- Отдадили: 9 требований сессии, capability-манифест, freshness-модель
  (`docs/integration/HANDOFF-COMPUTER-FABRIC.md`) — Fabric может представить
  мост как «application-native semantic adapter» рядом с Cua provider.
- Взяли к сведению: snapshot-инвариант CuaDriver (re-snapshot каждый тубн,
  токены умирают со снапшотом) — зеркальный принцип нашему state_revision:
  оба слоя делают «наблюдение → действие» атомарной парой с честным устареванием.
