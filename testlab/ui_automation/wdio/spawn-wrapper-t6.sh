#!/bin/bash
# T6 narrow-window lane wrapper (#161): packaged AUTOMATION build in a
# NORMAL VISIBLE window placed at the derived safe frame (right-bottom of
# the real visibleFrame — /tmp/place-win, 980x640), ephemeral (the
# window-state plugin is off in automation sessions), ZERO activations
# (forked wry/tao). Same protocol as spawn-wrapper.sh / spawn-wrapper-release.sh.
#
# Differences from the siblings: the resolved app pid is published via
# $T6_PIDFILE so the lane can attach native measurements (winframe.py,
# shot-window.py) to the EXACT process the embedded WebDriver drives, and
# the binary is taken from arg-1 > $T6_BIN (the runner pins the artifact —
# never the rel14 default: identity gate soak-preflight.sh decides upstream).
# Usage: spawn-wrapper-t6.sh <path-to-rimloc-gui-binary>
set -u
BIN="${1:-${T6_BIN:-}}"
if [ -z "$BIN" ]; then
  echo "spawn-wrapper-t6: no binary (pass arg-1 or \$T6_BIN)" >&2
  exit 2
fi
if [ ! -x "$BIN" ]; then
  echo "spawn-wrapper-t6: binary not executable: $BIN" >&2
  exit 2
fi
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
# Test isolation (owner directive §4): a disposable projects universe via
# RIMLOC_DATA_DIR — the owner's managed projects are absent entirely.
RIMLOC_DATA_DIR="${T6_DATA_DIR:-${RIMLOC_DATA_DIR:-/tmp/rimloc-t6/data}}" \
RUST_BACKTRACE=1 RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 \
  TAURI_WEBDRIVER_PORT="${T6_PORT:-4475}" \
  RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
if [ -n "${T6_PIDFILE:-}" ]; then
  printf '%s' "$APP" > "$T6_PIDFILE"
fi
cleanup() {
  kill "$APP" 2>/dev/null
  [ -n "${T6_PIDFILE:-}" ] && rm -f "$T6_PIDFILE"
}
trap cleanup EXIT INT TERM
wait "$APP"
