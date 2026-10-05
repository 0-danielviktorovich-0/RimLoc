#!/usr/bin/env python3
"""Canonical deterministic tree hash (§11, T6 correction 02.10).

Rules:
- relative paths (POSIX separators), sorted by UTF-8 bytes of the path string;
- SHA-256 of file BYTES per file; final digest = SHA-256 over
  "sha256hex  relpath\\n" lines concatenated in sorted order;
- symlinks: recorded as "symlink  relpath -> target" (never followed);
- ignored: .DS_Store, ._* (AppleDouble), Thumbs.db, *.tmp, desktop.ini.
Usage: canonical-tree-hash.py <root> [--exclude NAME ...]
"""
import hashlib, os, sys

root = os.path.realpath(sys.argv[1])
ignore_names = {".DS_Store", "Thumbs.db", "desktop.ini"}
lines = []
for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
    dirnames[:] = sorted(d for d in dirnames if d not in ignore_names and not d.startswith("._"))
    for name in sorted(filenames):
        if name in ignore_names or name.startswith("._") or name.endswith(".tmp"):
            continue
        full = os.path.join(dirpath, name)
        rel = os.path.relpath(full, root).replace(os.sep, "/")
        if os.path.islink(full):
            target = os.readlink(full)
            lines.append(f"symlink  {rel} -> {target}")
            continue
        h = hashlib.sha256()
        with open(full, "rb") as f:
            for chunk in iter(lambda: f.read(1 << 20), b""):
                h.update(chunk)
        lines.append(f"{h.hexdigest()}  {rel}")
digest = hashlib.sha256("\n".join(sorted(lines)).encode("utf-8")).hexdigest()
print(digest)
