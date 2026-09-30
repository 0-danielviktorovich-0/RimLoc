#!/bin/bash
# Канонический wrapper для УСТАНОВЛЕННОГО release-артефакта (REL-13+).
# Тот же протокол, что spawn-wrapper.sh: обычное видимое окно, эфемерный
# фрейм, ноль активаций (форки wry/tao).
set -u
BIN="${1:-/Applications/RimLoc GUI.app/Contents/MacOS/rimloc-gui}"
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
wait "$APP"
