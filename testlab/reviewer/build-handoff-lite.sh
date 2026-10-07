#!/usr/bin/env bash
# Build the reviewer handoff-lite packet (Part A3) + sanitize it (A4).
# Usage: build-handoff-lite.sh <PRODUCT_SHA> <REASON_FOR_REVIEW> [WHAT_CHANGED_FILE]
# Output: RimLoc-evidence-handoff-lite-<SHA>.zip in RimLoc-evidence/ + REVIEW_REQUEST.md inside.
# The packet contains ONLY review-relevant evidence; binaries stay local.
set -euo pipefail
REPO=/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main
EV=/Users/danielviktorovich/Developing/RimLoc-evidence
PRODUCT_SHA="${1:?usage: build-handoff-lite.sh <PRODUCT_SHA> <REASON> [WHAT_CHANGED_FILE]}"
REASON="${2:?usage: build-handoff-lite.sh <PRODUCT_SHA> <REASON> [WHAT_CHANGED_FILE]}"
WHAT_CHANGED_FILE="${3:-}"

STAMP=$(date +%Y%m%d-%H%M%S)
MAIN_SHA=$(git -C "$REPO" rev-parse origin/main)
WORK="reviewer/packet-lite-$STAMP"
PKT="$EV/$WORK"
rm -rf "$PKT"
mkdir -p "$PKT/evidence"

# --- review-relevant evidence (A3 whitelist) ---
cat > "$PKT/REVIEW_REQUEST.md" <<EOF
[RIMLOC_REVIEW_REQUEST_V1]

MAIN_SHA:
$MAIN_SHA

PRODUCT_SHA:
$PRODUCT_SHA

RC:
rel22-rc (respin history in IDENTITY.md)

REASON_FOR_REVIEW:
$REASON

WHAT_CHANGED:
$(if [ -n "$WHAT_CHANGED_FILE" ] && [ -f "$WHAT_CHANGED_FILE" ]; then cat "$WHAT_CHANGED_FILE"; else echo "- see RELEASE_GATE.md rel22 section and CHANGELOG Unreleased"; fi)

QUESTIONS:
1. Is the release gate coverage honest and sufficient for an RC claim?
2. Any security/data-safety gaps in the zero-open state and its dismissals?
3. Any release-blocking defects in the C-gate outcomes (provider boundary, CodeQL tool status, DMG decision, dependency PR resolutions)?

FILES:
$(basename "$EV")/$(basename "$PKT").zip (this packet)

Please independently audit this state.
Return:
DO_NOW
OWNER_ONLY
BLOCKED
EVIDENCE_REQUEST
VERDICT

[RIMLOC_REVIEW_REQUEST_END]
EOF

cp "$REPO/docs/development/RELEASE_GATE.md" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/docs/competitive/RELEASE_PARITY_MATRIX.md" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/docs/competitive/release-parity.json" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/docs/security/CODE_SCANNING_RECONCILIATION.md" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/docs/security/DEPENDABOT_RECONCILIATION.md" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/docs/security/VENDOR_AUDIT_2026-10.md" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/CHANGELOG.md" "$PKT/evidence/" 2>/dev/null || true
cp "$REPO/.rimloc-release-state.json" "$PKT/evidence/" 2>/dev/null || true
cp "$EV/artifact-rel22-rc/IDENTITY.md" "$PKT/evidence/IDENTITY-rel22.md" 2>/dev/null || true
cp "$EV/artifact-rel22-rc/OWNER_TEST_PACKET.md" "$PKT/evidence/" 2>/dev/null || true
cp "$EV/C1_PROVIDER_BOUNDARY.md" "$PKT/evidence/" 2>/dev/null || true
cp "$EV/artifact-rel22-rc/palette-wdio-results.txt" "$PKT/evidence/" 2>/dev/null || true

# CI summary + CodeQL analyses summary (fresh, from gh)
(cd "$REPO" && gh run list --limit 12 --json workflowName,headSha,status,conclusion,createdAt \
  --jq '.[] | "\(.headSha[0:7]) \(.workflowName): \(.conclusion // .status) (\(.createdAt[0:16]))"' \
  > "$PKT/evidence/ci-summary.txt") 2>/dev/null || true
(gh api "repos/0-danielviktorovich-0/RimLoc/code-scanning/analyses?per_page=6" \
  --jq '.[] | "\(.commit_sha[0:7]) \(.created_at[0:16]) id=\(.id)"' \
  > "$PKT/evidence/codeql-analyses.txt" \
 && gh api "repos/0-danielviktorovich-0/RimLoc/code-scanning/alerts?state=open" \
  --jq 'length' >> "$PKT/evidence/codeql-analyses.txt" \
 && echo "^ open alerts" >> "$PKT/evidence/codeql-analyses.txt") 2>/dev/null || true

# Manifest of the packet contents
( cd "$PKT" && find . -type f -exec shasum -a 256 {} \; | sed 's|\./||' > MANIFEST.sha256 )

# --- A4 sanitizer gate: FAIL CLOSED ---
SAN="$REPO/testlab/reviewer/sanitize-packet.py"
python3 "$SAN" "$PKT"
RC=$?
if [ "$RC" -ne 0 ]; then
  echo "PACKET NOT BUILT: sanitizer refused (fail closed). Dir left at $PKT for redaction."
  exit 1
fi

# --- zip ---
ZIP="$EV/RimLoc-evidence-handoff-lite-${PRODUCT_SHA:0:7}.zip"
( cd "$PKT/.." && ditto -c -k --keepParent "$(basename "$PKT")" "$ZIP" )
echo "PACKET OK: $ZIP"
echo "  files: $(find "$PKT" -type f | wc -l | tr -d ' ')  review request: REVIEW_REQUEST.md"

# --- A11: push to the private evidence mailbox (durable channel) ---
MAILBOX=/tmp/rimloc-mailbox-seed
if [ -d "$MAILBOX/.git" ]; then
  cp "$ZIP" "$MAILBOX/"
  git -C "$MAILBOX" add "$(basename "$ZIP")"
  git -C "$MAILBOX" commit -q -m "handoff-lite ${PRODUCT_SHA:0:7}: sanitized packet ($(find "$PKT" -type f | wc -l | tr -d ' ') files)" || true
  if git -C "$MAILBOX" push -q origin HEAD:main 2>/dev/null; then
    echo "MAILBOX: pushed to RimLoc-review-handoff (private)"
  else
    echo "MAILBOX: push failed (offline?) — ZIP остаётся локально, Finder-fallback покажет"
  fi
else
  echo "MAILBOX: worktree не найден — пропуск (Finder-fallback остаётся каналом)"
fi

echo "$ZIP"
