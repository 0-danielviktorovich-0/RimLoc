#!/bin/bash
# React-lane (R1) spawn wrapper: same non-interference protocol as the
# Svelte wrapper — visible window, ephemeral frame, ZERO activations.
# BIN precedence: arg1 > $BIN > the react bundle default. Optional isolated
# data dir via RIMLOC_DATA_DIR (the caller decides the sandbox).
set -u
BIN="${1:-${BIN:-/Volumes/Portable-SSD/caches/targets/AI-OS/release/bundle/macos/RimLoc GUI.app/Contents/MacOS/rimloc-gui}}"
if [ "${WRAP_PRINT_ONLY:-}" = "1" ]; then echo "$BIN"; exit 0; fi
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
RUST_BACKTRACE=1 RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 TAURI_WEBDRIVER_PORT=4457 RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
wait "$APP"
