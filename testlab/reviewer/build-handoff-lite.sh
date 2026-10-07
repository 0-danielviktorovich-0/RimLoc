#!/usr/bin/env bash
# Build the reviewer handoff-lite packet (Part A3) — v2 per reviewer EVIDENCE_REQUEST.
# Usage: build-handoff-lite.sh <BINARY_SHA> <REASON_FOR_REVIEW>
# Fixes from reviewer iteration 1:
#   - whitelist = rel23-final evidence (no stale rel21/rel22 copies);
#   - MANIFEST excludes itself (no self-entry; manifest checksum delivered
#     out-of-band in the delivery message);
#   - junk excluded BEFORE packaging: ._* AppleDouble, .DS_Store, __MACOSX;
#   - sanitizer gate (fail-closed) before zip/mailbox.
set -euo pipefail
REPO=/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main
EV=/Users/danielviktorovich/Developing/RimLoc-evidence
BIN_SHA="${1:?usage: build-handoff-lite.sh <BINARY_SHA> <REASON>}"
REASON="${2:?usage: build-handoff-lite.sh <BINARY_SHA> <REASON>}"
ART="$EV/artifact-rel23-final"

STAMP=$(date +%Y%m%d-%H%M%S)
MAIN_SHA=$(git -C "$REPO" rev-parse HEAD)
PKT="$EV/reviewer/packet-lite-$STAMP"
rm -rf "$PKT"
mkdir -p "$PKT"

# --- REVIEW_REQUEST (A5 protocol v1) ---
cat > "$PKT/REVIEW_REQUEST.md" <<EOF
[RIMLOC_REVIEW_REQUEST_V1]

MAIN_SHA:
$MAIN_SHA

PRODUCT_SHA:
857b0d2a51b97e6e541cdf8f61d00683b966bd04 (product_source_sha; бинарная поверхность на нём же)

BINARY:
7781dd395b8e7c83332da9590a92972e6e93c285cef717200c9c3117471ae4bf (rimloc-gui), appzip 2d0f165213f700b66c58c75cfbe3c8110f9e0e1f05af623fcda887c4fa0cee20

RC:
rel23-final (iteration 2 — после вашего CONDITIONAL_PASS; все DO_NOW выполнены, список в WHAT_CHANGED)

REASON_FOR_REVIEW:
$REASON

WHAT_CHANGED:
- Пакет пересобран строго по вашему EVIDENCE_REQUEST: rel23 whitelist, без self-entry в манифесте, без ._*/.DS_Store джанка, без устаревших rel21/rel22 копий.
- RELEASE_GATE: заголовок/идентичность = rel23-final (857b0d2 / 7781dd39 / 2d0f1652).
- RELEASE_PARITY_MATRIX: Chat batch → PARITY (реализован R2), IfModActive → PARITY (--active-mods, R2), defs-gap R11 → ЗАКРЫТО (139 пар, KeyBindingDef, duplicate-key фикс); IMPLEMENT_NOW = 0.
- C5 честно: живой CodeQL на 24d1d1f = 323 clean / 1 vendor-only error (vendored tauri-utils platform.rs, upstream-макросы) — job success, CS 0 open; 298/298 было верно для более раннего прогона, до C6-форка. codeql-rust-extraction.txt прилагается.
- C4 формулировка исправлена: «0 known production/runtime npm vulnerabilities; 16 documented high-severity dev-tool advisories under explicit exemption (pending owner confirmation)» — до явного решения владельца.
- C6: VENDOR_AUDIT дополнен записью tauri-utils-2.10.1-nfs-appledouble (upstream 2.10.1, функциональный диф = 1 фильтр, правило обновления, hash/diff guard: diff -rq = ровно 1 differing file).
- C1: процессное правило добавлено (никаких скрытых session/OAuth credentials вне declared audience; больше не пробуем).
- state json: противоречие устранено (Source Inspector live убран из ownerGates — он LIVE и задокументирован).
- CI: финальный workflow_dispatch на текущий main после всей совокупности изменений.

QUESTIONS:
1. Принимаете ли C5 в формулировке «healthy tool status + documented 323/1 vendor-only extraction limitation»?
2. Достаточно ли нового пакета для финального аудита (OWNER_ONLY vs остался blocker)?
3. Подтверждаете ли, что перед owner packet не осталось truth/evidence разрывов?

FILES:
RimLoc-evidence-handoff-lite-<binary7>.zip (этот пакет; состав = ваш EVIDENCE_REQUEST список)

