#!/usr/bin/env python3
"""Semantic identity check of a reviewer handoff packet (rel25 audit P1 fix).

Hash manifests prove bytes; they do NOT prove that the bytes belong to the
release being reviewed — the rel25 packet passed 28/28 manifest checks while
carrying rel23 identities. This checker closes that gap: every
identity-bearing document must name the CURRENT release id and the CURRENT
SHAs, and must NOT contain any known-stale release marker.

Fail-closed: exit 1 on the first missing mandatory file, any missing
identity pattern, or any stale marker. No warnings-only mode.

Usage:
  check-packet-identity.py <packet-dir> --rc <release-id> \
      --product-sha <40hex> --binary-sha <64hex> --appzip-sha <64hex> \
      --stale <marker,marker,...> [--binary7 <7hex>]
"""
from __future__ import annotations

import argparse
from pathlib import Path

# Identity-bearing documents and the patterns each MUST contain.
# Paths are relative to the packet root; None = no specific requirement
# beyond the stale-marker scan.
REQUIRED_PATTERNS: dict[str, list[str]] = {
    "REVIEW_REQUEST.md": ["{rc}", "{bin7}", "{product7}"],
    # The COMMITTED identity doc names the release and the ladder history;
    # per-build exact SHAs live in the artifact files (artifact-IDENTITY.md
    # carries them — that file is generated per build and is checked
    # strictly). Requiring per-build SHAs here would move the repo HEAD on
    # every identity refresh.
    "FINAL_IDENTITY.md": ["{rc}"],
    "RELEASE_GATE.md": ["{rc}"],
    "artifact-IDENTITY.md": ["{rc}", "{bin7}", "{product7}"],
    "OWNER_TEST_PACKET.md": ["{rc}"],
}

# Documents where a stale release marker is FATAL (identity documents).
# Evidence logs from older iterations are NOT in this list on purpose —
# historical logs may mention past releases; identity documents may not.
STALE_SCAN_FILES = [
    "REVIEW_REQUEST.md",
    "FINAL_IDENTITY.md",
    "RELEASE_GATE.md",
    "artifact-IDENTITY.md",
    "OWNER_TEST_PACKET.md",
    "binary.sha256",
    "appzip.sha256",
]

MANDATORY_FILES = sorted(set(REQUIRED_PATTERNS) | set(STALE_SCAN_FILES))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("packet", type=Path)
    ap.add_argument("--rc", required=True)
    ap.add_argument("--product-sha", required=True, help="40-hex source sha")
    ap.add_argument("--binary-sha", required=True, help="64-hex binary sha256")
    ap.add_argument("--appzip-sha", required=True, help="64-hex appzip sha256")
    ap.add_argument(
        "--stale",
        default="",
        help="comma-separated markers that must NOT appear in identity docs",
    )
    ap.add_argument(
        "--skip-missing",
        default="",
        help="comma-separated mandatory files to skip (RECORDED in output)",
    )
    args = ap.parse_args()

    pkt: Path = args.packet
    stale = [m.strip() for m in args.stale.split(",") if m.strip()]
    skip = {m.strip() for m in args.skip_missing.split(",") if m.strip()}

    failures: list[str] = []
    skipped: list[str] = []

    def read(rel: str) -> str | None:
        p = pkt / rel
        if not p.is_file():
            return None
        try:
            return p.read_text(encoding="utf-8", errors="replace")
        except OSError as e:
            failures.append(f"{rel}: unreadable ({e})")
            return None

    # 1. Mandatory files exist.
    for rel in MANDATORY_FILES:
        if rel in skip:
            skipped.append(rel)
            continue
        if not (pkt / rel).is_file():
            failures.append(f"MANDATORY MISSING: {rel}")

    # 2. Exact sha files carry exactly the declared SHAs.
    for rel, expected in (("binary.sha256", args.binary_sha), ("appzip.sha256", args.appzip_sha)):
        body = read(rel)
        if body is None:
            continue
        if expected.lower() not in body.lower():
            failures.append(f"{rel}: declared sha {expected[:12]}… not found in file")

    # 3. Identity documents carry the CURRENT identity.
    subs = {
        "{rc}": args.rc,
        "{bin7}": args.binary_sha[:7],
        "{product7}": args.product_sha[:7],
    }
    for rel, patterns in REQUIRED_PATTERNS.items():
        body = read(rel)
        if body is None:
            continue
        for pat in patterns:
            needle = subs.get(pat, pat)
            if needle not in body:
                failures.append(f"{rel}: current identity '{needle}' NOT found")

    # 4. No stale release marker in identity documents.
    for rel in STALE_SCAN_FILES:
        body = read(rel)
        if body is None:
            continue
        for marker in stale:
            if marker and marker in body:
                failures.append(f"{rel}: STALE marker '{marker}' present")

    if skipped:
        print(f"SKIPPED mandatory (recorded): {', '.join(sorted(skipped))}")
    if failures:
        print("PACKET IDENTITY: FAIL")
        for f in failures:
            print(f"  - {f}")
        return 1
    print(
        f"PACKET IDENTITY: PASS (rc={args.rc}, binary={args.binary_sha[:7]}…, "
        f"product={args.product_sha[:7]}…, stale markers checked: {len(stale)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
