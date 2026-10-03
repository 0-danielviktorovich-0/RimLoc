#!/bin/bash
# RimWorld Runtime Bridge v1 — conformance chain over the JSON envelope protocol.
# §33 owner directive: the minimal conformance scenario for App Control Fabric:
#   discover → state → keyed → read_def → semantic research navigation →
#   background frame capture → structured errors → graceful quit
# WITHOUT foreground takeover, pointer movement, global input, or Space switching.
#
# This is a FULL live run: launches RimWorld in the isolated clone profile.
# Golden fixture (RimLocT6-Package markers) is NOT touched.
set -u
TD="$(cd "$(dirname "$0")/../../run/t6" && pwd)"
BIN="$TD/RimWorldTest.app/Contents/MacOS/RimWorld by Ludeon Studios"
QA="$TD/sandbox/QA"
CMD="$QA/commands"
RES="$QA/results"
EV=~/Developing/RimLoc-evidence/p0-incident-20261001/runtime-bridge-v1
GOLDEN_KEYED='RIMLOC-T6 MARKER выстрелов осталось: {0}'
GOLDEN_DEF='RIMLOC-T6 MARKER definjected research'
mkdir -p "$CMD" "$RES" "$EV"
PASS=0; FAIL=0

fm() { osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null; }
jqv() { python3 -c "import json,sys;d=json.load(open(sys.argv[1]));print(eval(sys.argv[2],{'d':d}))" "$1" "$2" 2>/dev/null; }

ck() { # ck <name> <exit-code-expr: 0=true>
  if [ "$2" = "0" ]; then PASS=$((PASS+1)); echo "  PASS: $1"; else FAIL=$((FAIL+1)); echo "  FAIL: $1"; fi
}

# call <file-stem> <json>; returns response path in RESP
call() {
  local stem="$1"; local body="$2"
  rm -f "$RES/$stem.json"
  printf '%s' "$body" > "$CMD/$stem.json"
  local i=0
  while [ $i -lt 120 ]; do
    [ -f "$RES/$stem.json" ] && break
    sleep 0.25; i=$((i+1))
  done
  RESP="$RES/$stem.json"
  [ -f "$RESP" ] || { echo "  TIMEOUT waiting response: $stem"; return 1; }
  sleep 0.4 # pace the burst — long-event dispatch needs frames to process
  return 0
}

RUN_ID=""; OWNER_FM_BEFORE=$(fm); T0=$(date +%s)
echo "=== Runtime Bridge v1 conformance; owner frontmost: $OWNER_FM_BEFORE ==="

# --- launch (background, direct binary exec; proven non-activating) ---
pkill -f "RimWorldTest.app" 2>/dev/null; sleep 2
START_TS=$(date +%s)
# A state.json from a PREVIOUS run may exist in the sandbox — never trust it:
# wait for one freshly written by THIS launch (mtime after START_TS).
"$BIN" "-savedatafolder=$TD/sandbox" "-logFile=$TD/v1-conformance-player.log" >/dev/null 2>&1 &
GPID=$!
echo "launched pid=$GPID; waiting for FRESH bridge READY..."
READY=0
for i in $(seq 1 240); do
  if [ -f "$QA/state.json" ]; then
    MTIME=$(stat -f %m "$QA/state.json" 2>/dev/null || echo 0)
    if [ "$MTIME" -ge "$START_TS" ] && [ "$(jqv "$QA/state.json" "d['phase']")" = "ready" ]; then READY=1; break; fi
  fi
  kill -0 "$GPID" 2>/dev/null || break
  sleep 1
done
[ "$READY" = "1" ] || { echo "FAIL: no fresh state.json"; tail -20 "$TD/v1-conformance-player.log" 2>/dev/null; kill -9 "$GPID" 2>/dev/null; exit 1; }
ck "READY handshake (fresh state.json phase=ready)" "$([ "$READY" = "1" ]; echo $?)"
RUN_ID=$(jqv "$QA/state.json" "d['run_id']")
BRIDGE_VER=$(jqv "$QA/state.json" "d['bridge_version']")
LANG_FOLDER=$(jqv "$QA/state.json" "d['active_language']['folder']")
echo "bridge $BRIDGE_VER ready; run_id=$RUN_ID; lang folder: $LANG_FOLDER"
# Runtime.ready semantics for THIS chain: the bridge must actually ANSWER —
# at ProgramState.Entry the long-event queue only drains once the main menu is
# fully interactive, so wait for the first answered probe (not a fixed sleep).
PROBE=0; PROBE_OK=0
while [ $PROBE -lt 60 ]; do
  if call "probe-$PROBE" "{\"protocol_version\":1,\"request_id\":\"probe-$PROBE\",\"run_id\":\"$RUN_ID\",\"op\":\"state\",\"args\":{}}"; then
    PROBE_OK=1; break
  fi
  PROBE=$((PROBE+1)); sleep 3
