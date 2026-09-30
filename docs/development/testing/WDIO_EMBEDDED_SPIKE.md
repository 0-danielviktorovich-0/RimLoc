# Embedded WDIO spike — отчёт (2026-09-30, фронтир-директива)

Проводка: `tauri-plugin-wdio-webdriver` 1.4.0 (embedded WebDriver-сервер, TCP 4445)
+ `tauri-plugin-wdio` 1.4.0 (execute/моки/логи) в RimLoc, оба под runtime-гейтом
`RIMLOC_AUTOMATION=1`. WDIO: `@wdio/tauri-service` 1.4.0, driverProvider=embedded.
Бинарь: debug (workspace target), окно за экраном (RIMLOC_WINDOW_ORIGIN=-3000,-3000,
borderless). Спека: `frontend-v2/e2e/wdio-spike/spike.spec.ts`. Обёртка:
`testlab/ui_automation/wdio/spawn-wrapper.sh` (запуск + двойной возврат фокуса).

## Результаты чеклиста фронтира

| Шаг | Результат |
|-----|-----------|
| Сессия с реальным приложением без активации | ✅ (webkit 605.1.15, embedded-сервер в приложении) |
| Home семантически | ✅ h1 текст |
| Клик → навигация (фон) | ✅ Настройки → На главную, оба клика фоном |
| Ввод текста | ✅ native setter + InputEvent (Svelte-видимый), round-trip значения |
| Смена вкладки | ✅ «Проверка» в workspace |
| Скролл контейнера | ✅ UL.queue 3731/262 px, scrollTop=200 |
| Геометрия | ✅ innerW/H, docH, hOverflow=false |
| Бэкенд-оп (browser.tauri.execute) | ✅ (fa4422b: guest-плагин под автоматизационным гейтом; project_list → 8 живьём) |
| Логи фронт/бэк | ✅ доступны через captureBackendLogs/captureFrontendLogs (мост жив) |
| Закрытие сессии | ✅ |

Итог: **1 passing** (3м57с до моста → **1.3с** после: 5с-таймауты хуков были почти всей длительностью); полный джорни Home→Настройки→Home→ввод→проект→
вкладка→скролл→геометрия — полностью в фоне, окно за экраном, владелец работал
(ZCode/godot/ChatGPT/FluidVoice переключались свободно).

## Не-интерференция (монитор frontmost/pointer/clipboard, 1с)

- Указатель: нами не трогается (плагин — только DOM-события; движения в логах —
  активность владельца).
- Клипборд: нетронут во всех прогонах.
- frontmost: чистая drive-фаза — 2.5+ мин без единой кражи (финальный прогон
  21:49:38→21:52:11 сплошняком у владельца). НО:
  - стартовое моргание ~1.2с (spawn) — гасится первым restore (2.5с);
  - активация при создании сессии (~+5с, блок до 24с) — гасится вторым restore (12с);
  - **остаточные блоки ре-активации в хвосте длинных прогонов** (~24с×2,
    причина не до конца атрибутирована; кандидат — фолбэк сервиса
    switchWindow при 5с-таймаутах моста → makeKeyAndOrderFront).

## Нулевая активация — фальсифицирована (вверх по стеку)

`focus:false`, `open -g`, `nohup`-spawn — ВСЕ активируют: **wry 0.55.1
безусловно вызывает NSApplication.activate() при создании webview**
(`src/wkwebview/mod.rs`, блок «make sure the window is always on top»).
Апп-уровневые `deactivate()` и `yieldActivationAsNeeded()` ПОСЛЕ setup фокус
не возвращают (в лонч-последовательности есть более поздняя ре-активация).
Вывод: нулевая активация = патч wry / upstream PR; в app-коде недостижима.

## Матрица AX vs embedded WDIO (по evidence 30.09)

