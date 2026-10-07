#!/bin/bash
# Формальный macOS-артефакт: .app → .app.zip + sha256.
#
# Контракт поставки (fix/dmg-stage, 2026-10-07): macOS artifact = .app + .app.zip;
# DMG-стадия НЕ запускается — hdiutil не может создавать образы на NFS-томе
# Portable-SSD («create failed - Файл существует», 0-байтные rw.*.dmg, rel19–rel22).
# Подробности и первоисточник диагноза — docs/development/RELEASE_GATE.md.
#
# Usage: build-make-appzip.sh <path/to/App.app> [output-dir]
# Output: <output-dir>/App.app.zip и <output-dir>/App.app.zip.sha256
set -euo pipefail

APP_PATH="${1:?usage: build-make-appzip.sh <path/to/App.app> [output-dir]}"
OUT_DIR="${2:-$(dirname "$APP_PATH")}"

if [[ ! -d "$APP_PATH" ]]; then
  echo "ERROR: .app not found: $APP_PATH" >&2
  exit 1
fi

APP_NAME="$(basename "$APP_PATH")" # напр. "RimLoc GUI.app"
ZIP_PATH="$OUT_DIR/$APP_NAME.zip"

mkdir -p "$OUT_DIR"

# Стейджинг на локальном томе: с NFS Portable-SSD .app приезжает с AppleDouble-«._»
# сиблингами (backing xattr com.apple.provenance) — в поставку они не должны попасть.
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/rimloc-appzip.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT

ditto "$APP_PATH" "$STAGE/$APP_NAME"
find "$STAGE/$APP_NAME" \( -name '._*' -o -name '.DS_Store' \) -delete
xattr -cr "$STAGE/$APP_NAME" 2>/dev/null || true

rm -f "$ZIP_PATH"
ditto -c -k --norsrc --keepParent "$STAGE/$APP_NAME" "$ZIP_PATH"

shasum -a 256 "$ZIP_PATH" > "$ZIP_PATH.sha256"
echo "OK: $ZIP_PATH"
cat "$ZIP_PATH.sha256"
