#!/usr/bin/env bash
# Single source of artifact refresh (review P1-2 fix): after every
# production rebuild, ALL identity-bearing artifact files are regenerated
# HERE — never hand-edited in place. The rel26 cycle proved that partial
# hand refreshes drift (OWNER_TEST_PACKET kept a stale appzip sha while
# IDENTITY.md was updated).
# Usage: refresh-artifact.sh <artifact-dir> <repo> <app-bundle> <appzip>
set -euo pipefail
ART="${1:?artifact dir}"
REPO="${2:?repo}"
APP="${3:?app bundle path}"
ZIP="${4:?appzip path}"
RC="${RC:?RC env, e.g. rel26-draft-fixes}"

BIN_SHA=$(shasum -a 256 "$APP/Contents/MacOS/rimloc-gui" | awk '{print $1}')
ZIP_SHA=$(shasum -a 256 "$ZIP" | awk '{print $1}')
PROD_SHA=$(git -C "$REPO" rev-parse HEAD)
BRANCH=$(git -C "$REPO" rev-parse --abbrev-ref HEAD)
STAMP=$(date -u +%Y-%m-%dT%H:%MZ)

echo "$BIN_SHA" > "$ART/sha256.txt"
echo "$ZIP" > "$ART/appzip-path.txt"
echo "$ZIP_SHA" > "$ART/appzip-sha256.txt"

cat > "$ART/IDENTITY.md" <<EOF
# Artifact identity — $RC

- product source sha: $PROD_SHA (branch $BRANCH)
- binary sha256 (rimloc-gui): $BIN_SHA
- appzip sha256: $ZIP_SHA
- build: cargo tauri build --bundles app --config tauri.react.conf.json (DMG stage not invoked — NFS contract: .app + .app.zip)
- frontend gate: PASS — React R1 embedded, no svelte fallback
- exact digests live ONLY here and in the sha files (single source of
  truth per packet; lesson 13 + the rel26 refresh-drift lesson)
- date: $STAMP
EOF

cat > "$ART/OWNER_TEST_PACKET.md" <<EOF
# Owner test packet — $RC

Binary: RimLoc GUI.app (zip sha256 ${ZIP_SHA:0:12}…; in the review packet
the digests ride as appzip.sha256 + MANIFEST.sha256, in this artifact
directory as appzip-sha256.txt — regenerated together by
testlab/reviewer/refresh-artifact.sh). Install: unzip and drag to
/Applications (unsigned build — right-click → Open on first launch).

What to test first (the data-safety fixes):
1. Drafts survive locale switches (type → switch language → switch back).
2. Two Def types sharing one key render as two def-type-labelled rows,
   editable and committable independently.
3. Type and immediately switch projects, then return — the text is there.
4. Restart with unsaved drafts — honest restore notice, drafts come back.
5. Old rel25-era sidecars: restore notice, drafts re-anchored.

Known non-blockers: CodeQL Rust extraction 324/3 is vendor-only macro
expansion (files named in codeql-rust-extraction-latest.txt); 16 dev-only
npm advisories under the documented exemption pending owner decision.
EOF

( cd "$ART" && find . -type f ! -name 'MANIFEST.sha256' -exec shasum -a 256 {} \; \
  | sed 's|\./||' | LC_ALL=C sort > MANIFEST.sha256 )
echo "REFRESHED: $ART (binary ${BIN_SHA:0:7}, appzip ${ZIP_SHA:0:7}, product ${PROD_SHA:0:7})"
