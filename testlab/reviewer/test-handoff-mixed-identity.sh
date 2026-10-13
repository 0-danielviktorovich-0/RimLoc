#!/usr/bin/env bash
# Regression for the packet identity checker (rel25 audit P1): a packet with
# intentionally MIXED release identities must FAIL the checker, and a clean
# packet must PASS. Both outcomes are asserted — the checker is only real
# when it refuses.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
CHECK="$HERE/check-packet-identity.py"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

RC="rel26-draft-fixes"
PRODUCT="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
BINARY="1111111111111111111111111111111111111111111111111111111111111111"
APPZIP="2222222222222222222222222222222222222222222222222222222222222222"
STALE="7781dd39,rel23-final,2d0f1652"

fail() { echo "REGRESSION FAIL: $*" >&2; exit 1; }

make_clean_packet() {
  local d="$1"
  mkdir -p "$d"
  cat > "$d/REVIEW_REQUEST.md" <<EOF
MAIN_SHA: $(printf 'p%.0s' {1..40})
PRODUCT_SHA: $PRODUCT
BINARY: ${BINARY:0:7}
RC: $RC
EOF
  cat > "$d/FINAL_IDENTITY.md" <<EOF
# Final identity — $RC
product source: $PRODUCT
binary sha256: $BINARY
appzip sha256: $APPZIP
EOF
  printf '# Release gate — %s\n' "$RC" > "$d/RELEASE_GATE.md"
  cat > "$d/artifact-IDENTITY.md" <<EOF
artifact identity: $RC product $PRODUCT binary ${BINARY:0:7}
EOF
  printf 'owner test packet for %s (binary %s)\n' "$RC" "${BINARY:0:7}" > "$d/OWNER_TEST_PACKET.md"
  echo "$BINARY" > "$d/binary.sha256"
  echo "$APPZIP" > "$d/appzip.sha256"
}

# 1. Clean packet → PASS.
make_clean_packet "$TMP/clean"
python3 "$CHECK" "$TMP/clean" --rc "$RC" --product-sha "$PRODUCT" \
  --binary-sha "$BINARY" --appzip-sha "$APPZIP" --stale "$STALE" > "$TMP/out1" \
  || fail "clean packet refused: $(cat "$TMP/out1")"
grep -q "PASS" "$TMP/out1" || fail "clean packet did not print PASS"

# 2. Mixed identities — stale rel23 marker inside FINAL_IDENTITY → FAIL.
make_clean_packet "$TMP/mixed-marker"
printf '\nhistorical note: supersedes rel23-final\n' >> "$TMP/mixed-marker/FINAL_IDENTITY.md"
python3 "$CHECK" "$TMP/mixed-marker" --rc "$RC" --product-sha "$PRODUCT" \
  --binary-sha "$BINARY" --appzip-sha "$APPZIP" --stale "$STALE" > "$TMP/out2" 2>&1 \
  && fail "stale marker in FINAL_IDENTITY was ACCEPTED"
grep -q "STALE marker 'rel23-final'" "$TMP/out2" || fail "wrong refusal: $(cat "$TMP/out2")"

# 3. Stale BINARY sha in the sha file → FAIL.
make_clean_packet "$TMP/mixed-sha"
echo "7781dd395b8e7c83332da9590a92972e6e93c285cef717200c9c3117471ae4bf" > "$TMP/mixed-sha/binary.sha256"
python3 "$CHECK" "$TMP/mixed-sha" --rc "$RC" --product-sha "$PRODUCT" \
  --binary-sha "$BINARY" --appzip-sha "$APPZIP" --stale "$STALE" > "$TMP/out3" 2>&1 \
  && fail "stale binary.sha256 was ACCEPTED"

# 4. Missing mandatory evidence → FAIL (no silent || true).
make_clean_packet "$TMP/missing"
rm "$TMP/missing/OWNER_TEST_PACKET.md"
python3 "$CHECK" "$TMP/missing" --rc "$RC" --product-sha "$PRODUCT" \
  --binary-sha "$BINARY" --appzip-sha "$APPZIP" --stale "$STALE" > "$TMP/out4" 2>&1 \
  && fail "missing mandatory file was ACCEPTED"
grep -q "MANDATORY MISSING: OWNER_TEST_PACKET.md" "$TMP/out4" || fail "wrong refusal: $(cat "$TMP/out4")"

# 5. Identity document without the CURRENT release id → FAIL.
make_clean_packet "$TMP/stale-rc"
printf '# Release gate — rel23-final\n' > "$TMP/stale-rc/RELEASE_GATE.md"
python3 "$CHECK" "$TMP/stale-rc" --rc "$RC" --product-sha "$PRODUCT" \
  --binary-sha "$BINARY" --appzip-sha "$APPZIP" --stale "$STALE" > "$TMP/out5" 2>&1 \
  && fail "stale RELEASE_GATE.md was ACCEPTED"
grep -q "STALE marker 'rel23-final'" "$TMP/out5" || fail "wrong refusal: $(cat "$TMP/out5")"

echo "REGRESSION OK: clean=PASS, mixed-marker=FAIL, mixed-sha=FAIL, missing=FAIL, stale-rc=FAIL"
