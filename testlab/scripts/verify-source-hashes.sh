#!/usr/bin/env bash
# READ-ONLY GUARANTEE: verify that canonical source mods are unmodified.
# Baseline: testlab/manifests/source-mods-hashes.sha256 (regenerate with --update).
# Exit 0 = unchanged; exit 1 = MISMATCH (something wrote into a source mod!).
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
HASHES="$HERE/manifests/source-mods-hashes.sha256"
MODS="/Applications/RimWorld.app/Mods"
VANILLA_TAR="/Applications/RimWorld.app/Data/Core/Languages/Russian (Русский).tar"
TARGETS=(1814383360 2927850179 2126925929)

if [ "${1:-}" = "--update" ]; then
  : > "$HASHES"
  for t in "${TARGETS[@]}"; do
    (cd "$MODS" && find "$t" -type f -not -name '.DS_Store' -print0 | sort -z |
      xargs -0 shasum -a 256) >> "$HASHES"
  done
  shasum -a 256 "$VANILLA_TAR" >> "$HASHES"
  echo "baseline written: $(wc -l < "$HASHES" | tr -d ' ') files"
  exit 0
fi

[ -f "$HASHES" ] || { echo "no baseline; run with --update first"; exit 2; }
cd "$MODS"
if shasum -a 256 -c "$HASHES" --quiet 2>/dev/null; then
  echo "OK: source mods unchanged ($(grep -c . "$HASHES") files verified)"
else
  echo "MISMATCH DETECTED — source mods were modified!"
  shasum -a 256 -c "$HASHES" 2>&1 | grep -v ': OK' | head -20
  exit 1
fi
