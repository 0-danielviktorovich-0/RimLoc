#!/usr/bin/env python3
"""Sanitize a reviewer handoff packet — FAIL CLOSED (C/A4 gate).

Scans every file in the packet directory for credential-shaped material.
Any hit => exit 1 with the offending paths; the caller must NOT upload.
Patterns are structural (no hardcoded values): token prefixes, PEM blocks,
auth headers, assignment-style secrets, JWTs, and oversized high-entropy
base64 blobs. Screenshots/images are not scanned for content, but private
paths in filenames are refused.
"""
from __future__ import annotations

import base64
import math
import re
import sys
from pathlib import Path

PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("github-token", re.compile(r"gh[pousr]_[A-Za-z0-9]{16,}")),
    ("github-fine-grained-pat", re.compile(r"github_pat_[A-Za-z0-9_]{20,}")),
    ("openai-key", re.compile(r"sk-[A-Za-z0-9_-]{16,}")),
    ("anthropic-key", re.compile(r"sk-ant-[A-Za-z0-9_-]{16,}")),
    ("aws-access-key", re.compile(r"AKIA[0-9A-Z]{16}")),
    ("slack-token", re.compile(r"xox[baprs]-[A-Za-z0-9-]{10,}")),
    ("pem-private-key", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    ("auth-header", re.compile(r"(?i)authorization:\s*(bearer|basic|token)\s+\S+")),
    ("jwt", re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{5,}\b")),
    ("api-key-assignment", re.compile(r"(?i)\b(api[_-]?key|secret|passwd|password|token)\b[\"']?\s*[:=]\s*[\"'][A-Za-z0-9+/_-]{16,}[\"']")),
    ("bearer-inline", re.compile(r"(?i)bearer\s+[A-Za-z0-9._-]{25,}")),
    ("zai-oauth-blob", re.compile(r"(?i)(access_token|refresh_token)[\"']?\s*[:=]")),
]

# Redacted-marker whitelist: a sanitizer report or redaction placeholder may
# legitimately contain the WORD "token" in prose; only value-shaped hits fail.
PRIVATE_PATH_HINTS = re.compile(
    r"(?i)(клиент|client|парол|password|секрет|secret|ключ|key)[-_ ]?\w*\.(png|jpg|jpeg|heic|pdf)$"
)


def entropy(s: str) -> float:
    if not s:
        return 0.0
    counts: dict[str, int] = {}
    for ch in s:
        counts[ch] = counts.get(ch, 0) + 1
    return -sum((c / len(s)) * math.log2(c / len(s)) for c in counts.values())


def scan_file(path: Path) -> list[str]:
    hits: list[str] = []
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError as e:
        return [f"{path}: UNREADABLE ({e})"]
    for name, rx in PATTERNS:
        for m in rx.finditer(text):
            frag = m.group(0)
            # Prose guards: 'Authorization: Bearer' docs mention, redacted values.
            if name == "auth-header" and ("<" in frag or "…" in frag or "REDACTED" in frag.upper()):
                continue
            if name == "api-key-assignment" and any(
                w in frag.lower() for w in ("redacted", "example", "your_", "<", "..."
            )):
                continue
            if name == "bearer-inline" and entropy(frag.split()[-1]) < 3.5:
                continue
            if name == "zai-oauth-blob" and ("cookie" in path.name.lower() or "sanitiz" in path.name.lower()):
                continue
            line = text[: m.start()].count("\n") + 1
            hits.append(f"{path}:{line}: {name}")
    # High-entropy base64 blobs: require 64+ chars of base64 alphabet, entropy
    # >= 5.0, at least one digit and mixed case — true inline secrets qualify;
    # prose runs, path segments and hex hashes (entropy < ~4.2) do not.
    for m in re.finditer(r"\b[A-Za-z0-9+/]{64,}={0,2}\b", text):
        frag = m.group(0)
        if entropy(frag) >= 5.0 and any(c.isdigit() for c in frag) and any(c.islower() for c in frag) and any(c.isupper() for c in frag):
            line = text[: m.start()].count("\n") + 1
            hits.append(f"{path}:{line}: high-entropy-blob(len={len(frag)})")
    return hits


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: sanitize-packet.py <packet-dir>", file=sys.stderr)
        return 2
    root = Path(sys.argv[1]).resolve()
    if not root.is_dir():
        print(f"packet dir not found: {root}", file=sys.stderr)
        return 2

    all_hits: list[str] = []
    files = sorted(p for p in root.rglob("*") if p.is_file())
    for p in files:
        if PRIVATE_PATH_HINTS.search(p.name):
            all_hits.append(f"{p}: private-looking filename")
        if p.suffix.lower() in (".png", ".jpg", ".jpeg", ".heic", ".pdf", ".zip", ".gz", ".app"):
            continue  # binary/image: content not scannable; filenames checked above
        all_hits.extend(scan_file(p))

    if all_hits:
        print("SANITIZER: FAIL-CLOSED — credential-shaped material found:")
        for h in all_hits[:50]:
            print(f"  {h}")
        if len(all_hits) > 50:
            print(f"  … and {len(all_hits) - 50} more")
        print("UPLOAD REFUSED. Remove/redact the findings and re-run.")
        return 1

    print(f"SANITIZER: PASS ({len(files)} files scanned, 0 credential-shaped hits)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
