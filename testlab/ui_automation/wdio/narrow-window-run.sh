#!/bin/bash
# T6 narrow-window lane runner (#161, parent #155) — ONE command.
#
#   bash testlab/ui_automation/wdio/narrow-window-run.sh [path-to-RimLoc GUI.app]
#
# What it does (in order):
#   1. resolves the AUTOMATION artifact (arg-1 > $T6_APP > this tree's
#      target/release/bundle/macos/RimLoc GUI.app) and gates its identity
#      through soak-preflight.sh (never trust the path — soak-hardening §1);
#   2. bootstraps machine-local helpers once: /tmp/place-win (frame math),
#      the pyobjc venv for winframe/axquit/shot-window, /tmp/winframe +/tmp/axquit;
#   3. runs the §11A-F window-state regression (window-state-check.sh);
#   4. runs the embedded-WDIO lane narrow-window.conf.ts: the app is placed
#      at 980x640 by the RIMLOC_WINDOW_FRAME env hook (main.rs, feature
#      automation-bridge), every surface is measured with DOM-rect probes
#      (h-overflow / root-scroll / offscreen / clipped / sibling-overlap),
#      screenshots taken with CGWindowListCreateImage + screencapture -l;
#   5. writes the evidence pack and prints the verdict.
#
# Background-only: zero activations, zero keystrokes (owner P0 rule) —
# the lane drives AX-values, DOM state and env hooks exclusively.
# Env: T6_PORT (default 4475), T6_EVIDENCE (default /tmp/rimloc-t6/evidence),
#      T6_SKIP_WINDOW_CHECK=1 (skip §11).
set -u
REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
PORT="${T6_PORT:-4475}"
EVIDENCE="${T6_EVIDENCE:-/tmp/rimloc-t6/evidence}"
WORK=/tmp/rimloc-t6
mkdir -p "$EVIDENCE" "$WORK/data"

note() { echo "[t6-run] $*"; }
die() { echo "[t6-run] FAIL: $*" >&2; exit 1; }

# --- 1. artifact resolution + identity gate ---------------------------------
APP="${1:-${T6_APP:-$REPO/target/release/bundle/macos/RimLoc GUI.app}}"
[ -d "$APP" ] || die "automation .app not found: $APP (build: bash testlab/build/build-automation-react.sh)"
BIN="$APP/Contents/MacOS/rimloc-gui"
[ -x "$BIN" ] || die "binary not executable: $BIN"
note "artifact: $APP"
note "sha256: $(shasum -a 256 "$BIN" | cut -d' ' -f1)"
bash "$REPO/testlab/ui_automation/wdio/soak-preflight.sh" --bin "$BIN" --class automation --repo "$REPO" \
  --out "$EVIDENCE/preflight.json" || die "artifact identity gate rejected the build (is it the automation bundle?)"

# --- 2. machine-local helpers (idempotent) ----------------------------------
if [ ! -x /tmp/place-win ]; then
  note "building /tmp/place-win"
  bash "$REPO/testlab/ui_automation/bin/build-tools.sh" >/dev/null
fi
PY="${T6_PY:-$WORK/venv/bin/python3}"
if [ ! -x "$PY" ] || ! "$PY" -c "import Quartz, ApplicationServices" 2>/dev/null; then
  note "bootstrapping pyobjc venv at $WORK/venv (one-time)"
  python3 -m venv "$WORK/venv"
  "$WORK/venv/bin/pip" install -q pyobjc-framework-Cocoa pyobjc-framework-Quartz pyobjc-framework-ApplicationServices
  "$PY" -c "import Quartz, ApplicationServices" || die "pyobjc bootstrap failed"
fi
install_helper_shim() { # name src — /tmp/<name> is a shim running <src> with
  # the pyobjc venv python (the repo scripts need Quartz/AX; system python3
  # has no pyobjc — a bare copy with a python3 shebang dies silently and
  # window-state-check.sh reads empty frames, the 06:02 lesson)
  local dst="/tmp/$1" src="$2"
  if [ ! -x "$dst" ] || [ "$src" -nt "$dst" ] || ! grep -qF "$PY" "$dst" 2>/dev/null; then
    printf '#!/bin/bash\nexec %q %q "$@"\n' "$PY" "$src" > "$dst"
    chmod +x "$dst"
    note "installed $dst -> $src"
  fi
}
install_helper_shim winframe "$REPO/testlab/ui_automation/bin/winframe.py"
install_helper_shim axquit "$REPO/testlab/ui_automation/bin/axquit.py"

# --- 3. §11A-F window-state regression --------------------------------------
WCHECK_RC=0
if [ "${T6_SKIP_WINDOW_CHECK:-0}" != "1" ]; then
  # P0 isolation (AGENTS.md test-profile rule, codex review #174): §11's
  # NORMAL launches persist window geometry via tauri-plugin-window-state
  # into app_config_dir, which RIMLOC_DATA_DIR does NOT cover — redirect
  # HOME to a disposable dir for every §11 launch so the owner's saved
  # frame is never overwritten.
  WCHECK_HOME="$(mktemp -d "$WORK/home.XXXXXX")"
  export WCHECK_HOME
  export RIMLOC_DATA_DIR="$WORK/data"   # managed-root isolation for §11 too
  note "running window-state-check.sh §11A-F (isolated HOME: $WCHECK_HOME)"
  bash "$REPO/testlab/ui_automation/wdio/window-state-check.sh" "$BIN" \
    2>&1 | tee "$EVIDENCE/window-state-check.log" >/dev/null
  WCHECK_RC=${PIPESTATUS[0]}
  tail -3 "$EVIDENCE/window-state-check.log" | sed 's/^/[t6-run] wcheck /'
  [ "$WCHECK_RC" -eq 0 ] || note "WARNING: window-state §11 reported failures (see log)"
fi

# --- 4. WDIO narrow-window lane ---------------------------------------------
export T6_BIN="$BIN"
export T6_PIDFILE="$WORK/app.pid"
export T6_EVIDENCE="$EVIDENCE"
export T6_PY="$PY"
export T6_PORT="$PORT"
export T6_DATA_DIR="$WORK/data"   # isolated projects universe (owner §4)
# The wrapper resolves from THIS checkout (codex review #174) — never a
# hard-coded author-worktree path; the conf validates the absolute path.
export T6_SPAWN_WRAPPER="$REPO/testlab/ui_automation/wdio/spawn-wrapper-t6.sh"
note "running WDIO lane (port $PORT)"
cd "$REPO/gui/tauri-app/frontend-v2"
# Capture WDIO's status BEFORE the optional log filter can mask it
# (codex review #174): no `|| true` before reading PIPESTATUS — an early
# WDIO crash must fail the lane, not print zero. The filter's own rc is
# irrelevant by design (nothing to show = empty summary, not a failure).
npx wdio run e2e/wdio-spike/narrow-window.conf.ts 2>&1 | tee "$EVIDENCE/wdio-run.log" | grep -E '√|×|FAIL|SUMMARY|spec' > "$EVIDENCE/wdio-summary.txt"
WDIO_RC=${PIPESTATUS[0]}
cd "$REPO"

# --- 5. verdict --------------------------------------------------------------
echo
note "evidence: $EVIDENCE (report.json, *.png, preflight.json, logs)"
if [ "$WDIO_RC" -eq 0 ] && [ "$WCHECK_RC" -eq 0 ]; then
  note "PASS: narrow-window lane + window-state §11"
  exit 0
fi
note "FAIL: wdio=$WDIO_RC window-state=$WCHECK_RC — see $EVIDENCE"
exit 1
