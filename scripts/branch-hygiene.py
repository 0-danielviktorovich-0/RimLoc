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
             "number,headRefName,baseRefName,state,mergeCommit,headRefOid,updatedAt"],
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

    def recent_runs(self, branch: str) -> list[dict] | None:
        """None = the CI state could NOT be determined (API failure) —
        the caller must treat that as UNKNOWN and refuse deletion."""
        out = subprocess.run(
            ["gh", "run", "list", "-R", self.slug, "--branch", branch,
             "--limit", "3", "--json", "status,createdAt"],
            capture_output=True, text=True, timeout=60,
        )
        if out.returncode != 0:
            return None
        try:
            return json.loads(out.stdout) or []
        except json.JSONDecodeError:
            return None


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
    ci_state: str           # active / unknown / quiet
    ci_note: str
    advanced_past_pr_head: str  # commits the branch gained after its PR head
    unique_commits: int
    category: str           # SAFE_TO_DELETE / NEEDS_REVIEW / KEEP_ACTIVE / PROTECTED
    delete_safe: bool
    reason: str


def revert_of_pr_in_targets(git: RealGit, pr_label: str, branch: str,
                            targets: list[str]) -> str | None:
    """Detect an explicit GitHub-style revert commit in the targets that
    names this PR. Reachability of a merge commit alone does not prove the
    change survived; a later Revert commit is the common counter-evidence."""
    if not pr_label:
        return None
    for target in targets:
        log = git.run("log", "--format=%s", "-n", "500", target)
        for line in log.splitlines():
            if line.startswith("Revert") and (pr_label in line
                                              or branch in line):
                return f"revert commit in {target}: {line[:80]}"
    return None


