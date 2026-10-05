#!/bin/bash
# T6 EXCLUSIVE lease runner (§9) — финальный визуальный захват маркера.
# Запускается ТОЛЬКО в согласованное с владельцем idle-окно (2-3 мин).
# Lease-правила:
#  - единственный источник глобального ввода — этот скрипт;
#  - перед каждым вводом: frontmost == игра, иначе ABORT (без ретраев
#    в чужие окна);
#  - после маркера: гасим игру, возвращаем frontmost владельцу.
set -u
TD=~/Developing/_rimloc-worktrees/ba-main/testlab/run/t6
BIN="$TD/RimWorldTest.app/Contents/MacOS/RimWorld by Ludeon Studios"
OUT=~/Developing/RimLoc-evidence/p0-incident-20261001/t6-final
mkdir -p "$OUT"
FM() { osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null; }
PREV=$(FM)
echo "lease start; owner frontmost was: $PREV"

# 1) запуск до главного меню (язык применится при загрузке меню)
"$BIN" "-savedatafolder=$TD/sandbox" > /dev/null 2>&1 &
sleep 40
osascript -e 'tell application "System Events" to set frontmost of process "RimWorld by Ludeon Studios" to true' || exit 1
sleep 2
[ "$(FM)" = "RimWorld by Ludeon Studios" ] || { echo "ABORT: lease lost at menu"; exit 1; }
screencapture -x "$OUT/menu-ru.png"

# 2) Новая колония → мир → высадка (vision-guided: координаты пишет оператор
#    по мере прохождения; каждый клик с гардом frontmost)
# CLICKS_FILLED_BY_OPERATOR

# 3) завершение
pkill -f "RimWorldTest.app" 2>/dev/null
sleep 2
[ -n "$PREV" ] && osascript -e "tell application \"System Events\" to set frontmost of process \"$PREV\" to true" 2>/dev/null
echo "lease end; game closed; frontmost restored to $PREV"
