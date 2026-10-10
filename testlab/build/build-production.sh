#!/bin/bash
# PRODUCTION artifact (§C): no automation feature, no VITE automation env,
# automation capability file absent (build scripts manage its presence —
# Tauri resolves every file in capabilities/).
#
# macOS-артефакт (fix/dmg-stage, 2026-10-07): .app + .app.zip. DMG-стадия НЕ
# запускается: hdiutil не может создавать образы на NFS-томе Portable-SSD
# («create failed - Файл существует», 0-байтные rw.*.dmg — воспроизводилось
# с rel19), поэтому бандл-таргеты ограничены `--bundles app`, а дистрибутивный
# zip собирает scripts/build-make-appzip.sh. Диагноз — docs/development/RELEASE_GATE.md.
set -eu
unset VITE_RIMLOC_AUTOMATION
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT/gui/tauri-app/src-tauri"
rm -f capabilities/automation.json

TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"

# NFS Portable-SSD: у каждого файла, записанного из user-сессии, NFS-клиент
# держит AppleDouble «._»-сиблинга (backing xattr). Новые файлы отфильтровывает
# vendored tauri-utils (патч «._»-гло́ба), старый мусор из build out-каталогов
# снимаем здесь, чтобы сборка не зависела от истории таргета.
if [[ -d "$TARGET_DIR/release/build" ]]; then
  find "$TARGET_DIR/release/build" -name '._*' -delete 2>/dev/null || true
fi

# Production frontend = React R1 (frontend-react). Явный конфиг обязателен:
# без него дефолтный tauri.conf.json собрал бы замороженный Svelte-фолбэк
# из frontend-v2. release-gate-frontend.sh после сборки доказывает flavor.
cargo tauri build --bundles app --config tauri.react.conf.json

# Release gate (R4 §1): prove the React R1 frontend is what shipped.
"$REPO_ROOT/testlab/release-gate-frontend.sh" "$TARGET_DIR/release/bundle/macos/RimLoc GUI.app/Contents/MacOS/rimloc-gui"

# Дистрибутивный артефакт — последний шаг контракта: .app → .app.zip + sha256.
"$REPO_ROOT/scripts/build-make-appzip.sh" \
  "$TARGET_DIR/release/bundle/macos/RimLoc GUI.app" \
  "$TARGET_DIR/release/bundle/appzip"
