#!/bin/bash
# PRODUCTION artifact (§C): no automation feature, no VITE automation env,
# automation capability file absent (build scripts manage its presence —
# Tauri resolves every file in capabilities/).
set -u
unset VITE_RIMLOC_AUTOMATION
cd "$(dirname "$0")/../../gui/tauri-app/src-tauri"
rm -f capabilities/automation.json
exec cargo tauri build
