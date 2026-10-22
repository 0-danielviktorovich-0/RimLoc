#!/usr/bin/env python3
"""Publish preflight (Issue #8): every workspace path-dependency on a
rimloc-* crate must carry a version key — `cargo publish` resolves
dependencies against the registry, and a versionless path dep fails the
build for anyone but this checkout. Exit 1 naming the offending
manifest lines."""
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
bad = []
for mf in sorted(root.glob("crates/*/Cargo.toml")):
    for i, line in enumerate(mf.read_text().splitlines(), 1):
        m = re.match(r"^(rimloc-[a-z0-9-]+)\s*=\s*\{\s*(.*)\}\s*$", line)
        if not m:
            continue
        body = m.group(2)
        if "path" in body and "version" not in body:
            bad.append(f"{mf.relative_to(root)}:{i}: {line.strip()}")
if bad:
    print("versionless path dependencies block cargo publish:")
    print("\n".join(bad))
    sys.exit(1)
print("all rimloc path dependencies carry a version key")
