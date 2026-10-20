#!/usr/bin/env python3
"""Branch hygiene audit for RimLoc — classify remote branches, never delete.

Categories: ACTIVE / INTEGRATED / SUPERSEDED / ABANDONED / NEEDS_REVIEW /
PROTECTED. A branch is only ever *suggested* for deletion; this script has no
destructive mode by design.

Rules (each protects against a real past mistake):
  - PROTECTED: names in --protected (default: main) — never suggested.
  - ACTIVE: has an open PR whose head is NOT fully contained in the base
    (u_base > 0), or is checked out in a worktree, or named in --active.
    Stacked PRs: a branch another OPEN PR lists as its BASE is ACTIVE even
    if its own diff is merged elsewhere (its commits still gate the child).
  - INTEGRATED: open PR exists but every commit is an ancestor of the base
    (staged PR fully contained — e.g. lanes already merged into the
    integration branch), OR the PR is MERGED (squash-merged PRs keep unique
    git objects; PR state is the authority for content integration).
  - SUPERSEDED: PR CLOSED (not merged) while a successor merged the same
    content, or closed with the branch fully contained in the base.
  - ABANDONED: no PR at all and last commit older than --stale-days (30),
    excluding dependabot/release-plz bots which get their own categories.
  - NEEDS_REVIEW: everything else — including open dependabot PRs and
    branches with unique commits and no merged PR. Never auto-deleted.

Usage:
  scripts/branch-hygiene.py                 # human table + suggestions
  scripts/branch-hygiene.py --json          # machine report
  scripts/branch-hygiene.py --base feat/integration-rel27
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass, asdict

DEFAULT_PROTECTED = ["main"]


def git(*args: str, repo: str = ".") -> str:
    out = subprocess.run(
        ["git", "-C", repo, *args], capture_output=True, text=True
    )
    if out.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)}: {out.stderr.strip()}")
    return out.stdout.strip()


def gh_prs(repo_slug: str) -> dict[str, dict]:
    """headRefName -> latest PR record (via gh; empty map if unavailable)."""
    try:
        out = subprocess.run(
            ["gh", "pr", "list", "-R", repo_slug, "--state", "all",
             "--limit", "300", "--json", "number,headRefName,baseRefName,state"],
            capture_output=True, text=True, timeout=60,
        )
        rows = json.loads(out.stdout) if out.returncode == 0 else []
    except Exception:
        rows = []
    by_head: dict[str, dict] = {}
    for row in rows:
        head = row.get("headRefName") or ""
        if head:
            by_head[head] = row  # latest wins (list is newest-first already)
    return by_head


@dataclass
class BranchRow:
    branch: str
    pr: str
    pr_state: str
    pr_base: str
    integrated_in_base: bool  # every commit is an ancestor of --base
    unique_commits: int       # commits not in --base
    active_worktree: str      # worktree path or ""
    used_as_base_by: str      # open PR that stacks on this branch, or ""
    category: str
    recommendation: str
    risk: str


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo", default=".")
    ap.add_argument("--slug", default="0-danielviktorovich-0/RimLoc")
    ap.add_argument("--base", default="feat/integration-rel27",
                    help="integration target branch (default feat/integration-rel27)")
    ap.add_argument("--protected", nargs="*", default=DEFAULT_PROTECTED)
    ap.add_argument("--active", nargs="*", default=[],
                    help="branches to force-mark ACTIVE")
    ap.add_argument("--stale-days", type=int, default=30)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    git("-C", args.repo, "fetch", "--prune", "--quiet") if False else None
    base_ref = f"origin/{args.base}"
    git(*["-C", args.repo, "rev-parse", "--verify", "--quiet", base_ref])

    branches = [
        b for b in git(*["-C", args.repo, "branch", "-r", "--format=%(refname:short)"]).splitlines()
        if b and b != "origin/HEAD" and b.startswith("origin/") and b != base_ref
    ]
    prs = gh_prs(args.slug)

    # worktree usage: branch name -> path (across ALL worktrees of the repo)
    worktrees: dict[str, str] = {}
    for line in git(*["-C", args.repo, "worktree", "list"]).splitlines():
        # /path/to/wt  sha [branch]  | (detached)
        if "[" not in line:
            continue
        path = line.split()[0]
        branch = line.split("[", 1)[1].split("]", 1)[0]
        worktrees[branch] = path

    # stacked PRs: open PR whose BASE is this branch
    stacked: dict[str, str] = {}
    for head, row in prs.items():
        if row.get("state") == "OPEN" and row.get("baseRefName"):
            stacked.setdefault(row["baseRefName"], f"#{row['number']}")

    rows: list[BranchRow] = []
    for full in sorted(branches):
        name = full[len("origin/"):]
        if name in args.protected:
            rows.append(BranchRow(name, "-", "-", "-", True, 0,
                                  worktrees.get(name, ""), "", "PROTECTED",
                                  "keep", "protected branch"))
            continue

        pr = prs.get(name)
        unique = int(git(*["-C", args.repo, "rev-list", "--count",
                           f"{base_ref}..{full}"]) or 0)
        ancestor = subprocess.run(
            ["git", "-C", args.repo, "merge-base", "--is-ancestor", full, base_ref],
            capture_output=True,
        ).returncode == 0
        wt = worktrees.get(name, "")
        stack = stacked.get(name, "")
        forced = name in args.active
        bot = name.startswith("dependabot/") or name.startswith("release-plz-")

        if forced or wt or stack:
            cat = "ACTIVE"
            why = []
            if wt:
                why.append(f"worktree {wt}")
            if stack:
                why.append(f"base of stacked PR {stack}")
            if forced:
                why.append("flagged --active")
            rec, risk = "keep", "; ".join(why)
        elif pr and pr["state"] == "OPEN":
            if ancestor:
                cat = "INTEGRATED"
                rec = ("close PR as superseded by the staged integration PR, "
                       "then delete branch")
                risk = ("every commit already in " + args.base + "; PR duplicates "
                        "the staged PR — close it to keep review history, do not delete the PR")
            else:
                cat = "NEEDS_REVIEW"
                rec = "review open PR (unique commits against " + args.base + ")"
                risk = f"{unique} unique commit(s) not in {args.base}"
        elif pr and pr["state"] == "MERGED":
            cat = "INTEGRATED"
            rec = "delete branch (content merged via PR)"
            risk = ("unique git objects may remain after squash-merge — "
                    "PR keeps the review history; deletion is safe")
        elif pr and pr["state"] == "CLOSED":
            if ancestor:
                cat = "SUPERSEDED"
                rec = "delete branch (closed PR, commits contained in " + args.base + ")"
                risk = "low — history preserved in the closed PR"
            else:
                cat = "SUPERSEDED" if not bot else "NEEDS_REVIEW"
                rec = "confirm successor merged the content, then delete"
                risk = f"{unique} unique commit(s); verify content landed"
        elif bot:
            cat = "ABANDONED" if name.startswith("release-plz-") else "NEEDS_REVIEW"
            rec = ("delete stale bot branch" if cat == "ABANDONED"
                   else "bot branch with no PR — decide or ignore")
            risk = "no PR references it" if cat == "ABANDONED" else "bot automation"
        elif ancestor:
            cat = "INTEGRATED"
            rec = "delete branch (no PR ever opened, every commit is in " + args.base + ")"
            risk = "legacy/sync branch fully contained in the base"
        else:
            # no PR, not contained: age decides between abandoned and review
            import time
            try:
                ts = int(git(*["-C", args.repo, "for-each-ref",
                               "--format=%(committerdate:unix)", f"refs/remotes/{full}"]))
                age_days = (time.time() - ts) / 86400
            except Exception:
                age_days = -1
            if 0 <= age_days > args.stale_days:
                cat = "ABANDONED"
                rec = f"delete stale branch (no PR, untouched {age_days:.0f} days; confirm no unique content needed)"
                risk = f"{unique} unique commit(s) — verify nothing valuable before deletion"
            else:
                cat = "NEEDS_REVIEW"
                rec = "investigate: no PR, unique content?"
                risk = f"{unique} unique commit(s), no PR"

        rows.append(BranchRow(
            name,
            f"#{pr['number']}" if pr else "-",
            pr["state"] if pr else "none",
            pr["baseRefName"] if pr else "-",
            ancestor,
            unique,
            wt,
            stack,
            cat, rec, risk,
        ))

    if args.json:
        json.dump([asdict(r) for r in rows], sys.stdout, indent=1)
        return 0

    order = ["ACTIVE", "INTEGRATED", "SUPERSEDED", "ABANDONED",
             "NEEDS_REVIEW", "PROTECTED"]
    for cat in order:
        subset = [r for r in rows if r.category == cat]
        if not subset:
            continue
        print(f"\n== {cat} ({len(subset)}) ==")
        for r in subset:
            print(f"  {r.branch}")
            print(f"    PR {r.pr} [{r.pr_state}] base={r.pr_base} "
                  f"unique={r.unique_commits} integrated_in_{args.base}={r.integrated_in_base}"
                  + (f" worktree={r.active_worktree}" if r.active_worktree else "")
                  + (f" stacked_base_of={r.used_as_base_by}" if r.used_as_base_by else ""))
            print(f"    -> {r.recommendation}   [risk: {r.risk}]")

    deletable = [r for r in rows if r.category in ("INTEGRATED", "SUPERSEDED", "ABANDONED")
                 and r.category != "ACTIVE"]
    print(f"\n== SUMMARY ==")
    print(f"total remote branches (excl. protected/base): {len(rows)}")
    print(f"deletion candidates (owner approval required): {len(deletable)}")
    for r in deletable:
        print(f"  git push origin --delete {r.branch}")
    print("\nThis script never deletes anything; run the printed commands "
          "only after owner approval.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