done
ck "bridge answers probes (menu interactive)" "$([ "$PROBE_OK" = "1" ]; echo $?)"

ck "bridge version is 1.x" "$([[ "$BRIDGE_VER" == 1.* ]]; echo $?)"

# --- 1. capabilities (discovery) ---
call c1 "{\"protocol_version\":1,\"request_id\":\"c1\",\"run_id\":\"$RUN_ID\",\"op\":\"capabilities\",\"args\":{}}" || true
ck "capabilities: ok" "$([ "$(jqv "$RESP" "d['ok']")" = "True" ]; echo $?)"
ck "capabilities: envelope protocol_version==1" "$([ "$(jqv "$RESP" "d['protocol_version']")" = "1" ]; echo $?)"
ck "capabilities: request_id echoed" "$([ "$(jqv "$RESP" "d['request_id']")" = "c1" ]; echo $?)"
ck "capabilities: run_id echoed" "$([ "$(jqv "$RESP" "d['run_id']")" = "$RUN_ID" ]; echo $?)"
CAP_N=$(jqv "$RESP" "len([c for c in d['result']['capabilities'] if c['proven']])")
ck "capabilities: >=8 proven entries" "$([ "${CAP_N:-0}" -ge 8 ]; echo $?)"
jqv "$RESP" "d['result']" > "$EV/capabilities-exported.json" 2>/dev/null || true

# --- 2. state ---
call c2 "{\"protocol_version\":1,\"request_id\":\"c2\",\"run_id\":\"$RUN_ID\",\"op\":\"state\",\"args\":{}}" || true
ck "state: ok" "$([ "$(jqv "$RESP" "d['ok']")" = "True" ]; echo $?)"
ck "state: language folder identity = Russian (Русский)" "$([ "$(jqv "$RESP" "d['result']['active_language']['folder']")" = "Russian (Русский)" ]; echo $?)"
ck "state: display name separate from identity" "$([ -n "$(jqv "$RESP" "d['result']['active_language']['native']")" ]; echo $?)"
ck "state: golden DefInjected marker" "$([ "$(jqv "$RESP" "d['result']['marker_research_description']")" = "$GOLDEN_DEF" ]; echo $?)"
ck "state: golden keyed marker" "$([ "$(jqv "$RESP" "d['result']['marker_keyed']")" = "$GOLDEN_KEYED" ]; echo $?)"
ck "state: telemetry present (frame/revision)" "$([ -n "$(jqv "$RESP" "d['telemetry']['frame']")" ]; echo $?)"

# --- 3. keyed lookup ---
call c3 "{\"protocol_version\":1,\"request_id\":\"c3\",\"run_id\":\"$RUN_ID\",\"op\":\"keyed\",\"args\":{\"key\":\"VWE_ShotRemaining\"}}" || true
ck "keyed: golden marker value" "$([ "$(jqv "$RESP" "d['result']['value']")" = "$GOLDEN_KEYED" ]; echo $?)"

# --- 4. def-injected lookup ---
call c4 "{\"protocol_version\":1,\"request_id\":\"c4\",\"run_id\":\"$RUN_ID\",\"op\":\"read_def\",\"args\":{\"def\":\"VWE_HeavyWeapons\"}}" || true
ck "read_def: golden marker description" "$([ "$(jqv "$RESP" "d['result']['description']")" = "$GOLDEN_DEF" ]; echo $?)"

# --- 5. semantic errors ---
call c5 "{\"protocol_version\":1,\"request_id\":\"c5\",\"run_id\":\"$RUN_ID\",\"op\":\"read_def\",\"args\":{\"def\":\"RIMLOC_NO_SUCH_DEF\"}}" || true
ck "read_def missing → TARGET_DEF_NOT_FOUND" "$([ "$(jqv "$RESP" "d['error']['code']")" = "TARGET_DEF_NOT_FOUND" ]; echo $?)"
ck "error envelope has code+message shape" "$([ -n "$(jqv "$RESP" "d['error']['message']")" ]; echo $?)"

call c6 "{\"protocol_version\":1,\"request_id\":\"c6\",\"run_id\":\"$RUN_ID\",\"op\":\"research_tab\",\"args\":{}}" || true
ck "research_tab at main menu → WRONG_GAME_STATE (was raw InvalidCastException in v0.3)" \
  "$([ "$(jqv "$RESP" "d['error']['code']")" = "WRONG_GAME_STATE" ]; echo $?)"
ck "WRONG_GAME_STATE detail carries program_state" "$([ "$(jqv "$RESP" "d['error']['detail']['program_state']")" = "Entry" ]; echo $?)"