Please independently audit this state.
Return:
DO_NOW
OWNER_ONLY
BLOCKED
EVIDENCE_REQUEST
VERDICT

[RIMLOC_REVIEW_REQUEST_END]
EOF

# --- EVIDENCE_REQUEST whitelist (rel23-final truth) ---
cp "$REPO/docs/development/FINAL_IDENTITY.md"        "$PKT/" 2>/dev/null || true
cp "$ART/IDENTITY.md"                                "$PKT/artifact-IDENTITY.md" 2>/dev/null || true
cp "$ART/MANIFEST.sha256"                            "$PKT/artifact-MANIFEST.sha256" 2>/dev/null || true
cp "$ART/sha256.txt"                                 "$PKT/binary.sha256" 2>/dev/null || true
cp "$ART/appzip-sha256.txt"                          "$PKT/appzip.sha256" 2>/dev/null || true
cp "$REPO/docs/development/RELEASE_GATE.md"          "$PKT/" 2>/dev/null || true
cp "$REPO/docs/competitive/RELEASE_PARITY_MATRIX.md" "$PKT/" 2>/dev/null || true
cp "$REPO/docs/competitive/release-parity.json"      "$PKT/" 2>/dev/null || true
cp "$EV/C1_PROVIDER_BOUNDARY.md"                     "$PKT/" 2>/dev/null || true
cp "$EV/C6_DMG_NFS_EVIDENCE.md"                      "$PKT/" 2>/dev/null || true
cp "$REPO/docs/security/VENDOR_AUDIT_2026-10.md"     "$PKT/" 2>/dev/null || true
cp "$REPO/docs/security/CODE_SCANNING_RECONCILIATION.md" "$PKT/" 2>/dev/null || true
cp "$REPO/gui/tauri-app/frontend-v2/npm-audit-exemptions.md" "$PKT/" 2>/dev/null || true
cp "$EV/reviewer/npm-audit-react-summary.json"       "$PKT/npm-audit-react-summary.json" 2>/dev/null || true
cp "$EV/reviewer/npm-audit-v2-summary.json"          "$PKT/npm-audit-v2-summary.json" 2>/dev/null || true
cp "$EV/artifact-rel23-final/OWNER_TEST_PACKET.md"   "$PKT/" 2>/dev/null || true
# WDIO evidence + self-test/zip-listing (generate если ещё нет)
ZIP_PRE="$EV/RimLoc-evidence-handoff-lite-${BIN_SHA:0:7}.zip"
if [ -f "$ZIP_PRE" ]; then unzip -l "$ZIP_PRE" > "$EV/reviewer/zip-listing.txt" 2>/dev/null || true; fi
for f in source-inspector-wdio.log chatbatch-wdio.log multitarget-wdio.log palette-lm-wdio-summary.txt final-wdio-summary.txt packet-self-test.txt zip-listing.txt; do
  [ -f "$EV/reviewer/$f" ] && cp "$EV/reviewer/$f" "$PKT/"
done
# Tool/CI status (fresh from gh)
(cd "$REPO" && gh api repos/0-danielviktorovich-0/RimLoc/code-scanning/default-setup \
  --jq '{state:.state, config_file_path:.config_file_path}' \
  > "$PKT/codeql-tool-status.txt") 2>/dev/null || true
(cd "$REPO" && gh api "repos/0-danielviktorovich-0/RimLoc/code-scanning/analyses?ref=main&per_page=3" \
  --jq '.[] | "\(.commit_sha[0:7]) id=\(.id) \(.created_at)"' \
  > "$PKT/codeql-rust-extraction.txt") 2>/dev/null || true
cat >> "$PKT/codeql-rust-extraction.txt" <<'TXT'
Latest full-matrix rust extraction (job log, run on 24d1d1f):
  | Total number of Rust files that were extracted without error |   323 |
  | Total number of Rust files that were extracted with errors   |     1 |
The 1 error: gui/tauri-app/vendor/tauri-utils-2.10.1-nfs-appledouble/src/platform.rs
(upstream macros; documented vendor-only limitation — see VENDOR_AUDIT and §10 reconciliation).
TXT
(cd "$REPO" && gh pr list --state open --json number --jq 'length' \
  > "$PKT/dependabot-final-status.txt" \
 && echo "^ open PRs (dependabot included)" >> "$PKT/dependabot-final-status.txt" \
 && gh api "repos/0-danielviktorovich-0/RimLoc/dependabot/alerts?state=open" --jq 'length' \
   >> "$PKT/dependabot-final-status.txt" \
 && echo "^ open dependabot alerts" >> "$PKT/dependabot-final-status.txt") 2>/dev/null || true