def extra_commits_past_pr_head(git: RealGit, branch_ref: str,
                               pr_head: str | None) -> list[str]:
    """Commits the branch gained AFTER the head its PR was last updated to.
    A merged PR proves only its own head; anything past it needs its own
    proof (P0 counterexample: branch advanced 3 commits post-merge)."""
    if not pr_head:
        return []
    out = git.run("rev-list", "--count", f"{pr_head}..{branch_ref}").strip()
    if out == "0" or not out.isdigit():
        return []
    return git.run("rev-list", f"{pr_head}..{branch_ref}").split()


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
        head_sha = git.run("rev-parse", full).strip()
        unique = int(git.run("rev-list", "--count", f"{base_ref}..{full}").strip() or 0)

        if name in args.protected:
            rows.append(BranchRow(name, head_sha, "-", "-", args.base, True,
                                  "protected branch", True, "n/a", "", "quiet",
                                  "not checked for protected", "", 0,
                                  "PROTECTED", False,
                                  "protected: never a candidate"))
            continue

        pr = prs.get(name)
        pr_label = f"#{pr['number']}" if pr else "-"
        pr_state = (pr or {}).get("state", "none")
        merge_sha = ((pr or {}).get("mergeCommit") or {}).get("oid")
        pr_head = (pr or {}).get("headRefOid")

        verified, evidence = verify_content(git, full, merge_sha, targets)

        # P0: a merged PR proves only its own head. If the branch moved on,
        # each extra commit needs its own containment proof.
        extra = extra_commits_past_pr_head(git, full, pr_head)
        advanced = (f"{len(extra)} commit(s) past the PR head"
                    if extra else "")
        if extra and pr_state == "MERGED":
            unproven = [c for c in extra
                        if all(git.code("merge-base", "--is-ancestor",
                                        c, tgt) != 0 for tgt in targets)]
            if unproven:
                verified = False
                evidence = (f"PR merge proof covers the PR head only; branch "
                            f"advanced {len(extra)} commit(s) afterwards and "
                            f"{len(unproven)} of them are not contained in "
                            f"any target")
            else:
                evidence += f"; {len(extra)} post-merge commit(s) also contained"

        # Revert heuristic: a reachable merge commit can still have been
        # reverted later in the target.
        if verified and merge_sha:
            reverted = revert_of_pr_in_targets(git, pr_label, name, targets)
            if reverted:
                verified = False
                evidence = f"{evidence}; BUT {reverted}"

        wt = by_branch_wt.get(name)
        if wt:
            clean, note = worktree_clean(wt["path"])
            wt_note = f"{wt['path']} ({note})"
        else:
            clean, note = True, "clean"
            wt_note = "none"

        deps = ",".join(stacked.get(name, []))
        runs = gh.recent_runs(name)
        if runs is None:
            ci_state, ci_note = "unknown", "CI state could not be determined"
        else:
            active = next((r for r in runs
                           if r.get("status") in ("queued", "in_progress")), None)
            if active:
                ci_state = "active"
                ci_note = f"{active['status']} at {active.get('createdAt', '?')}"
            else:
                ci_state, ci_note = "quiet", "no queued/in-progress runs"

        if name == args.base:
            # the staging base itself is never a deletion candidate, even
            # when no CI run happens to be active on it
            rows.append(BranchRow(name, head_sha, pr_label, pr_state,
                                  args.base, True, "is the target base itself",
                                  True, wt_note, deps, ci_state, ci_note, "",
                                  unique, "KEEP_ACTIVE", False,
                                  "integration target branch (staging base)"))
            continue

        forced = name in args.active

        if forced or wt or deps or ci_state == "active" or not clean:
            why = []
            if wt:
                why.append(f"worktree {wt_note}")
            if deps:
                why.append(f"base of stacked PR {deps}")
            if forced:
                why.append("flagged --active")
            if ci_state == "active":
                why.append(f"CI {ci_note}")
            rows.append(BranchRow(
                name, head_sha, pr_label, pr_state, args.base, verified,
                evidence, clean, wt_note, deps, ci_state, ci_note, advanced,
                unique, "KEEP_ACTIVE", False, "; ".join(why)))
            continue

        if pr_state == "OPEN":
            if verified:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, True,
                    evidence, True, wt_note, deps, ci_state, ci_note, advanced, unique,
                    "SAFE_TO_DELETE", True,
                    "open PR fully contained in the target — close the PR as "
                    "superseded (keep the PR for review history), then delete"))
            else:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, False,
                    evidence, True, wt_note, deps, ci_state, ci_note, advanced, unique,
                    "NEEDS_REVIEW", False,
                    f"open PR with {unique} unique commit(s) — normal review"))
        elif pr_state == "MERGED":
            if verified:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, True,
                    evidence, True, wt_note, deps, ci_state, ci_note, advanced, unique,
                    "SAFE_TO_DELETE", True,
                    "merged and content reachable in the target; review "
                    "history lives in the PR"))
            else:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, False,
                    evidence, True, wt_note, deps, ci_state, ci_note, advanced, unique,
                    "NEEDS_REVIEW", False,
                    "PR labeled MERGED but content not reachable — investigate"))
        elif pr_state == "CLOSED":
            if verified:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, True,
                    evidence, True, wt_note, deps, ci_state, ci_note, advanced, unique,
                    "SAFE_TO_DELETE", True,
                    "closed PR, content contained in the target"))
            else:
                rows.append(BranchRow(
                    name, head_sha, pr_label, pr_state, args.base, False,
                    evidence, True, wt_note, deps, ci_state, ci_note, advanced, unique,
                    "NEEDS_REVIEW", False,
                    "closed without merge, content not reachable — check for "
                    "a successor PR before deleting"))
        else:  # no PR at all
            if verified:
                rows.append(BranchRow(
                    name, head_sha, "-", "none", args.base, True, evidence,
                    True, wt_note, "", ci_state, ci_note, "", unique, "SAFE_TO_DELETE", True,
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
                    True, wt_note, "", ci_state, ci_note, "", unique, "NEEDS_REVIEW", False,
                    f"{label} with {unique} unique commit(s) — confirm "
                    "nothing valuable"))
    # P1: CI state UNKNOWN forbids an automatic deletion recommendation
    for i, row in enumerate(rows):
        if (row.ci_state == "unknown" and row.delete_safe
                and row.category == "SAFE_TO_DELETE"):
            rows[i] = BranchRow(**{**asdict(row),
                                   "category": "NEEDS_REVIEW",
                                   "delete_safe": False,
                                   "reason": ("CI state unknown (Actions API "
                                              "unavailable): " + row.reason)})
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
                + (f" | ci={r.ci_state}" if r.ci_state != "quiet" else "")
                + (f" | advanced={r.advanced_past_pr_head}" if r.advanced_past_pr_head else "")
                + f" | unique={r.unique_commits}")
            out.append(f"      delete_safe={r.delete_safe} — {r.reason}")
            if r.content_evidence:
                out.append(f"      evidence: {r.content_evidence}")
    deletable = [r for r in rows if r.delete_safe]
    out.append(f"\n== MANIFEST-DERIVED DELETION CANDIDATES ({len(deletable)}) ==")
    out.append("  # atomic compare-and-delete: refuses if the remote ref")
    out.append("  # moved after this audit (force-with-lease on expected SHA)")
    for r in deletable:
        out.append(f"  git push origin "
                   f"--force-with-lease=refs/heads/{r.branch}:{r.head_sha} "
                   f":refs/heads/{r.branch}")
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
