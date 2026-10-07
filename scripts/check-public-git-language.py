#!/usr/bin/env python3
"""Fail-closed public Git history hygiene for RimLoc.

Public Git surfaces describe the software, not the private orchestration that
produced it. Internal planning vocabulary may live in local/private state and
engineering notes, but not in commit subjects, PR titles, branch names,
release titles, or newly added CHANGELOG lines.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

CYRILLIC_RE = re.compile(r"[\u0400-\u04FF]")


@dataclass(frozen=True)
class Rule:
    name: str
    pattern: re.Pattern[str]
    replacement_hint: str


RULES: tuple[Rule, ...] = (
    Rule("mandate terminology", re.compile(r"(?i)(?:\bmandate\w*\b|\bmandat\w*\b|мандат\w*)"), "describe the requirement or behavior instead"),
    Rule("review protocol terminology", re.compile(r"(?i)(?:\breviewer\b|\breview[_ -]?request\b|\brimloc_review_request\b|\bconditional[_ -]?pass\b)"), "describe the verified engineering result instead"),
    Rule("automation status token", re.compile(r"(?i)\b(?:do[_ -]?now|owner[_ -]?only|evidence[_ -]?request|implement[_ -]?now)\b"), "translate the status into the actual change"),
    Rule("owner orchestration terminology", re.compile(r"(?i)\bowner(?:[_ -]?(?:gate|decision|approval))\b"), "describe the release or product constraint directly"),
    Rule("workflow lane identifier", re.compile(r"(?i)(?:\bWF[-_:][A-Za-z0-9_-]+\b|\blane\b|\blejn\w*\b)"), "describe the component or subsystem instead"),
    Rule("wave identifier", re.compile(r"(?i)(?:\bwave\b|\bvoln\w*\b|волн\w*)"), "describe the delivered behavior instead"),
    Rule("campaign or iteration terminology", re.compile(r"(?i)(?:\bcampaign\b|\bkampan\w*\b|кампан\w*|\biteration\w*\b|\biterac\w*\b|итерац\w*)"), "describe the concrete engineering outcome instead"),
    Rule("handoff terminology", re.compile(r"(?i)(?:\bhandoff\b|\breview[_ -]?packet\b|\bcontext[_ -]?packet\b)"), "describe the artifact or documentation purpose instead"),
    Rule("internal shorthand identifier", re.compile(r"(?<![A-Za-z0-9])(?:R|C|G|W|P|I|S)\d+(?:[-.][A-Z0-9]+)*(?![A-Za-z0-9])"), "spell out the public subsystem or behavior"),
    Rule("internal gate label", re.compile(r"\bGate\s+[A-Z0-9]+\b"), "name the validated behavior instead"),
    Rule("internal section marker", re.compile(r"§\s*\d+"), "omit private section numbers"),
    Rule("agent runtime name", re.compile(r"(?i)\b(?:ZCode|ChatGPT|Codex|Claude Code)\b"), "describe the software or automation capability without naming the agent runtime"),
)

PUBLIC_TEXT_SURFACES = {"commit", "pr", "release", "branch", "changelog"}


def violations(text: str, surface: str) -> list[str]:
    found: list[str] = []
    if CYRILLIC_RE.search(text):
        found.append("Cyrillic text (public Git metadata must be English)")
    for rule in RULES:
        match = rule.pattern.search(text)
        if match:
            found.append(f"{rule.name}: {match.group(0)!r}; {rule.replacement_hint}")
    return found


def check_text(text: str, surface: str, label: str | None = None) -> bool:
    issues = violations(text, surface)
    if not issues:
        return True
    where = label or surface
    print(f"ERROR: public Git {where} uses internal orchestration language:", file=sys.stderr)
    print(f"  {text}", file=sys.stderr)
    for issue in issues:
        print(f"  - {issue}", file=sys.stderr)
    print("Rewrite it as an English engineering outcome: what changed in the software/repository and why.", file=sys.stderr)
    return False


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True, stderr=subprocess.STDOUT)


def check_commit_range(base: str, head: str) -> bool:
    output = git("log", "--format=%H%x09%s", f"{base}..{head}")
    ok = True
    for line in output.splitlines():
        if not line.strip():
            continue
        sha, subject = line.split("\t", 1)
        if not check_text(subject, "commit", f"commit {sha[:12]}"):
            ok = False
    return ok


def added_changelog_lines(base: str, head: str) -> list[str]:
    diff = git("diff", "--unified=0", f"{base}...{head}", "--", "CHANGELOG.md")
    lines: list[str] = []
    for line in diff.splitlines():
        if line.startswith("+++") or not line.startswith("+"):
            continue
        value = line[1:]
        if value.strip():
            lines.append(value)
    return lines


def check_changelog_diff(base: str, head: str) -> bool:
    ok = True
    for index, line in enumerate(added_changelog_lines(base, head), start=1):
        if not check_text(line, "changelog", f"CHANGELOG added line {index}"):
            ok = False
    return ok


def run_self_test() -> bool:
    allowed = (
        ("commit", "feat(gui): show live source provenance in workspace"),
        ("commit", "fix(llm): compile keychain-only provider builds"),
        ("commit", "docs(release): reconcile rel23 release evidence"),
        ("pr", "Add conditional LoadFolders resolution"),
        ("branch", "fix/provider-feature-gates"),
        ("release", "RimLoc v0.1.0-beta.1"),
        ("changelog", "- [gui] Add live source provenance to the workspace."),
    )
    rejected = (
        ("commit", "docs(review): DO_NOW iteration 1 - release truth"),
        ("commit", "docs(gate): reviewer mandate results"),
        ("commit", "chore(integration): wave integration 2"),
        ("commit", "docs(process): R4 §1-2 autonomous failures"),
        ("branch", "release/wave-integration-2"),
        ("pr", "R4 owner review"),
        ("changelog", "- Wave 5 owner gate cleanup"),
        ("commit", "docs(review): iteracii 1-3 Reviewer Bridge -> OWNER_ONLY"),
        ("commit", "docs(process): 5 lejnov"),
    )
    failures: list[str] = []
    for surface, text in allowed:
        if violations(text, surface):
            failures.append(f"false positive: {surface}: {text}")
    for surface, text in rejected:
        if not violations(text, surface):
            failures.append(f"false negative: {surface}: {text}")
    if failures:
        print("Public Git hygiene self-test FAILED:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return False
    print("Public Git hygiene self-test passed.")
    return True


def read_text_arg(args: argparse.Namespace) -> str:
    if args.text is not None:
        return args.text
    if args.file is not None:
        return Path(args.file).read_text(encoding="utf-8").splitlines()[0]
    raise SystemExit("--text or --file is required for this surface")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--surface", choices=sorted(PUBLIC_TEXT_SURFACES | {"commit-range", "changelog-diff"}))
    parser.add_argument("--text")
    parser.add_argument("--file")
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return 0 if run_self_test() else 1
    if args.surface is None:
        parser.error("--surface is required unless --self-test is used")
    if args.surface == "commit-range":
        if not args.base or not args.head:
            parser.error("--base and --head are required for commit-range")
        return 0 if check_commit_range(args.base, args.head) else 1
    if args.surface == "changelog-diff":
        if not args.base or not args.head:
            parser.error("--base and --head are required for changelog-diff")
        return 0 if check_changelog_diff(args.base, args.head) else 1
    return 0 if check_text(read_text_arg(args), args.surface) else 1


if __name__ == "__main__":
    raise SystemExit(main())
