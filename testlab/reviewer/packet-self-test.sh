#!/usr/bin/env bash
# Packet self-test + mini-proof (Reviewer iteration 3): доказательства
# считаются с КОНЕЧНОГО артефакта (распаковка ZIP), не из рабочей директории.
# Usage: packet-self-test.sh <ZIP> <PKT_DIR>
set -euo pipefail
ZIP="${1:?usage: packet-self-test.sh <ZIP> <PKT_DIR>}"
PKT="${2:?usage: packet-self-test.sh <ZIP> <PKT_DIR>}"
REPO=/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main

EXTRACT=$(mktemp -d)
unzip -q "$ZIP" -d "$EXTRACT"
INNER=$(find "$EXTRACT" -maxdepth 1 -mindepth 1 -type d | head -1)

APPLED=$(find "$INNER" -name '._*' | wc -l | tr -d ' ')
DSSTORE=$(find "$INNER" -name '.DS_Store' | wc -l | tr -d ' ')
MACOSX=$(unzip -l "$ZIP" | grep -c "__MACOSX" || true)
STALE=$(find "$INNER" -maxdepth 2 \( -iname '*rel21*' -o -iname '*rel22*' \) | wc -l | tr -d ' ')

# MANIFEST verify: каждый entry сверяется с распакованным файлом
cd "$INNER"
MANI_FAIL=0
while IFS= read -r line; do
  [ -z "$line" ] && continue
  h=$(echo "$line" | awk '{print $1}')
  f=$(echo "$line" | sed 's|^[a-f0-9]*  ||')
  actual=$(shasum -a 256 "$f" 2>/dev/null | awk '{print $1}')
  [ "$actual" = "$h" ] || { echo "MANIFEST MISMATCH: $f"; MANI_FAIL=1; }
done < MANIFEST.sha256
MANI=$([ "$MANI_FAIL" -eq 0 ] && echo PASS || echo FAIL)

RC=$(grep -m1 -o "rel23-final" OWNER_TEST_PACKET.md || echo NOT-FOUND)
WDIO=$(grep -m1 -o "WDIO 59/59" OWNER_TEST_PACKET.md || echo NOT-FOUND)
PROD=$(grep -m1 -o "857b0d2a51b97e6e541cdf8f61d00683b966bd04" FINAL_IDENTITY.md | head -1)
HEAD_AT=$(git -C "$REPO" rev-parse HEAD)

cat <<EOF
RC = $RC
product_source_sha = ${PROD:-NOT-FOUND}
repo_head_at_packaging = $HEAD_AT
MANIFEST = $MANI
AppleDouble = $APPLED
DS_Store = $DSSTORE
__MACOSX = $MACOSX
stale top-level rel21/rel22 = $STALE
OWNER_TEST_PACKET RC = $RC
OWNER_TEST_PACKET WDIO = $WDIO
EOF

rm -rf "$EXTRACT"
# Гейт: всё должно быть зелёным
[ "$MANI" = "PASS" ] && [ "$APPLED" = "0" ] && [ "$DSSTORE" = "0" ] && \
  [ "$MACOSX" = "0" ] && [ "$STALE" = "0" ] && [ "$RC" = "rel23-final" ] && \
  [ "$WDIO" = "WDIO 59/59" ] && [ -n "$PROD" ] \
  && echo "SELF-TEST: PASS" || { echo "SELF-TEST: FAIL"; exit 1; }
