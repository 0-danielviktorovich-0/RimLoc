#!/bin/bash
# AUTOMATION artifact (§C): feature ON + bridge capability overlay +
# guest-JS chunk emitted (VITE_RIMLOC_AUTOMATION=1). NOT shippable.
set -u
export VITE_RIMLOC_AUTOMATION=1
cd "$(dirname "$0")/../../gui/tauri-app/src-tauri"
cp automation-capability.json capabilities/automation.json
exec cargo tauri build --features automation-bridge --config tauri.automation.conf.json
