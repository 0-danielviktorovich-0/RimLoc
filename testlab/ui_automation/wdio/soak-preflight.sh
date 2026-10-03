#!/bin/bash
# Soak pre-flight artifact-identity gate (owner soak-hardening §1/§2, 03.10).
# A long-running acceptance run must NEVER silently drive the wrong artifact:
# before cycle 1 this gate verifies the binary on disk AND, after spawn, the
# running process — fail-closed on any mismatch.
#
# Usage:
#   soak-preflight.sh --bin <binary> --class automation|production \
#                     [--expect-sha256 <sha>] [--expect-commit <sha>] [--gate]
#   soak-preflight.sh --selftest
#
# --gate: STRICT release-gate mode — --bin and --expect-sha256 are REQUIRED
# (a gate run may not fall back to any historical default artifact).
# Output: preflight.json next to the run dir (caller passes --out <file>).
set -u
BIN=""; CLASS=""; EXPECT_SHA=""; EXPECT_COMMIT=""; GATE=0; OUT=""; SELFTEST=0
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN="$2"; shift 2;;
    --class) CLASS="$2"; shift 2;;
    --expect-sha256) EXPECT_SHA="$2"; shift 2;;
    --expect-commit) EXPECT_COMMIT="$2"; shift 2;;
    --gate) GATE=1; shift;;
    --out) OUT="$2"; shift 2;;
    --selftest) SELFTEST=1; shift;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done

fail() { echo "[preflight] FAIL: $*" >&2; exit 1; }

if [ "$SELFTEST" = "1" ]; then
  # --- §2 wrapper precedence: arg1 > $BIN > default (three paths) ---
  W="$(cd "$(dirname "$0")" && pwd)/spawn-wrapper-release.sh"
  P1=$(BIN="/env/override/path" WRAP_PRINT_ONLY=1 bash "$W" /arg1/path)
  [ "$P1" = "/arg1/path" ] || fail "wrapper precedence: arg1 must win (got $P1)"
  P2=$(BIN="/env/override/path" WRAP_PRINT_ONLY=1 bash "$W")
  [ "$P2" = "/env/override/path" ] || fail "wrapper precedence: \$BIN second (got $P2)"
  P3=$(WRAP_PRINT_ONLY=1 bash "$W")
  case "$P3" in
    *artifact-rel14-automation*) :;;
    *) fail "wrapper precedence: rel14 default third (got $P3)";;
  esac
  echo "[preflight] ok: wrapper precedence arg1 > \$BIN > default"

  # --- §1 fail-closed regressions: wrong sha / wrong class / missing file ---
  TMP=$(mktemp -d)
  printf 'not-an-app-binary' > "$TMP/fake"
  chmod +x "$TMP/fake"
  out=$("$0" --bin "$TMP/fake" --class automation --expect-sha256 "0000deadbeef" 2>&1)
  [ $? -ne 0 ] || fail "wrong expected sha must abort"
  echo "$out" | grep -q "sha256 mismatch" || fail "wrong-sha abort must name the cause"
  printf 'binary-with-wdio-webdriver-marker' > "$TMP/fake2"
  chmod +x "$TMP/fake2"
  out=$("$0" --bin "$TMP/fake2" --class production 2>&1)
  [ $? -ne 0 ] || fail "production artifact carrying automation surface must abort"
  echo "$out" | grep -q "automation surface in a production-class artifact" || fail "class abort must name the cause"
  out=$("$0" --bin "$TMP/does-not-exist" --class automation 2>&1)
  [ $? -ne 0 ] || fail "missing binary must abort"
  # gate mode: missing explicit --expect-sha256 fails closed
  out=$("$0" --bin "$TMP/fake" --class automation --gate 2>&1)
  [ $? -ne 0 ] || fail "gate mode without --expect-sha256 must abort"
  echo "$out" | grep -q "gate mode requires" || fail "gate abort must name the requirement"
  rm -rf "$TMP"
  echo "[preflight] SELFTEST PASS (fail-closed + wrapper precedence)"
  exit 0
fi

[ -n "$BIN" ] || fail "missing --bin"
[ -n "$CLASS" ] || fail "missing --class"
[ "$CLASS" = "automation" ] || [ "$CLASS" = "production" ] || fail "class must be automation|production"
[ -x "$BIN" ] || fail "binary missing or not executable: $BIN"

if [ "$GATE" = "1" ] && [ -z "$EXPECT_SHA" ]; then
  fail "gate mode requires --expect-sha256 (a release gate never trusts a historical default)"
fi

ACTUAL_SHA=$(shasum -a 256 "$BIN" | awk '{print $1}')
if [ -n "$EXPECT_SHA" ] && [ "$ACTUAL_SHA" != "$EXPECT_SHA" ]; then
  fail "sha256 mismatch: expected $EXPECT_SHA, actual $ACTUAL_SHA for $BIN"
fi

# Artifact class by automation surface (independent of any path naming).
if strings "$BIN" 2>/dev/null | grep -q "wdio-webdriver"; then
  SURFACE="automation"
else
  SURFACE="production"
fi
if [ "$CLASS" = "production" ] && [ "$SURFACE" = "automation" ]; then
  fail "automation surface in a production-class artifact: $BIN (soak/E2E must never target production, §C)"
fi
if [ "$CLASS" = "automation" ] && [ "$SURFACE" != "automation" ]; then
  fail "no automation surface in an automation-class artifact: $BIN (bridge missing — WDIO cannot drive it)"
fi

JSON="{\"binary\":\"$BIN\",\"sha256\":\"$ACTUAL_SHA\",\"class\":\"$CLASS\",\"surface\":\"$SURFACE\""
if [ -n "$EXPECT_COMMIT" ]; then JSON="$JSON,\"expected_commit\":\"$EXPECT_COMMIT\""; fi
JSON="$JSON}"
if [ -n "$OUT" ]; then
  printf '%s\n' "$JSON" > "$OUT"
fi
echo "[preflight] ok: $JSON"
exit 0