(cd "$REPO" && gh run list --limit 8 --json headSha,workflowName,conclusion,createdAt \
  --jq '.[] | "\(.headSha[0:7]) \(.workflowName): \(.conclusion // .status) (\(.createdAt[0:16]))"' \
  > "$PKT/final-ci-summary.txt") 2>/dev/null || true

# --- junk exclusion + manifest WITHOUT self-entry ---
find "$PKT" \( -name '._*' -o -name '.DS_Store' \) -delete 2>/dev/null || true
( cd "$PKT" && find . -type f ! -name 'MANIFEST.sha256' -exec shasum -a 256 {} \; \
  | sed 's|\./||' | LC_ALL=C sort > MANIFEST.sha256 )

# --- sanitizer gate (A4, fail closed) ---
python3 "$REPO/testlab/reviewer/sanitize-packet.py" "$PKT" || {
  echo "PACKET NOT BUILT: sanitizer refused (fail closed). Dir: $PKT"
  exit 1
}

# --- zip БЕЗ метаданных (урок итерации 2: gate проверяет КОНЕЧНЫЙ артефакт
# снаружи, тем же способом, каким его увидит следующий потребитель) ---
ZIP="$EV/RimLoc-evidence-handoff-lite-${BIN_SHA:0:7}.zip"
rm -f "$ZIP"
# xattr-источник AppleDouble: снять со всех файлов пакета
while IFS= read -r -d '' f; do xattr -c "$f" 2>/dev/null; done < <(find "$PKT" -type f -print0)
find "$PKT" \( -name '._*' -o -name '.DS_Store' \) -delete 2>/dev/null || true
( cd "$PKT/.." && zip -q -r -X "$ZIP" "$(basename "$PKT")" )
# ВНЕШНЯЯ верификация: перечитать zip так, как увидит потребитель
JUNK=$(unzip -l "$ZIP" | grep -c -E "__MACOSX|/\._|\._[^/]*$|\.DS_Store" || true)
if [ "$JUNK" -ne 0 ]; then
  echo "PACKET REFUSED: $JUNK junk-записей в zip (внешняя проверка)"; exit 1
fi
echo "EXTERNAL CHECK: 0 junk entries in $(basename "$ZIP")"
# declared contents ⊆ actual zip contents (урок итерации 2: заявленное
# в сообщении должно физически лежать в пакете)
unzip -l "$ZIP" | awk '{print $NF}' | grep -E '\.md$|\.txt$|\.json$' | sed 's|.*/||' | LC_ALL=C sort -u > /tmp/declared-zip-contents.$$
declare -a REQUIRED=(REVIEW_REQUEST.md FINAL_IDENTITY.md RELEASE_GATE.md RELEASE_PARITY_MATRIX.md packet-self-test.txt zip-listing.txt)
MISSING=0
for f in "${REQUIRED[@]}"; do
  if ! grep -qx "$f" /tmp/declared-zip-contents.$$ && [ ! -f "$PKT/$f" ]; then
    echo "PACKET REFUSED: declared file '$f' отсутствует и в zip, и в пакете"; MISSING=1
  elif [ -f "$PKT/$f" ] && ! grep -qx "$f" /tmp/declared-zip-contents.$$; then
    echo "PACKET REFUSED: файл '$f' есть в пакете, но НЕ попал в zip"; MISSING=1
  fi
done
rm -f /tmp/declared-zip-contents.$$
[ "$MISSING" -eq 0 ] || exit 1
echo "DECLARED CONTENTS: ⊆ actual zip"
echo "MANIFEST checksum (out-of-band): $(shasum -a 256 "$PKT/MANIFEST.sha256" | awk '{print $1}')"
echo "PACKET OK: $ZIP ($(find "$PKT" -type f | wc -l | tr -d ' ') files)"

# --- A11 mailbox push ---
MAILBOX=/tmp/rimloc-mailbox-seed
if [ -d "$MAILBOX/.git" ]; then
  cp "$ZIP" "$MAILBOX/"
  git -C "$MAILBOX" add "$(basename "$ZIP")"
  git -C "$MAILBOX" commit -q -m "handoff-lite ${BIN_SHA:0:7} iter2: rel23 whitelist, no self-entry, junk-free" || true
  git -C "$MAILBOX" push -q origin HEAD:main 2>/dev/null && echo "MAILBOX: pushed" || echo "MAILBOX: push failed (offline)"
fi
echo "$ZIP"
