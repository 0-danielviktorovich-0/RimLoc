#!/usr/bin/env python3
"""Public Git history language guard for RimLoc.

The repository may use internal orchestration vocabulary in private/local
state and in explicitly internal development documents. Public Git metadata
must describe software outcomes, not the agent process that produced them.

This checker intentionally focuses on:
- commit subjects
- pull-request titles
- release titles
- public branch names used by PRs
- newly added CHANGELOG lines

It does not rewrite existing history.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


CYRILLIC_RE = re.compile(r"[\u0400-\u04FF]")
CONVENTIONAL_SUBJECT_RE = re.compile(
    r"^(feat|fix|docs|chore|refactor|test|ci|build|perf|revert)"
    r"(\([^)]+\))?: [^ ].+"
)


@dataclass(frozen=True)
class Rule:
    name: str
    pattern: re.Pattern[str]
    hint: str


RULES: tuple[Rule, ...] = (
    Rule(
        "mandate-language",
        re.compile(r"(?i)(?:\bmandate(?:s)?\b|\bmandat(?:e|es|y|a|u|om)?\b|мандат)"),
        "describe the requirement or engineering outcome instead of an internal mandate",
    ),
    Rule(
        "machine-review-directive",
        re.compile(r"\b(?:DO_NOW|OWNER_ONLY|EVIDENCE_REQUEST|IMPLEMENT_NOW)\b"),
        "translate reviewer protocol directives into the actual engineering change",
    ),
    Rule(
        "owner-gate",
        re.compile(r"(?i)\bowner[-_ ]?gates?\b"),
        "name the concrete external requirement (for example signing or visual acceptance)",
    ),
    Rule(
        "workflow-id",
        re.compile(r"(?i)\bWF-[A-Z0-9][A-Z0-9_-]*\b"),
        "omit internal workflow identifiers from public Git metadata",
    ),
    Rule(
        "review-iteration",
        re.compile(
            r"(?i)(?:\breviewer\s+bridge\b|\breview(?:er)?[-_ ]?iteration\s*\d+\b|"
            r"\biterac(?:ii|iya|ia|ion)?\s*\d*\b|итерац)"
        ),
        "describe what changed rather than which review iteration produced it",
    ),
    Rule(
        "orchestration-wave",
        re.compile(
            r"(?i)\bwave(?:[-_ ](?:integration|final|audit|security|release|ui|"
            r"[A-Z]\b|\d+\b|[A-Z]\d+\b))"
        ),
        "replace internal wave naming with the product or subsystem outcome",
    ),
    Rule(
        "orchestration-lane",
        re.compile(
            r"(?i)\blane(?:[-_ ](?:integration|security|release|provider|arch|comp|ui|"
            r"[A-Z]\b|\d+\b|[A-Z]\d+\b))"
        ),
        "replace internal lane naming with the affected subsystem",
    ),
    Rule(
        "campaign-language",
        re.compile(r"(?i)\b(?:release\s+)?campaign\b"),
        "describe the release, feature, fix, or documentation change directly",
    ),
    Rule(
        "orchestration-phase",
        re.compile(
            r"(?i)(?:\bR\d+\s*(?:§|[-_: ](?:audit|gate|wave|lane|review|integration))|"
            r"\bC(?:\d+(?:[-–]\d+)?)?[-_ ]?gates?\b)"
        ),
        "do not expose internal phase/gate identifiers in public Git metadata",
    ),
    Rule(
        "known-transliterated-process-language",
        re.compile(
            r"(?i)\b(?:zakryt\w*|itog\w*|gotov\w*|pravd\w*|novyj|novyi|"
            r"lejnov?|shodits\w*|priemk\w*|svodk\w*)\b"
        ),
        "write the public summary in normal English rather than transliterated internal notes",
    ),
)

# Public branch names should also avoid process-oriented path segments.
BRANCH_RULES: tuple[Rule, ...] = (
    Rule(
        "branch-orchestration-name",
        re.compile(
            r"(?i)(?:^|/)(?:wf-|wave(?:-|/)|lane(?:-|/)|mandat(?:e)?(?:-|/)|"
            r"reviewer(?:-|/)|campaign(?:-|/)|r\d+(?:-|/))"
        ),
        "name the branch after the engineering outcome, e.g. feat/source-inspector",
    ),
)


def _violations(text: str, surface: str) -> list[str]:
    problems: list[str] = []
    value = text.strip()

    if surface in {"commit-subject", "pr-title", "release-title"} and CYRILLIC_RE.search(value):
        problems.append(
            "english-only: public commit/PR/release titles must be written in English"
        )

    if surface == "commit-subject":
        if len(value) > 72:
            problems.append("subject-length: commit subjects must be at most 72 characters")
        if not CONVENTIONAL_SUBJECT_RE.fullmatch(value):
            problems.append(
                "conventional-subject: use 'type(scope): engineering outcome'"
            )

    rules = RULES + (BRANCH_RULES if surface == "branch-name" else ())
    for rule in rules:
        if rule.pattern.search(value):
            problems.append(f"{rule.name}: {rule.hint}")

    # The 'review' scope is reserved for internal reviewer traffic. Public docs
    # commits should name the actual subject, usually docs(release|security|ui).
    if surface == "commit-subject" and re.match(
        r"(?i)^(?:docs|chore|test)\(review\):", value
    ):
        problems.append(
            "review-scope: use a product-facing scope such as release, security, ui, or repo"
        )

    return problems


def check_text(text: str, surface: str, label: str | None = None) -> list[str]:
    problems = _violations(text, surface)
    if not problems:
        return []
    head = label or surface
    return [f"{head}: {text!r}: {problem}" for problem in problems]


def _run_git(args: list[str]) -> str:
    proc = subprocess.run(
        ["git", *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return proc.stdout


def check_git_range(rev_range: str) -> list[str]:
    # Pull-request checkout refs include a synthetic GitHub merge commit.
    # Its title is generated by GitHub, not authored by the contributor; PR
    # title is checked separately. Validate the actual non-merge commits.
    out = _run_git(["log", "--no-merges", "--format=%s", rev_range])
    problems: list[str] = []
    for index, subject in enumerate(filter(None, out.splitlines()), start=1):
        problems.extend(
            check_text(subject, "commit-subject", f"commit[{index}]")
        )
    return problems


def _added_changelog_lines(diff_text: str) -> Iterable[str]:
    for line in diff_text.splitlines():
        if not line.startswith("+") or line.startswith("+++"):
            continue
        content = line[1:].strip()
        if content:
            yield content


def check_changelog_diff(diff_text: str, label: str) -> list[str]:
    problems: list[str] = []
    for index, line in enumerate(_added_changelog_lines(diff_text), start=1):
        problems.extend(check_text(line, "changelog", f"{label}[{index}]"))
    return problems


def check_staged_changelog() -> list[str]:
    diff_text = _run_git(["diff", "--cached", "--unified=0", "--", "CHANGELOG.md"])
    return check_changelog_diff(diff_text, "CHANGELOG staged addition")


def check_changelog_range(rev_range: str) -> list[str]:
    diff_text = _run_git(["diff", "--unified=0", rev_range, "--", "CHANGELOG.md"])
    return check_changelog_diff(diff_text, "CHANGELOG PR addition")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--surface",
        choices=("commit-subject", "pr-title", "release-title", "branch-name", "changelog"),
    )
    parser.add_argument("--text")
    parser.add_argument("--file", type=Path)
    parser.add_argument("--git-range")
    parser.add_argument("--staged-changelog", action="store_true")
    parser.add_argument("--changelog-range")
    args = parser.parse_args()

    problems: list[str] = []

    if args.surface:
        if args.text is not None:
            value = args.text
        elif args.file is not None:
            value = args.file.read_text(encoding="utf-8")
            if args.surface == "commit-subject":
                value = value.splitlines()[0] if value.splitlines() else ""
        else:
            parser.error("--surface requires --text or --file")
        problems.extend(check_text(value, args.surface))

    if args.git_range:
        problems.extend(check_git_range(args.git_range))

    if args.staged_changelog:
        problems.extend(check_staged_changelog())

    if args.changelog_range:
        problems.extend(check_changelog_range(args.changelog_range))

    if not any(
        (
            args.surface,
            args.git_range,
            args.staged_changelog,
            args.changelog_range,
        )
    ):
        parser.error("no check selected")

    if problems:
        print("Public Git history language check failed:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            "\nPublic Git history must describe the software outcome, not agent orchestration.",
            file=sys.stderr,
        )
        print(
            "See docs/development/PUBLIC_GIT_HISTORY_POLICY.md for examples.",
            file=sys.stderr,
        )
        return 1

    print("Public Git history language check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
