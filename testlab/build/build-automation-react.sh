#!/bin/bash
# AUTOMATION artifact, REACT lane (rel26 draft-safety native E2E): the
# automation overlay carries the embedded WebDriver plugin, the production
# tauri.react.conf.json carries the React R1 frontend — both merge over the
# default base on the tauri CLI command line, so WDIO drives the SAME
# frontend the release ships. NOT shippable (feature ON + capability).
# DMG stage is NOT invoked (NFS contract: --bundles app).
set -u
export VITE_RIMLOC_AUTOMATION=1
cd "$(dirname "$0")/../../gui/tauri-app/src-tauri"
cp automation-capability.json capabilities/automation.json
exec cargo tauri build --features automation-bridge --bundles app \
  --config tauri.react.conf.json \
  --config '{"app":{"security":{"capabilities":["default","automation-bridge"]}}}'
