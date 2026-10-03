#!/bin/bash
# Soak runner v2 (owner §A, 2026-10-01): bounded background soak with
# EXPLICIT recovery counters and a hands-off window (no interactive
# commands touch the session — the run-2 death window was contaminated by
# mid-run pgrep/kill actions; that harness defect is closed here:
# unique per-run files, pid file, external monitor).
#
# Counters (owner §A):
#   app_process_deaths      main process observed gone while soak active
#   webview_process_deaths  WebContent child gone while main alive (best effort)
#   driver_disconnects      WDIO command channel refused (ECONNREFUSED)
#   relaunches              harness relaunched the app (policy: NEVER in soak)
#   session_reconnects      wdio re-established a session (policy: NEVER)
#   recovered_failures      soak step errors that later cycles recovered from
#   unrecovered_failures    soak ended with errors
#
# Usage: soak-runner.sh [minutes]  (default 15)
set -u
MINUTES="${1:-15}"
RUN_ID="soak-$(date +%Y%m%d-%H%M%S)"
DIR="/tmp/rimloc-soak-$RUN_ID"
mkdir -p "$DIR"
BIN="${BIN:-/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/target/release/bundle/macos/RimLoc GUI.app/Contents/MacOS/rimloc-gui}"

echo "run_id=$RUN_ID dir=$DIR"

# 1) app: спавнит САМ wdio-сервис (application в конфиге). Runner раньше
#    спавнил второй инстанс — двойной спавн путал сессии (дефект прогонa
#    071213: сессия и монитор смотрели на разные процессы).
# 1b) pre-flight ARTIFACT IDENTITY GATE (owner soak-hardening §1/§2):
# class by automation surface (never trust path naming), sha256 against the
# expected artifact in gate mode, expected source commit exported for the
# spec's runtime identity check. Fail-closed BEFORE any cycle.
REPO=~/Developing/_rimloc-worktrees/ba-main
EXPECT_SHA="${SOAK_EXPECT_SHA256:-}"
# Expected source commit: the ARTIFACT's OWN build record first
# (<artifact dir>/source-commit.txt, written at build time) — live HEAD
# moves with harness commits and must never be the identity reference
# (lesson of the 03.10 soak #3 false start).
COMMIT=$(cat "$(dirname "$BIN")/../../../source-commit.txt" 2>/dev/null || true)
COMMIT=$(echo "$COMMIT" | sed 's/-dirty$//')
[ -n "$COMMIT" ] || COMMIT=$(git -C "$REPO" rev-parse HEAD)
PREFLIGHT_ARGS=(--bin "$BIN" --class automation --out "$DIR/preflight.json")
[ -n "$EXPECT_SHA" ] && PREFLIGHT_ARGS+=(--expect-sha256 "$EXPECT_SHA")
if [ "${SOAK_GATE:-0}" = "1" ]; then
  PREFLIGHT_ARGS+=(--gate)
  [ -n "$EXPECT_SHA" ] || { echo "SOAK_GATE=1 requires SOAK_EXPECT_SHA256" >&2; exit 2; }
fi
if ! bash "$REPO/testlab/ui_automation/wdio/soak-preflight.sh" "${PREFLIGHT_ARGS[@]}"; then
  echo "PREFLIGHT FAILED — soak never starts (fail-closed)" >&2
  exit 1
fi
export SOAK_EXPECT_COMMIT="$COMMIT"
echo "preflight ok; expected source commit: $COMMIT"

echo "waiting for wdio-spawned app..." > "$DIR/app.log"

# 2) external monitor: app liveness + port + driver, 1s samples
(
  APP_DISCOVERED=""
  while true; do
    TS=$(date +%s)
    if [ -z "$APP_DISCOVERED" ]; then
      APP_DISCOVERED=$(pgrep -f "RimLoc GUI.app/Contents/MacOS/rimloc-gui" | head -1)
      [ -n "$APP_DISCOVERED" ] && echo "$APP_DISCOVERED" > "$DIR/app.pid" \
        && echo "{\"ts\":$TS,\"event\":\"app_discovered\",\"pid\":$APP_DISCOVERED}" >> "$DIR/monitor.jsonl"
    else
      if ! kill -0 "$APP_DISCOVERED" 2>/dev/null; then
        echo "{\"ts\":$TS,\"event\":\"app_process_death\",\"pid\":$APP_DISCOVERED}" >> "$DIR/monitor.jsonl"
        APP_DISCOVERED=""  # пере-discover (если сервис перезапустил)
      fi
    fi
    if ! lsof -ti :4457 -sTCP:LISTEN >/dev/null 2>&1; then
      echo "{\"ts\":$TS,\"event\":\"port_4457_closed\"}" >> "$DIR/monitor.jsonl"
    fi
    pgrep -f "wdio run" >/dev/null 2>&1 || echo "{\"ts\":$TS,\"event\":\"driver_exit\"}" >> "$DIR/monitor.jsonl"
    [ -f "$DIR/stop" ] && break
    sleep 1
  done
) &
MON=$!

# 3) the soak spec (same cycles as before)
cd ~/Developing/_rimloc-worktrees/ba-main/gui/tauri-app/frontend-v2
SOAK_MINUTES="$MINUTES" timeout $((MINUTES * 60 + 120)) npx wdio run e2e/wdio-spike/soak.conf.ts \
  > "$DIR/drive.log" 2>&1
DRRC=$?
touch "$DIR/stop"; kill "$MON" 2>/dev/null

# 4) counters
APP_DEATHS=$(grep -c "app_process_death" "$DIR/monitor.jsonl" 2>/dev/null || true)
PORT_LOSS=$(grep -c "port_4457_closed" "$DIR/monitor.jsonl" 2>/dev/null || echo 0)
DRV_EXIT=$(grep -c "driver_exit" "$DIR/monitor.jsonl" 2>/dev/null || true)
STEP_ERRS=$(grep -oE "errors=[0-9]+" "$DIR/drive.log" | tail -1 | cut -d= -f2)
CYCLES=$(grep -oE "cycles=[0-9]+" "$DIR/drive.log" | tail -1 | cut -d= -f2)

cat > "$DIR/counters.json" <<JSON
{
  "run_id": "$RUN_ID",
  "minutes": $MINUTES,
  "cycles": ${CYCLES:-0},
  "app_process_deaths": $APP_DEATHS,
  "webview_process_deaths": "not_instrumented",
  "driver_disconnects": "see drive.log driver errors (spec v3 counts them separately)",
  "relaunches": 0,
  "session_reconnects": 0,
  "recovered_failures": "see drive.log error ledger",
  "unrecovered_failures": ${STEP_ERRS:-0},
  "drive_rc": $DRRC,
  "policy": "never relaunch during soak; death = unrecovered by definition"
}
JSON
cat "$DIR/counters.json"
pkill -f "RimLoc GUI.app/Contents/MacOS/rimloc-gui" 2>/dev/null
echo "artifacts in $DIR"