| Критерий | Фоновый AX | Embedded WDIO |
|----------|-----------|---------------|
| Активация для старта | 1 моргание (материализация web-AX-дерева) | 1–2 моргания (spawn + сессия; гасятся обёрткой) |
| Фоновые клики | ✅ AXPress | ✅ el.click() DOM |
| Текстовый ввод | ⚠️ AXSetValue не проверен | ✅ native setter + InputEvent (Svelte) |
| Скролл | ⚠️ не проверен | ✅ scrollTop/scrollHeight прямые числа |
| DOM-геометрия | нет (только AX-координаты) | ✅ полная (rect/scroll/client) |
| Accessibility-view | ✅ нативное AX-дерево | нет (для этого есть AX) |
| IPC/бэкенд-опы | нет | 🔜 мост (шим не прижился — follow-up) |
| Логи фронт/бэк | trace.jsonl (RIMLOC_TRACE) | 🔜 тот же мост |
| Нативные диалоги (файлы) | ✅ (System Events) | ❌ |
| Влияние на фокус | стабильно ноль после старта | чисто в drive-фазе; остаточные блоки в хвосте |
| Ресурсы | C-тулзы, копейки | Node/wdio + сервер в приложении |
| Часовая стабильность | не мерилась (soak общий follow-up) | не мерилась |

## Полис (по директиве §4, не финальный вердикт)

- Оба канала сохраняются; «canonical» не объявляется до soak + моста.
- WDIO — кандидат в primary для web-джорни (мощность DOM), AX — для
  accessibility-проверок, нативных диалогов и как независимый второй сенсор.
- Фолбьюки WDIO до моста: бэкенд-истина через RIMLOC_TRACE (уже есть),
  T2a Playwright покрывает UI-логику.

## Soak §11 — 55 минут (30.09, 22:35–23:30 JST)

Automation-бинарь (окно за экраном), 650 циклов: ре-аквизиция элементов
каждую итерацию, навигация в обе стороны, живой бэкенд-оп (`project_list`),
скролл-пробы каждые 5-е, простои 20 с.

- **1 passing, 55м10с; ошибок 1/650 (0.15% при гейте 10%)**; латентность
  шага p50 54–58 мс, p95 1.1 с — мост стабилен час.
- Владелец весь час активно работал (ChatGPT/godot/ZCode/Finder/firefox/
  Terminal/bookkeeper) — drive-фаза фокус не крала.
- **Хвостовая проблема воспроизведена и атрибуирована по времени**: два
  блока активации rimloc-gui (21 с на 15-й минуте; 128 с на 50-й) —
  единственная ошибка соука (~23:24) совпадает с началом 128-с блока
  (23:24:16): кандидат — фолбэк-переключение окна сервиса при ошибке
  элемента. Детали ошибки в лог не печатались (spec считал их молча) —
  follow-up: печатать сразу + трассировать switchWindow.
- Evidence: soak-55min.log, soak-noninterference.jsonl (2743 сэмпла).

## P0 zero-activation — RE-SOLVED (2026-10-01, владелец)

Инцидент: спавн без restore украл фокус → владелец отменил терпимость
к «одному морганию»: НОЛЬ активаций в background-only. Расследование
(WindowServer-лог + bisect) нашло **четыре активатора в стеке**:

1. wry wkwebview/mod.rs — NSApplication::activate() при создании webview;
2. tao app_state.rs window_activation_hack — makeKeyAndOrderFront видимого
   окна на event loop;
3. tao window.rs focused-ветка — makeKeyAndOrderFront при создании;
4. tao async set_focus — makeKey + activateIgnoringOtherApps:YES (сильнейший,
   пробивал все флаги) + launch-активация в app delegate.

Все зашиты в vendor-форках ([patch.crates-io]; window-status: orderFront
без key/activate). Замер: 10/10 запусков без единой кражи (было 10/10
краж). §12 acceptance: 12м20с, 135 циклов, 0 ошибок, 0 программных
активаций, 0 manual-reveals (watchdog 852 сэмпла, evidence
RimLoc-evidence/p0-incident-20261001/). Политика
RIMLOC_AUTOMATION_POLICY=background-only|exclusive — в манифесте;
превышение глушится watchdog'ом мгновенно.

## Открытые follow-up

1. ~~Мост~~ — ЗАКРЫТ (fa4422b): guest-плагин под автоматизационным гейтом.
2. Остаточные ре-активации — временная корреляция с ошибкой элемента
   получена (соук); нужна детальная атрибуция fallback switchWindow +
   печать деталей ошибок в соуке.
3. wry-патч на безусловный activate (upstream issue/PR) — единственный путь
   к честной нулевой активации.
4. Soak 60–120 мин победителя (директива §11).
5. Release-бинарь с плагинами: собран только debug; для release-прогона —
   пересборка релиза (после security-ревью границы плагина, директива §10).
