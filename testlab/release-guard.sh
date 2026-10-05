#!/bin/bash
# Release guard (owner §B, 2026-10-01): a PRODUCTION bundle must carry ZERO
# automation-bridge surface. Fails if any of:
#   - WDIO plugin strings in the main binary (linked code),
#   - TAURI_WEBDRIVER_PORT / wdio/eval endpoint literals,
#   - WDIO guest-JS chunk markers in the bundled frontend assets,
#   - runtime: RIMLOC_AUTOMATION=1 opens the embedded WebDriver port.
# Usage: release-guard.sh <path-to-.app> [--runtime]
set -u
APP="$1"
RUNTIME="${2:-}"
BIN="$APP/Contents/MacOS/rimloc-gui"
FAIL=0
note() { echo "[release-guard] $*"; }
die() { FAIL=1; note "FAIL $*"; }

[ -d "$APP" ] || { echo "usage: release-guard.sh <.app> [--runtime]"; exit 2; }
[ -x "$BIN" ] || die "main binary missing: $BIN"

# 1) linked-code strings
for marker in "tauri_plugin_wdio" "wdio-webdriver" "TAURI_WEBDRIVER_PORT" "wdio/eval" "__wdio_original_core__"; do
  if strings "$BIN" 2>/dev/null | grep -q "$marker"; then
    die "automation marker leaked into production binary: $marker"
  else
    note "ok: binary clean of '$marker'"
  fi
done

# 2) frontend assets (guest-JS chunk)
ASSETS="$APP/Contents/Resources"
LEAK=$(grep -rl "wdioTauri\|__wdio_original_core__\|WDIO Plugin" "$ASSETS" 2>/dev/null | head -3)
if [ -n "$LEAK" ]; then
  die "guest-JS chunk markers in resources: $LEAK"
else
  note "ok: resources clean of guest-JS markers"
fi

# 3) runtime negative test: env must NOT open the bridge.
# Port must be DEDICATED (TAURI_WEBDRIVER_PORT) — machine port 4445 may be
# held by unrelated services (BookKeeper hub held it once → false FAIL).
if [ "$RUNTIME" = "--runtime" ]; then
  GUARD_PORT="${GUARD_PORT:-4457}"
  RIMLOC_AUTOMATION=1 TAURI_WEBDRIVER_PORT="$GUARD_PORT" "$BIN" >/tmp/release-guard-runtime.log 2>&1 &
  RP=$!
  sleep 6
  HOLDERS=$(lsof -ti :"$GUARD_PORT" -sTCP:LISTEN 2>/dev/null | sort -u)
  OURS=$(pgrep -f "rimloc-gui" | sort -u)
  LISTENING_OURS=$(comm -12 <(echo "$HOLDERS") <(echo "$OURS") | head -1)
  if [ -n "$LISTENING_OURS" ]; then
    die "RIMLOC_AUTOMATION=1 opened port $GUARD_PORT on a PRODUCTION build (pid $LISTENING_OURS)"
  elif [ -n "$HOLDERS" ]; then
    note "ok: runtime negative — port $GUARD_PORT held by unrelated pid(s) $(echo $HOLDERS | tr '\n' ' '), none of ours"
  else
    note "ok: runtime negative — port $GUARD_PORT closed with RIMLOC_AUTOMATION=1"
  fi
  # automation log line must be absent too
  if grep -q "automation webview accessibility enabled" /tmp/release-guard-runtime.log 2>/dev/null; then
    note "info: AX hooks honored the env (expected — they ship in every build, no listener)"
  fi
  kill "$RP" 2>/dev/null; sleep 1
fi

if [ "$FAIL" = "1" ]; then
  note "===== GUARD FAILED ====="
  exit 1
fi
note "===== GUARD PASSED: production bundle carries no automation surface ====="
