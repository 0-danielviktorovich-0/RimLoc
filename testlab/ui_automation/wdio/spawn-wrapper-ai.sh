#!/bin/bash
# AI-cancel acceptance lane: React automation build, isolated data dir,
# mock jobs explicitly enabled for the IPC-level journey (the production
# frontend hides the mock mode — that hiding is itself asserted in-spec).
set -u
BIN="${1:-${BIN:-/Volumes/Portable-SSD/caches/targets/rimloc/release/bundle/macos/RimLoc GUI.app/Contents/MacOS/rimloc-gui}}"
if [ "${WRAP_PRINT_ONLY:-}" = "1" ]; then echo "$BIN"; exit 0; fi
FRAME=$(/tmp/place-win 2>/dev/null || echo "348,70,980x640")
RUST_BACKTRACE=1 RIMLOC_AUTOMATION=1 RIMLOC_TRACE=1 TAURI_WEBDRIVER_PORT=4471 \
  RIMLOC_DATA_DIR=/tmp/rimloc-ai/data RIMLOC_ALLOW_MOCK_JOBS=1 \
  RIMLOC_WINDOW_FRAME="$FRAME" "$BIN" &
APP=$!
cleanup() { kill "$APP" 2>/dev/null; }
trap cleanup EXIT INT TERM
wait "$APP"
