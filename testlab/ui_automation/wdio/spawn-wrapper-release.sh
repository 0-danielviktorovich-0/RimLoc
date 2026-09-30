#!/bin/bash
# Канонический wrapper для УСТАНОВЛЕННОГО release-артефакта (REL-13+).
# Тот же протокол, что spawn-wrapper.sh: обычное видимое окно, эфемерный
# фрейм, ноль активаций (форки wry/tao).
set -u
BIN="${1:-$HOME/Developing/RimLoc-evidence/artifact-rel14-automation/RimLoc GUI.app/Contents/MacOS/rimloc-gui}"
# AUTOMATION artifact only: production (installed in /Applications) carries
# no bridge — soak/E2E must never target it (owner §C artifact classes).
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
RUST_BACKTRACE=1 RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 TAURI_WEBDRIVER_PORT=4457 RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
wait "$APP"
