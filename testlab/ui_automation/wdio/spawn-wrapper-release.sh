#!/bin/bash
# Канонический wrapper для УСТАНОВЛЕННОГО release-артефакта (REL-13+).
# Тот же протокол, что spawn-wrapper.sh: обычное видимое окно, эфемерный
# фрейм, ноль активаций (форки wry/tao).
set -u
# Arg-1 wins, then $BIN (soak-runner passes the artifact under test), then
# the rel14 default — the wrapper must never silently soak a different
# artifact than the runner announced (lesson of the 03.10 run: BIN env on
# the runner did not reach the wrapper, the soak drove rel14 by default).
BIN="${1:-${BIN:-$HOME/Developing/RimLoc-evidence/artifact-rel14-automation/RimLoc GUI.app/Contents/MacOS/rimloc-gui}}"
# Test seam: print the resolved binary and exit — precedence (arg1 > $BIN >
# default) is verified by testlab/ui_automation/wdio/soak-preflight.sh --selftest.
if [ "${WRAP_PRINT_ONLY:-}" = "1" ]; then
  echo "$BIN"
  exit 0
fi
# AUTOMATION artifact only: production (installed in /Applications) carries
# no bridge — soak/E2E must never target it (owner §C artifact classes).
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
RUST_BACKTRACE=1 RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 TAURI_WEBDRIVER_PORT=4457 RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
wait "$APP"
