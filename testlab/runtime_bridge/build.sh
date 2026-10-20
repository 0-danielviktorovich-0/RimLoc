#!/bin/bash
# Build the RimWorld Runtime Bridge against a game's Managed references.
#
# The legacy T6 clone defaults are GONE: the caller names the game and the
# destination explicitly (WP1 decoupling — the bridge must build against
# any licensed local RimWorld install without a RimLoc checkout layout).
#
# Usage:
#   GAME_MANAGED=/path/.../Data/Managed bash build.sh
#   GAME_MANAGED=... INSTALL_DIR=/path/Mods/RimLocQABridge bash build.sh --install
#   GAME_MANAGED=... bash build.sh --package /path/out           # staged ZIP
# The --install path is additive: it places THIS bridge's assembly and
# never deletes other assemblies (the old QaDriver replacement is
# retired; golden fixtures live in their own mod folders).
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
GAME_MANAGED="${GAME_MANAGED:-}"
INSTALL_DIR="${INSTALL_DIR:-}"
PACKAGE_DIR="${PACKAGE_DIR:-}"

usage() { grep '^#' "$0" | sed 's/^# \{0,1\}//' >&2; }

[ -n "$GAME_MANAGED" ] && [ -d "$GAME_MANAGED" ] || {
  echo "GAME_MANAGED not set or not a directory (point it at a licensed RimWorld install's Data/Managed)" >&2
  usage
  exit 2
}

cd "$DIR"
dotnet build RuntimeBridge.csproj -c Release -p:GameManaged="$GAME_MANAGED" -v q --nologo
BIN="bin/Release/RimWorldRuntimeBridge.dll"
[ -f "$BIN" ] || { echo "build produced no dll" >&2; exit 1; }

# Net48 facade nuget may pull reference assemblies only; verify the dll is a real assembly
python3 - "$BIN" <<'PY'
import sys
p = sys.argv[1]
with open(p, 'rb') as f:
    head = f.read(2)
assert head == b'MZ', f"not a PE: {p}"
print(f"OK {p}: PE header valid, {__import__('os').path.getsize(p)} bytes")
PY

MODE="${1:-build}"
case "$MODE" in
  --install)
    [ -n "$INSTALL_DIR" ] || { echo "INSTALL_DIR not set" >&2; usage; exit 2; }
    mkdir -p "$INSTALL_DIR/Assemblies"
    cp "$BIN" "$INSTALL_DIR/Assemblies/"
    # Mod metadata: written when absent, never overwritten (an existing
    # About.xml belongs to the destination's owner).
    if [ ! -f "$INSTALL_DIR/About/About.xml" ]; then
      mkdir -p "$INSTALL_DIR/About"
      cat > "$INSTALL_DIR/About/About.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<ModMetaData>
  <name>RimLoc QA Bridge</name>
  <author>RimLoc contributors</author>
  <packageId>rimloc.qabridge</packageId>
  <supportedVersions><li>1.6</li></supportedVersions>
  <description>File-channel runtime bridge for automated RimWorld QA. Test tooling, not gameplay content; never ships proprietary game files.</description>
</ModMetaData>
XML
    fi
    echo "installed: $INSTALL_DIR/Assemblies/RimWorldRuntimeBridge.dll"
    ;;
  --package)
    [ -n "$PACKAGE_DIR" ] || { echo "PACKAGE_DIR not set" >&2; usage; exit 2; }
    STAGE="$(mktemp -d)/RimLocQABridge"
    mkdir -p "$STAGE/About" "$STAGE/Assemblies"
    cp "$BIN" "$STAGE/Assemblies/"
    cp "$DIR/About-template.xml" "$STAGE/About/About.xml" 2>/dev/null || \
      echo "About-template.xml missing — package without About" >&2
    cp "$DIR/LICENSE" "$STAGE/" 2>/dev/null || true
    cp "$DIR/README-standalone.md" "$STAGE/README.md" 2>/dev/null || true
    mkdir -p "$PACKAGE_DIR"
    ZIP="$PACKAGE_DIR/RimLocQABridge-$(date +%Y%m%d-%H%M%S).zip"
    (cd "$(dirname "$STAGE")" && zip -qr "$ZIP" "$(basename "$STAGE")")
    SHASUM="$(shasum -a 256 "$ZIP" | cut -d' ' -f1)"
    echo "package: $ZIP"
    echo "sha256: $SHASUM"
    ;;
  build|"")
    echo "built: $BIN (no install/package requested)"
    ;;
  *)
    echo "unknown mode: $MODE" >&2; usage; exit 2
    ;;
esac
