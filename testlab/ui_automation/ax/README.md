# AX-канал фоновой GUI-автоматизации (мандат владельца 2026-09-30)

Владелец запретил keystroke-прогоны: System Events `keystroke` требует
frontmost, каждый `activate` передёргивает Space владельца и мешает его
работе. Этот канал управляет установленным release-приложением **без
переключения фокуса**: владелец продолжает работать, кадры приложения
меняются в фоне.

Доказано живьём 2026-09-30 (~12:50): `AXPress(Настройки)` и `AXPress(Справка)`
на REL-12 — `err=0`, кадры меняются (diff bbox ненулевой), frontmost
владельца всё время `firefox`.

## Протокол прогона

1. **Запуск без кражи фокуса** — прямым бинарем с агентским env (`open -a`
   активирует приложение, `open -g` не передаёт env):

   ```bash
   RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 nohup \
     "/Applications/RimLoc GUI.app/Contents/MacOS/rimloc-gui" \
     > /tmp/rimloc-live-ax.log 2>&1 &
   ```

2. **Одна короткая активация** для материализации web-AX-дерева (~1.5 c,
   единственное моргание экрана владельца за прогон; владелец одобрил
   дневные прогоны с этим морганием 2026-09-30), затем сразу вернуть
   фокус:

   ```bash
   PREV=$(osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true')
   osascript -e 'tell application "System Events" to set frontmost of process "rimloc-gui" to true'
   sleep 1.5
   osascript -e "tell application \"$PREV\" to activate"
   ```

   Без активации web-дерево не публикуется вовсе: в окне видны только
   нативные кнопки (traffic lights). Нужна именно frontmost-активация
   NSApp; `AXManualAccessibility` (включается приложением при
   `RIMLOC_AUTOMATION=1`) необходима, но не достаточна.

3. **Фоновые операции** — AXPress/AXSetValue по title/description, кадры
   через `screencapture -x -l<windowid>` (windowid из CGWindowList, не из
   AX). Никаких `activate`/`keystroke` до конца прогона.

## Тулзы (clang, только macOS)

```bash
clang -framework ApplicationServices axwin.c   -o axwin   # состояние моста: AXWindows/FocusedWindow/MainWindow
clang -framework ApplicationServices axdump.c  -o axdump  # дамп дерева (роли/title/value, [AXPress]-метки)
clang -framework ApplicationServices axpress.c -o axpress # фоновое нажатие: axpress <pid> <title>
```

## Канонические обходы (macOS 27 / darwin 27)

- **AXWindows пуст у фонового приложения** — окно брать через
  `kAXFocusedWindowAttribute` (или `kAXMainWindowAttribute`) на элементе
  приложения: отдают живое окно при пустом списке.
- **Мост флапает**: одиночный `AXUIElementCopyAttributeValue` может
  транзиентно отказать (занятость WebKit) — ретраи ~5×300 мс, не
  воспринимать как смерть моста. Дерево прожило ≥90 c полного простоя
  без пересоздания.
- **Элемент из CFArray — незадержанный указатель**: хранить элемент
  дольше `CFRelease(kids)` только через `CFRetain`, иначе висячий
  AXUIElementRef и SIGTRAP при PerformAction (найдено по крашу exit 133).
- **Элемент по координатам** (`AXUIElementCopyElementAtPosition`) видит
  только верхние окна — бесполезен, когда окно приложения перекрыто.

## Ограничения / открытые вопросы

- Единственная активация на прогон пока обязательна (материализация
  web-дерева). Нулевой вариант не найден — вопрос фронтир-модели,
  см. `docs/development/FRONTIER_QUERY_FOCUS_FREE_MACOS_AUTOMATION.md`.
- Ввод текста в поля фоном — не проверен; кандидат `AXSetValue` +
  проверка значения, вместо keystroke.
- Scroll фоном — не проверен; кандидат AXPress на стрелках полосы
  прокрутки или scroll-действия области.