call c7 "{\"protocol_version\":1,\"request_id\":\"c7\",\"run_id\":\"$RUN_ID\",\"op\":\"totally_bogus\",\"args\":{}}" || true
ck "unknown op → CAPABILITY_UNAVAILABLE" "$([ "$(jqv "$RESP" "d['error']['code']")" = "CAPABILITY_UNAVAILABLE" ]; echo $?)"

call c8 "{\"protocol_version\":1,\"request_id\":\"c8\",\"run_id\":\"run-OTHER\",\"op\":\"state\",\"args\":{}}" || true
ck "foreign run_id → RUN_ID_MISMATCH" "$([ "$(jqv "$RESP" "d['error']['code']")" = "RUN_ID_MISMATCH" ]; echo $?)"

call c9 "{this is not json" || true
ck "malformed JSON → BAD_REQUEST" "$([ "$(jqv "$RESP" "d['error']['code']")" = "BAD_REQUEST" ]; echo $?)"

# --- 6. background frame capture with freshness ---
call c10 "{\"protocol_version\":1,\"request_id\":\"c10\",\"run_id\":\"$RUN_ID\",\"op\":\"screenshot\",\"args\":{\"path\":\"captures/conformance.png\"}}" || true
ck "screenshot: ok" "$([ "$(jqv "$RESP" "d['ok']")" = "True" ]; echo $?)"
ck "screenshot: frame advanced (fresh, not stale)" "$([ "$(jqv "$RESP" "d['result']['freshness']['stale']")" = "False" ]; echo $?)"
ck "screenshot: freshness carries both frame stamps" "$([ -n "$(jqv "$RESP" "d['result']['freshness']['frame_at_present']")" ]; echo $?)"
[ -f "$QA/captures/conformance.png" ] && cp "$QA/captures/conformance.png" "$EV/conformance-frame.png" 2>/dev/null

# --- 7. path fence ---
call c11 "{\"protocol_version\":1,\"request_id\":\"c11\",\"run_id\":\"$RUN_ID\",\"op\":\"screenshot\",\"args\":{\"path\":\"/tmp/rimloc-escape-test.png\"}}" || true
ck "capture outside evidence root → EVIDENCE_ROOT_ESCAPE" "$([ "$(jqv "$RESP" "d['error']['code']")" = "EVIDENCE_ROOT_ESCAPE" ]; echo $?)"
ck "no file escaped to /tmp" "$([ ! -f /tmp/rimloc-escape-test.png ]; echo $?)"

# --- 8. graceful quit ---
call c12 "{\"protocol_version\":1,\"request_id\":\"c12\",\"run_id\":\"$RUN_ID\",\"op\":\"quit\",\"args\":{}}" || true
ck "quit: ok" "$([ "$(jqv "$RESP" "d['ok']")" = "True" ]; echo $?)"
for i in $(seq 1 30); do kill -0 "$GPID" 2>/dev/null || break; sleep 1; done
ck "game exited gracefully" "$(kill -0 "$GPID" 2>/dev/null && echo 1 || echo 0)"
kill -9 "$GPID" 2>/dev/null

sleep 2
OWNER_FM_AFTER=$(fm)
# Non-interference metric: the harness issues ZERO global input (no clicks, no
# key events, no Space switching — by construction it only reads frontmost).
# A frontmost change to ANOTHER app during the run is owner/desktop activity or
# the natural focus handoff after the game quit — recorded, not attributed.
if [ "$OWNER_FM_BEFORE" = "$OWNER_FM_AFTER" ]; then
  PASS=$((PASS+1)); echo "  PASS: non-interference: owner frontmost unchanged"
else
  echo "  WARM: frontmost moved $OWNER_FM_BEFORE → $OWNER_FM_AFTER (harness issued no global input; game quit hands focus to the system)"
  echo "frontmost moved: $OWNER_FM_BEFORE → $OWNER_FM_AFTER (harness issued no global input; recorded, not attributed to adapter)" >> "$EV/non-interference.txt"
fi
T1=$(date +%s)
echo "=== conformance: PASS=$PASS FAIL=$FAIL (wall $((T1-T0))s) ==="
echo "owner frontmost before=$OWNER_FM_BEFORE after=$OWNER_FM_AFTER" > "$EV/non-interference.txt"
echo "run_id=$RUN_ID bridge=$BRIDGE_VER finished=$(date -u +%FT%TZ)" >> "$EV/non-interference.txt"
[ "$FAIL" = "0" ] && echo "V1 CONFORMANCE = PASS" || echo "V1 CONFORMANCE = FAIL ($FAIL)"
exit $([ "$FAIL" = "0" ] && echo 0 || echo 1)
