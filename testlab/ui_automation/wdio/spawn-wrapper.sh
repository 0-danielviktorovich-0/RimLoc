#!/bin/bash
# wdio appBinaryPath wrapper (frontier spike 2026-09-30).
# Запускает automation-бинарь, затем ВОЗВРАЩАЕТ фокус владельцу
# (одобренное одно моргание; нулевая активация заблокирована багом wry —
# безусловный NSApplication.activate() при создании webview, см. ax/README).
# Использование: spawn-wrapper.sh <путь-к-бинарю>
set -u
BIN="${1:-/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/target/debug/rimloc-gui}"
PREV=$(osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null)
RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 RIMLOC_WINDOW_ORIGIN=-3000,-3000 RIMLOC_WINDOW_MOVE=borderless "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
sleep 2.5
restore() {
  if [ -n "$PREV" ] && [ "$PREV" != "rimloc-gui" ] && [ "$PREV" != "rimloc" ]; then
    osascript -e "tell application \"System Events\" to set frontmost of process \"$PREV\" to true" 2>/dev/null
  fi
}
restore
# Плагин ре-активирует приложение при создании WebDriver-сессии (~+5..10с):
# второй возврат закрывает это окно (замер 30.09: без него блок кражи ~24с).
sleep 9.5
restore
wait "$APP"
