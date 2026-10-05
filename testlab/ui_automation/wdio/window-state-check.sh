#!/bin/bash
# Window-state regression checks (owner window-correction directive §11).
# A: normal launch → move → quit → relaunch → user frame RESTORED.
# B: automation launch (ephemeral) → move → quit → normal relaunch →
#    ORIGINAL user frame intact (automation never persists).
# C: owner app stays frontmost during an automation run.
# D: automation window fully inside a display visible frame.
# F: automation window not fullscreen-sized.
# Uses the SAME debug binary for both modes (normal = no RIMLOC_AUTOMATION).
set -u
BIN="${1:-/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/target/debug/rimloc-gui}"
WINFRAME=/tmp/winframe
PASS=0; FAIL=0
note() { echo "[wcheck] $*"; }
check() { if [ "$2" = "0" ]; then PASS=$((PASS+1)); note "PASS $1"; else FAIL=$((FAIL+1)); note "FAIL $1 — $3"; fi; }

frame_of() { # pid → "x,y,WxH" главного окна слоя 0
  $WINFRAME "$1" 2>/dev/null | grep -oE 'frame=[0-9]+,[0-9]+,[0-9]+x[0-9]+' | head -1 | cut -d= -f2
}
main_frame_of() { # pid → ждёт ГЛАВНОЕ окно (≥700x500): служебные мини-окна
  # WKWebView читаются раньше главного — без поллинга проверка гонкается
  local f="" i
  for i in $(seq 1 40); do
    f=$(frame_of "$1")
    if [ -n "$f" ]; then
      local w h; w=$(echo "$f" | cut -d, -f3 | cut -dx -f1); h=$(echo "$f" | cut -d, -f3 | cut -dx -f2)
      if [ "${w:-0}" -ge 700 ] && [ "${h:-0}" -ge 500 ]; then echo "$f"; return 0; fi
    fi
    sleep 0.5
  done
  echo "$f"; return 1
}
inside_visible() { # "x,y,WxH" → 0 если целиком в каком-то visibleFrame
  python3 - "$1" "$2" <<'PY'
import sys
fx, fy, rest = sys.argv[1].split(','); fw, fh = rest.split('x')
fx, fy, fw, fh = map(int, (fx, fy, fw, fh))
ok = False
for scr in sys.argv[2].split('|'):
    sx, sy, srest = scr.split(','); sw, sh = srest.split('x')
    sx, sy, sw, sh = map(int, (sx, sy, sw, sh))
    if fx >= sx and fy >= sy and fx+fw <= sx+sw and fy+fh <= sy+sh:
        ok = True; break
sys.exit(0 if ok else 1)
PY
}
fullscreen_sized() { # окно ≈ во весь экран (видимый или полный)?
  python3 - "$1" "$2" <<'PY'
import sys
fx, fy, rest = sys.argv[1].split(','); fw, fh = rest.split('x')
fx, fy, fw, fh = map(int, (fx, fy, fw, fh))
area = fw*fh
for scr in sys.argv[2].split('|'):
    sx, sy, srest = scr.split(','); sw, sh = srest.split('x')
    sx, sy, sw, sh = map(int, (sx, sy, sw, sh))
    if area >= 0.97*sw*sh:
        sys.exit(1)  # fullscreen-sized
sys.exit(0)
PY
}
# P0 (2026-10-01): ноль активаций — restore_prev удалён (System Events
# set frontmost = форма передёргивания). Форки wry/tao дают чистый спавн.
launch_normal() { # → pid
  nohup "$BIN" >/tmp/wcheck-normal.log 2>&1 & echo $!
  sleep 3.5
}
launch_automation() { # → pid (КАНОНСКИЙ путь: wrapper + эфемерный фрейм)
  bash "$(dirname "$0")/spawn-wrapper.sh" >/tmp/wcheck-auto.log 2>&1 & echo $!
  sleep 4.5
}
move_window() { # pid x y — по ИМЕНИ главного окна (window 1 гонкается)
  osascript -e "tell application \"System Events\" to tell process \"rimloc-gui\" to set position of (first window whose name is \"RimLoc GUI\") to {$2, $3}" 2>/dev/null \
    || osascript -e "tell application \"System Events\" to tell process \"rimloc-gui\" to set position of window 1 to {$2, $3}" 2>/dev/null
}
quit_app() { # graceful: AX-крестик → окно закрыто → tauri выходит штатно,
  # window-state плагин сохраняет фрейм (SIGTERM пропускает сохранение)
  /tmp/axquit "$1" 2>/dev/null; sleep 1.2
  kill "$1" 2>/dev/null; sleep 0.8
}

pkill -f "target/debug/rimloc-gui" 2>/dev/null; sleep 1

# --- §11A: user frame persists across normal launches ---
P=$(launch_normal); sleep 1
move_window "$P" 200 300; sleep 1
F1=$(main_frame_of "$P"); note "A moved to: $F1"
quit_app "$P"
P=$(launch_normal); sleep 1.5
F2=$(main_frame_of "$P"); note "A relaunched at: $F2"
python3 - "$F1" "$F2" <<'PY' && check "§11A user frame restored" 0 "" || check "§11A user frame restored" 1 "$F1 → $F2"
import sys
def parse(f):
    x,y,r=f.split(','); w,h=r.split('x'); return tuple(map(int,(x,y,w,h)))
a,b=parse(sys.argv[1]),parse(sys.argv[2])
sys.exit(0 if abs(a[0]-b[0])<8 and abs(a[1]-b[1])<8 and a[2]==b[2] and a[3]==b[3] else 1)
PY
USER_FRAME="$F2"
quit_app "$P"

# --- §11B+C+D+F: automation run — ephemeral frame + non-interference ---
PREV_APP=$(osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null)
P=$(launch_automation)
sleep 0.5
P=$(pgrep -f "target/debug/rimloc-gui" | tail -1)  # бинарь под wrapper'ом
SCREENS=$($WINFRAME "$P" 2>/dev/null | grep -oE 'screens=\[.*\]' | cut -d'[' -f2 | tr -d ']')
AF=$(main_frame_of "$P"); note "automation frame: $AF (screens: $SCREENS)"
inside_visible "$AF" "$SCREENS"; check "§11D inside visible frame" $? "$AF"
fullscreen_sized "$AF" "$SCREENS"; check "§11F not fullscreen-sized" $? "$AF"
# C: фокус владельца в середине автоматизационной сессии
sleep 2
FM=$(osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null)
[ "$FM" != "rimloc-gui" ]; check "§11C owner stays frontmost (was $PREV_APP, now $FM)" $? "frontmost=$FM"
# B: сдвигаем автоматизационное окно, гасим, обычный запуск — юзер-фрейм цел
move_window "$P" 600 500; sleep 1
quit_app "$P"
P=$(launch_normal); sleep 1.5
F3=$(main_frame_of "$P"); note "B after automation: $F3 (user frame was $USER_FRAME)"
python3 - "$USER_FRAME" "$F3" <<'PY' && check "§11B automation frame ephemeral" 0 "" || check "§11B automation frame ephemeral" 1 "$USER_FRAME → $F3"
import sys
def parse(f):
    x,y,r=f.split(','); w,h=r.split('x'); return tuple(map(int,(x,y,w,h)))
a,b=parse(sys.argv[1]),parse(sys.argv[2])
sys.exit(0 if abs(a[0]-b[0])<8 and abs(a[1]-b[1])<8 else 1)
PY
kill "$P" 2>/dev/null

echo "[wcheck] ===== $PASS passed, $FAIL failed ====="
exit $([ "$FAIL" -eq 0 ] && echo 0 || echo 1)