#!/usr/bin/env bash
# Release gate: the production bundle must embed the React R1 frontend
# (frontend-react dist assets), never the frozen Svelte fallback.
# Fail closed. No -q/pipe pipelines (SIGPIPE turns a match into a mismatch
# under pipefail — the exact bug this gate originally had).
# Usage: release-gate-frontend.sh <path-to-rimloc-gui-binary>
set -euo pipefail
BIN="${1:?usage: release-gate-frontend.sh <binary>}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REACT_DIST="$REPO_ROOT/gui/tauri-app/frontend-react/dist"

fail() { echo "FRONTEND GATE: FAIL — $1" >&2; exit 1; }
[ -f "$BIN" ] || fail "binary not found: $BIN"
[ -f "$REACT_DIST/index.html" ] || fail "frontend-react/dist missing — build the React frontend first"

# Read the printable-string corpus ONCE; all checks are bash pattern matches
# against this snapshot (no pipes => no SIGPIPE misreads).
EMBEDDED=$(strings "$BIN")

JS_ASSET=$(ls "$REACT_DIST"/assets/index-*.js 2>/dev/null | head -1 | xargs -n1 basename || true)
CSS_ASSET=$(ls "$REACT_DIST"/assets/index-*.css 2>/dev/null | head -1 | xargs -n1 basename || true)
[ -n "$JS_ASSET" ] || fail "no index-*.js in frontend-react/dist"

[[ "$EMBEDDED" == *"$JS_ASSET"* ]] || fail "React js asset $JS_ASSET not embedded in binary"
if [ -n "$CSS_ASSET" ]; then
  [[ "$EMBEDDED" == *"$CSS_ASSET"* ]] || fail "React css asset $CSS_ASSET not embedded in binary"
fi

# The frozen Svelte fallback must NOT be the embedded frontend.
SVELTE_DIST="$REPO_ROOT/gui/tauri-app/frontend-v2/dist"
if [ -d "$SVELTE_DIST" ]; then
  while IFS= read -r sv; do
    [ -z "$sv" ] && continue
    if [[ "$EMBEDDED" == *"$sv"* ]]; then
      fail "Svelte fallback asset $sv embedded — wrong frontend flavor"
    fi
  done < <(find "$SVELTE_DIST/assets" -name 'index-*.js' -exec basename {} \; 2>/dev/null)
fi
if [[ "$EMBEDDED" == *"svelte"* ]]; then
  fail "svelte markers found in binary"
fi

echo "FRONTEND GATE: PASS — React R1 frontend embedded (assets $JS_ASSET), no svelte fallback markers"
