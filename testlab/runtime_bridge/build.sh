#!/bin/bash
# Build the RimWorld Runtime Bridge (test-only mod assembly) against a game
# clone's Managed references and optionally install it into the clone mod.
#
# Usage:
#   bash build.sh                       # build with default clone refs
#   bash build.sh --install [mod-dir]   # build + install into clone mod
#                                        # (default: ../run/t6/.../Mods/RimLocT6-QADriver)
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
GAME_MANAGED="${GAME_MANAGED:-$DIR/../run/t6/RimWorldTest.app/Contents/Resources/Data/Managed}"
INSTALL_DIR="${2:-$DIR/../run/t6/RimWorldTest.app/Mods/RimLocT6-QADriver}"

[ -d "$GAME_MANAGED" ] || { echo "game Managed dir not found: $GAME_MANAGED" >&2; exit 2; }

cd "$DIR"
dotnet build RuntimeBridge.csproj -c Release -p:GameManaged="$GAME_MANAGED" -v q --nologo
BIN="bin/Release/RimWorldRuntimeBridge.dll"
[ -f "$BIN" ] || { echo "build produced no dll" >&2; exit 1; }

# Net48 facade nuget may pull reference assemblies only; verify the dll is a real assembly
python3 - "$BIN" <<'PY'
import sys, struct
p = sys.argv[1]
with open(p, 'rb') as f:
    head = f.read(2)
assert head == b'MZ', f"not a PE: {p}"
print(f"OK {p}: PE header valid, {__import__('os').path.getsize(p)} bytes")
PY

if [ "${1:-}" = "--install" ]; then
  mkdir -p "$INSTALL_DIR/Assemblies"
  # v1 replaces the v0.3 QaDriverV3 assembly one-for-one; packageId unchanged,
  # marker package (RimLocT6-Package) is NOT touched (golden fixture).
  rm -f "$INSTALL_DIR/Assemblies/QaDriverV3.dll" "$INSTALL_DIR/Assemblies/QaDriver.dll"
  cp "$BIN" "$INSTALL_DIR/Assemblies/"
  echo "installed: $INSTALL_DIR/Assemblies/RimWorldRuntimeBridge.dll"
fi
