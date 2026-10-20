#!/usr/bin/env python3
"""Regression gate: machine-specific paths must never enter production UI code.

Do not confuse legitimate ephemeral paths in test fixtures with constants
compiled into the React/Tauri product. This guard intentionally targets only
shipping source directories. Dev/test fixtures are audited separately.
"""
from __future__ import annotations

from pathlib import Path
import re
import sys

REPO = Path(__file__).resolve().parents[2]
# Literal machine-specific paths; URLs, *relative* game paths, and runtime
# resolved user choices are intentionally not blocked.
ABSOLUTE_MACHINE_PATH = re.compile(
    r"/Users/[A-Za-z0-9_.-]+/|"
    r"/home/[A-Za-z0-9_.-]+/|"
    r"/Applications/RimWorld(?:\.app)?/|"
    r"[A-Za-z]:[\\/]+Users[\\/]+[A-Za-z0-9_.-]+[\\/]"
)


def find_bad_strings(text: str) -> list[tuple[int, str]]:
    return [
        (i, s.strip()[:200])
        for i, s in enumerate(text.splitlines(), start=1)
        if ABSOLUTE_MACHINE_PATH.search(s)
    ]


def files_to_check() -> list[Path]:
    react = REPO / "gui/tauri-app/frontend-react/src"
    tauri = REPO / "gui/tauri-app/src-tauri/src"
    excluded = ("/__tests__/", "/tests/", "/test/", "/mock/", ".test.", ".spec.")
    out: list[Path] = []
    for root, patterns in [(react, ("*.ts", "*.tsx")), (tauri, ("*.rs",))]:
        for glob in patterns:
            for p in root.rglob(glob):
                rel = "/" + p.relative_to(REPO).as_posix()
                if any(x in rel for x in excluded) or p.name == "tests.rs":
                    continue
                out.append(p)
    return sorted(set(out))


def self_test() -> None:
    assert find_bad_strings('const root = "/Users/jane/Projects/RimLoc/Mods"')
    assert find_bad_strings('let root = "/home/joe/rimworld/Mods";')
    assert find_bad_strings('const x = "C:\\\\Users\\\\bob\\\\RimWorld\\\\"')
    assert find_bad_strings('const game = "/Applications/RimWorld.app/Mods";')
    for text in (
        'const x = "./test/fixtures";',
        'const y = "/Users/<user>/example";',
        'const root = await client.pickDirectory();',
        'const root = snapshot.source_root?.path;',
        'const url = "https://github.com/0-danielviktorovich-0/RimLoc";'
    ):
        assert not find_bad_strings(text), f"false positive: {text}"


def main() -> int:
    self_test()
    failures = []
    files = files_to_check()
    for p in files:
        found = find_bad_strings(p.read_text(encoding="utf-8"))
        failures.extend((str(p.relative_to(REPO)), ln, sample) for ln, sample in found)
    if failures:
        for path, ln, sample in failures:
            print(f"INVALID_MACHINE_PATH: {path}:{ln}: {sample}", file=sys.stderr)
        print("Use chosen/persisted paths, Tauri path APIs, or isolated test fixtures.", file=sys.stderr)
        return 1
    print(f"Production absolute-path guard: PASS ({len(files)} files; self-tests PASS)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
