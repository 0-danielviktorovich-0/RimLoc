#!/usr/bin/env python3
"""Branch hygiene audit for RimLoc — classify remote branches, never delete.

Analysis ONLY: this script never deletes a branch, never closes a PR and
never changes repository settings; it prints evidence-backed candidates.

Guarantees (each backed by a test in scripts/tests/test_branch_hygiene.py):
  - PR metadata comes from `gh`; a GitHub CLI failure is a HARD error:
    no report, no deletion candidates, exit 2.
  - Remote refs are refreshed with `git fetch --prune` first; a fetch
    failure refuses to produce deletion recommendations (exit 3).
  - MERGED is not trusted as integration proof by itself: the PR's
    mergeCommit must be reachable from main or the integration base
    (squash-merge keeps unique git objects; reachability of the squash
    commit is the content-availability proof). Unreachable → NEEDS_REVIEW.
  - Branch reuse: for a head with several PRs the NEWEST PR wins
    (explicit sort by number — list order is not trusted).
  - A branch is KEEP_ACTIVE while: checked out in a worktree (parsed from
    `git worktree list --porcelain`), that worktree has uncommitted or
    untracked changes, a recent CI run is queued/in_progress on the
    branch, or it is the base of an OPEN stacked PR.
  - Deletion candidates come from ONE JSON manifest built first; the
    human report is rendered from that manifest only.

Categories: SAFE_TO_DELETE / NEEDS_REVIEW / KEEP_ACTIVE / PROTECTED.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from dataclasses import dataclass, asdict

DEFAULT_PROTECTED = ["main"]


class AuditError(RuntimeError):
    exit_code = 1


class GithubUnavailable(AuditError):
    exit_code = 2


class FetchFailed(AuditError):
    exit_code = 3


# ---------------------------------------------------------------------------
# Adapters (injected/mocked in tests)
# ---------------------------------------------------------------------------

class RealGit:
    def __init__(self, repo: str):
        self.repo = repo

    def run(self, *args: str) -> str:
        out = subprocess.run(["git", "-C", self.repo, *args],
                             capture_output=True, text=True)
        if out.returncode != 0:
            raise AuditError(f"git {' '.join(args)}: {out.stderr.strip()}")
        return out.stdout

    def code(self, *args: str) -> int:
        return subprocess.run(["git", "-C", self.repo, *args],
                              capture_output=True).returncode


class RealGh:
    def __init__(self, slug: str):
        self.slug = slug

    def prs(self) -> list[dict]:
        out = subprocess.run(
            ["gh", "pr", "list", "-R", self.slug, "--state", "all",
             "--limit", "300", "--json",
             "number,headRefName,baseRefName,state,mergeCommit,updatedAt"],
            capture_output=True, text=True, timeout=90,
        )
        if out.returncode != 0 or not out.stdout.strip():
            raise GithubUnavailable(
                f"gh pr list failed ({out.returncode}): {out.stderr.strip()}")
        try:
            rows = json.loads(out.stdout)
        except json.JSONDecodeError as exc:
            raise GithubUnavailable(f"gh pr list returned invalid JSON: {exc}")
        if not isinstance(rows, list):
            raise GithubUnavailable("gh pr list returned unexpected shape")
        return rows

    def recent_runs(self, branch: str) -> list[dict]:
        out = subprocess.run(
            ["gh", "run", "list", "-R", self.slug, "--branch", branch,
             "--limit", "3", "--json", "status,createdAt"],
            capture_output=True, text=True, timeout=60,
        )
        if out.returncode != 0:
            return []  # informational only; never gates safety alone
        try:
            return json.loads(out.stdout) or []
        except json.JSONDecodeError:
            return []


def fetch_refs(git: RealGit) -> None:
    out = subprocess.run(["git", "-C", git.repo, "fetch", "--prune", "origin"],
                         capture_output=True, text=True, timeout=300)
    if out.returncode != 0:
        raise FetchFailed(
            f"git fetch --prune origin failed: {out.stderr.strip()}")


# ---------------------------------------------------------------------------
# Pure helpers (unit-tested)
# ---------------------------------------------------------------------------

def latest_pr_per_head(rows: list[dict]) -> dict[str, dict]:
    """Newest PR per head branch — explicit sort by PR number. Branch names
    get REUSED; the higher number is the newer lifecycle entry, and list
    order from `gh` (newest-first) must not be relied upon."""
    by_head: dict[str, dict] = {}
    for row in sorted(rows, key=lambda r: int(r.get("number") or 0)):
        head = row.get("headRefName") or ""
        if head:
            by_head[head] = row  # highest number wins
    return by_head


def parse_worktrees(porcelain: str) -> list[dict]:
    """Parse `git worktree list --porcelain` blocks."""
    worktrees: list[dict] = []
    block: dict = {}
    for line in porcelain.splitlines():
        if not line:
            if block:
                worktrees.append(block)
                block = {}
            continue
        if line.startswith("worktree "):
            block = {"path": line[len("worktree "):]}
        elif line.startswith("branch ") and block:
            block["branch"] = line[len("branch "):].replace("refs/heads/", "")
        elif line.startswith("detached") and block:
            block["detached"] = True
        elif line.startswith("bare") and block:
            block["bare"] = True
    if block:
        worktrees.append(block)
    return worktrees


# ---------------------------------------------------------------------------
# Manifest
# ---------------------------------------------------------------------------

@dataclass
class BranchRow:
    branch: str
    head_sha: str
    pr: str
    pr_state: str
    target: str             # integration target used for containment
    content_verified: bool  # content provably available in main or base
    content_evidence: str
    worktree_clean: bool
    worktree_note: str
    dependent_prs: str      # open PRs stacked on this branch
    ci_recent: str
    unique_commits: int
    category: str           # SAFE_TO_DELETE / NEEDS_REVIEW / KEEP_ACTIVE / PROTECTED
    delete_safe: bool
    reason: str


def verify_content(git: RealGit, ref: str, merge_sha: str | None,
                   targets: list[str]) -> tuple[bool, str]:
    """Content-availability proof. Ancestor ⇒ contained. For squash-merged
    PRs the mergeCommit must be reachable from a target ref: the MERGED
    label alone says nothing about today's trees (history may be rewritten,
    the base branch may be gone)."""
    for target in targets:
        if git.code("merge-base", "--is-ancestor", ref, target) == 0:
            return True, f"every commit is an ancestor of {target}"
    if merge_sha:
        for target in targets:
            if git.code("merge-base", "--is-ancestor", merge_sha, target) == 0:
                return True, (f"PR merge commit {merge_sha[:10]} reachable "
                              f"from {target} (squash/merge content available)")
        return False, (f"PR merge commit {merge_sha[:10]} reachable from NO "
                       "target ref (history rewritten or base deleted)")
    return False, "not an ancestor and no merge commit to check"


def worktree_clean(path: str) -> tuple[bool, str]:
    out = subprocess.run(["git", "-C", path, "status", "--porcelain"],
                         capture_output=True, text=True)
    if out.returncode != 0:
        return False, "status-failed"
    if out.stdout.strip():
        return False, "dirty"
    return True, "clean"


def build_manifest(git: RealGit, gh: RealGh, args: argparse.Namespace
                   ) -> list[BranchRow]:
    base_ref = f"origin/{args.base}"
    targets = [f"origin/{r}" for r in [args.base] + list(args.main_refs)]

    branches = []
    for full in git.run("branch", "-r", "--format=%(refname:short)").splitlines():
        full = full.strip()
        if full and full != "origin/HEAD" and full.startswith("origin/"):
            branches.append(full)

    prs = latest_pr_per_head(gh.prs())
    stacked: dict[str, list[str]] = {}
    for head, row in prs.items():
        if row.get("state") == "OPEN" and row.get("baseRefName"):
            stacked.setdefault(row["baseRefName"], []).append(f"#{row['number']}")

    by_branch_wt: dict[str, dict] = {}
    for wt in parse_worktrees(git.run("worktree", "list", "--porcelain")):
        if wt.get("branch"):
            by_branch_wt[wt["branch"]] = wt

    rows: list[BranchRow] = []
    for full in sorted(branches):
        name = full[len("origin/"):]
        head_sha = git.run("rev-parse", "--short=10", full).strip()
        unique = int(git.run("rev-list", "--count", f"{base_ref}..{full}").strip() or 0)

        if name in args.protected:
            rows.append(BranchRow(name, head_sha, "-", "-", args.base, True,
                                  "protected branch", True, "n/a", "", "", 0,
                                  "PROTECTED", False, "protected: never a candidate"))
            continue

        pr = prs.get(name)
        pr_label = f"#{pr['number']}" if pr else "-"
        pr_state = (pr or {}).get("state", "none")
        merge_sha = ((pr or {}).get("mergeCommit") or {}).get("oid")

        verified, evidence = verify_content(git, full, merge_sha, targets)

        wt = by_branch_wt.get(name)
        if wt:
            clean, note = worktree_clean(wt["path"])
            wt_note = f"{wt['path']} ({note})"
        else:
            clean, note = True, "clean"
            wt_note = "none"

        deps = ",".join(stacked.get(name, []))
        ci = ""
        for run in gh.recent_runs(name):
            if run.get("status") in ("queued", "in_progress"):
                ci = f"{run['status']} at {run.get('createdAt', '?')}"
                break

        if name == args.base:
            # the staging base itself is never a deletion candidate, even
            # when no CI run happens to be active on it
            rows.append(BranchRow(name, head_sha, pr_label, pr_state,
                                  args.base, True, "is the target base itself",
                                  True, wt_note, deps, ci, unique,
                                  "KEEP_ACTIVE", False,
                                  "integration target branch (staging base)"))
            continue

        forced = name in args.active

        if forced or wt or deps or ci or not clean:
            why = []
            if wt:
                why.append(f"worktree {wt_note}")
            if deps:
                why.append(f"base of stacked PR {deps}")
            if ci:
                why.append(f"CI {ci}")
            if forced:
                why.append("flagged --active")
            rows.append(BranchRow(
                name, head_sha, pr_label, pr_state, args.base, verified,
                evidence, clean, wt_note, deps, ci, unique,
                "KEEP_ACTIVE", False, "; ".join(why)))
            continue

        if pr_state == "OPEN":
            if verified:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, True,
                    evidence, True, wt_note, deps, ci, unique,
                    "SAFE_TO_DELETE", True,
                    "open PR fully contained in the target — close the PR as "
                    "superseded (keep the PR for review history), then delete"))
            else:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, False,
                    evidence, True, wt_note, deps, ci, unique,
                    "NEEDS_REVIEW", False,
                    f"open PR with {unique} unique commit(s) — normal review"))
        elif pr_state == "MERGED":
            if verified:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, True,
                    evidence, True, wt_note, deps, ci, unique,
                    "SAFE_TO_DELETE", True,
                    "merged and content reachable in the target; review "
                    "history lives in the PR"))
            else:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, False,
                    evidence, True, wt_note, deps, ci, unique,
                    "NEEDS_REVIEW", False,
                    "PR labeled MERGED but content not reachable — investigate"))
        elif pr_state == "CLOSED":
            if verified:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, True,
                    evidence, True, wt_note, deps, ci, unique,
                    "SAFE_TO_DELETE", True,
                    "closed PR, content contained in the target"))
            else:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, False,
                    evidence, True, wt_note, deps, ci, unique,
                    "NEEDS_REVIEW", False,
                    "closed without merge, content not reachable — check for "
                    "a successor PR before deleting"))
        else:  # no PR at all
            if verified:
                rows.append(BranchRow(
                    name, head_sha, "-", "none", args.base, True, evidence,
                    True, wt_note, "", ci, unique, "SAFE_TO_DELETE", True,
                    "no PR, fully contained in the target (legacy/sync branch)"))
            else:
                try:
                    ts = int(git.run("for-each-ref", "--format=%(committerdate:unix)",
                                     f"refs/remotes/{full}").strip())
                except (AuditError, ValueError):
                    ts = -1
                age_days = (time.time() - ts) / 86400 if ts > 0 else -1
                label = (f"stale ({age_days:.0f} days, no PR)" if age_days > args.stale_days
                         else f"no PR, only {age_days:.0f} days old")
                rows.append(BranchRow(
                    name, head_sha, "-", "none", args.base, False, evidence,
                    True, wt_note, "", ci, unique, "NEEDS_REVIEW", False,
                    f"{label} with {unique} unique commit(s) — confirm "
                    "nothing valuable"))
    return rows


# ---------------------------------------------------------------------------
# Reporting (rendered FROM the manifest only)
# ---------------------------------------------------------------------------

ORDER = ["SAFE_TO_DELETE", "NEEDS_REVIEW", "KEEP_ACTIVE", "PROTECTED"]


def render(rows: list[BranchRow]) -> str:
    out: list[str] = [f"total remote branches (incl. protected): {len(rows)}"]
    for cat in ORDER:
        subset = [r for r in rows if r.category == cat]
        out.append(f"\n== {cat} ({len(subset)}) ==")
        for r in subset:
            out.append(
                f"  {r.branch} | {r.head_sha} | PR {r.pr} [{r.pr_state}] "
                f"| target={r.target} | content_verified={r.content_verified} "
                f"| worktree_clean={r.worktree_clean} ({r.worktree_note}) "
                f"| dependent_prs={r.dependent_prs or '-'}"
                + (f" | ci={r.ci_recent}" if r.ci_recent else "")
                + f" | unique={r.unique_commits}")
            out.append(f"      delete_safe={r.delete_safe} — {r.reason}")
            if r.content_evidence:
                out.append(f"      evidence: {r.content_evidence}")
    deletable = [r for r in rows if r.delete_safe]
    out.append(f"\n== MANIFEST-DERIVED DELETION CANDIDATES ({len(deletable)}) ==")
    for r in deletable:
        out.append(f"  git push origin --delete {r.branch}")
    out.append("\nAnalysis only — nothing is deleted; owner approval required.")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo", default=".")
    ap.add_argument("--slug", default="0-danielviktorovich-0/RimLoc")
    ap.add_argument("--base", default="feat/integration-rel27")
    ap.add_argument("--main-refs", nargs="*", default=["main"],
                    help="refs that also count as content targets")
    ap.add_argument("--protected", nargs="*", default=DEFAULT_PROTECTED)
    ap.add_argument("--active", nargs="*", default=[])
    ap.add_argument("--stale-days", type=int, default=30)
    ap.add_argument("--no-fetch", action="store_true",
                    help="skip the ref refresh (tests only)")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    try:
        git = RealGit(args.repo)
        gh = RealGh(args.slug)
        if not args.no_fetch:
            fetch_refs(git)
        rows = build_manifest(git, gh, args)
    except GithubUnavailable as exc:
        print(f"ERROR: GitHub metadata unavailable — refusing to audit: {exc}",
              file=sys.stderr)
        return GithubUnavailable.exit_code
    except FetchFailed as exc:
        print("ERROR: could not refresh remote refs — no deletion "
              f"recommendations: {exc}", file=sys.stderr)
        return FetchFailed.exit_code
    except AuditError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return AuditError.exit_code

    if args.json:
        json.dump([asdict(r) for r in rows], sys.stdout, indent=1)
        return 0
    print(render(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
