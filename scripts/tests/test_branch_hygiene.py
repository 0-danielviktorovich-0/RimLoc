"""Tests for scripts/branch-hygiene.py — run: python3 -m unittest discover -s scripts/tests

All tests are offline: git/gh adapters are fakes built on real fixture
repositories where needed; GitHub is never contacted.
Layout note: scripts/ has no __init__ packaging; we import by path.
"""
from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from argparse import Namespace

_MOD_PATH = os.path.join(os.path.dirname(__file__), "..", "branch-hygiene.py")
_spec = importlib.util.spec_from_file_location("branch_hygiene", _MOD_PATH)
bh = importlib.util.module_from_spec(_spec)
sys.modules["branch_hygiene"] = bh
_spec.loader.exec_module(bh)  # noqa: E402


def make_args(**kw) -> Namespace:
    base = dict(base="integration", main_refs=["main"], protected=["main"],
                active=[], stale_days=30)
    base.update(kw)
    return Namespace(**base)


class FakeGh:
    def __init__(self, prs=None, fail=False, runs=None):
        self._prs = prs or []
        self._fail = fail
        self._runs = runs or {}

    def prs(self):
        if self._fail:
            raise bh.GithubUnavailable("simulated gh outage")
        return self._prs

    def recent_runs(self, branch):
        return self._runs.get(branch, [])


class PureHelpers(unittest.TestCase):
    def test_branch_name_reuse_newest_pr_wins(self):
        rows = [
            {"number": 12, "headRefName": "lane", "state": "OPEN"},   # newer
            {"number": 7, "headRefName": "lane", "state": "MERGED"},  # older
        ]
        by_head = bh.latest_pr_per_head(rows)
        self.assertEqual(by_head["lane"]["number"], 12)

    def test_ordering_is_explicit_not_list_order(self):
        # oldest-first list order must not matter
        rows = [
            {"number": 7, "headRefName": "lane", "state": "MERGED"},
            {"number": 12, "headRefName": "lane", "state": "OPEN"},
        ]
        self.assertEqual(bh.latest_pr_per_head(rows)["lane"]["number"], 12)

    def test_parse_worktrees_porcelain(self):
        porcelain = (
            "worktree /wt/a\nbranch refs/heads/feat/one\n\n"
            "worktree /wt/b\ndetached\n\n"
            "worktree /wt/c\nbranch refs/heads/main\nbare\n\n"
        )
        wts = bh.parse_worktrees(porcelain)
        self.assertEqual(wts[0]["branch"], "feat/one")
        self.assertTrue(wts[1].get("detached"))
        self.assertEqual(wts[2]["branch"], "main")


class RealRepoTest(unittest.TestCase):
    """Fixture repo: real git plumbing, faked GitHub."""

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.repo = self.dir.name
        self.g("init", "-q", "-b", "main")
        self.g("config", "user.email", "t@example.com")
        self.g("config", "user.name", "T")
        self.commit("base", "c1")
        # main content target
        self.g("update-ref", "refs/remotes/origin/main", "HEAD")
        # integration branch off main + remote ref
        self.g("checkout", "-q", "-b", "integration")
        self.commit("int", "c2")
        self.g("update-ref", "refs/remotes/origin/integration", "HEAD")
        # tracked remote branches must exist as refs/remotes/*
        self.git = bh.RealGit(self.repo)

    def tearDown(self):
        self.dir.cleanup()

    def g(self, *args):
        return subprocess.run(["git", "-C", self.repo, *args],
                              capture_output=True, text=True, check=True).stdout

    def commit(self, msg, fname):
        with open(os.path.join(self.repo, fname), "w") as fh:
            fh.write(msg)
        self.g("add", "-A")
        self.g("commit", "-q", "-m", msg)

    def make_branch(self, name, from_ref="integration", msg="lane work",
                    push_remote=True, new_commit=True):
        self.g("checkout", "-q", "-B", name, from_ref)
        if new_commit:
            self.commit(msg, f"{name.replace('/', '_')}.txt")
        if push_remote:
            self.g("update-ref", f"refs/remotes/origin/{name}", "HEAD")
        self.g("checkout", "-q", "integration")
        return self.g("rev-parse", "HEAD").strip()

    def manifest(self, gh, **kw):
        return bh.build_manifest(self.git, gh, make_args(**kw))

    def by(self, rows, name):
        return next(r for r in rows if r.branch == name)

    # -- squash-merge verification ------------------------------------------

    def test_merged_ancestor_is_verified(self):
        self.make_branch("merged-lane", new_commit=False)
        gh = FakeGh(prs=[{"number": 5, "headRefName": "merged-lane",
                          "baseRefName": "main", "state": "MERGED",
                          "mergeCommit": None}])
        row = self.by(self.manifest(gh), "merged-lane")
        self.assertEqual(row.category, "SAFE_TO_DELETE")
        self.assertTrue(row.content_verified)

    def test_squash_merge_reachable_commit_is_verified(self):
        # lane commits NOT ancestors (unique objects), but a squash commit
        # on main carries the content → reachable mergeCommit proves it
        sha = self.make_branch("squashed-lane")
        self.g("checkout", "-q", "main")
        self.g("merge", "--squash", "squashed-lane")
        self.commit("squash of lane", "squashed_lane.txt")
        self.g("update-ref", "refs/remotes/origin/main", "HEAD")
        self.g("checkout", "-q", "integration")
        gh = FakeGh(prs=[{"number": 6, "headRefName": "squashed-lane",
                          "baseRefName": "main", "state": "MERGED",
                          "mergeCommit": {"oid": self.g("rev-parse", "HEAD").strip()} if False else None}])
        # mergeCommit points at the squash commit on main
        squash_sha = subprocess.run(
            ["git", "-C", self.repo, "rev-parse", "main"],
            capture_output=True, text=True).stdout.strip()
        gh = FakeGh(prs=[{"number": 6, "headRefName": "squashed-lane",
                          "baseRefName": "main", "state": "MERGED",
                          "mergeCommit": {"oid": squash_sha}}])
        row = self.by(self.manifest(gh), "squashed-lane")
        self.assertTrue(row.content_verified, row.content_evidence)
        self.assertEqual(row.category, "SAFE_TO_DELETE")

    def test_merged_label_with_unreachable_content_goes_to_review(self):
        self.make_branch("ghost-merge")
        gh = FakeGh(prs=[{"number": 8, "headRefName": "ghost-merge",
                          "baseRefName": "main", "state": "MERGED",
                          "mergeCommit": {"oid": "0" * 40}}])  # not reachable
        row = self.by(self.manifest(gh), "ghost-merge")
        self.assertFalse(row.content_verified)
        self.assertEqual(row.category, "NEEDS_REVIEW")
        self.assertFalse(row.delete_safe)

    # -- open PRs ------------------------------------------------------

    def test_open_pr_contained_is_supersede_candidate(self):
        self.make_branch("stale-open", new_commit=False)
        gh = FakeGh(prs=[{"number": 90, "headRefName": "stale-open",
                          "baseRefName": "main", "state": "OPEN",
                          "mergeCommit": None}])
        row = self.by(self.manifest(gh), "stale-open")
        self.assertEqual(row.category, "SAFE_TO_DELETE")
        self.assertIn("superseded", row.reason)

    def test_open_pr_with_unique_commits_is_review(self):
        self.make_branch("live-lane")
        # integration moves on so the lane has unique commits
        self.commit("int advanced", "c3.txt")
        self.g("update-ref", "refs/remotes/origin/integration", "HEAD")
        gh = FakeGh(prs=[{"number": 91, "headRefName": "live-lane",
                          "baseRefName": "main", "state": "OPEN",
                          "mergeCommit": None}])
        row = self.by(self.manifest(gh), "live-lane")
        self.assertEqual(row.category, "NEEDS_REVIEW")
        self.assertFalse(row.delete_safe)

    # -- stacked PRs and worktrees ------------------------------------------

    def test_base_of_open_stacked_pr_is_keep_active(self):
        self.make_branch("stack-base")
        gh = FakeGh(prs=[
            {"number": 92, "headRefName": "stack-base", "baseRefName": "main",
             "state": "MERGED", "mergeCommit": None},
            {"number": 93, "headRefName": "child", "baseRefName": "stack-base",
             "state": "OPEN", "mergeCommit": None},
        ])
        row = self.by(self.manifest(gh), "stack-base")
        self.assertEqual(row.category, "KEEP_ACTIVE")
        self.assertIn("#93", row.dependent_prs)

    def test_branch_in_dirty_worktree_is_keep_active(self):
        self.make_branch("wt-branch")
        # a real worktree checking it out, then make it dirty
        wt_dir = os.path.join(self.dir.name, "wt")
        self.g("worktree", "add", "-q", "--checkout", wt_dir, "wt-branch")
        with open(os.path.join(wt_dir, "untracked.txt"), "w") as fh:
            fh.write("dirty")
        try:
            gh = FakeGh(prs=[{"number": 94, "headRefName": "wt-branch",
                              "baseRefName": "main", "state": "MERGED",
                              "mergeCommit": None}])
            row = self.by(self.manifest(gh), "wt-branch")
            self.assertEqual(row.category, "KEEP_ACTIVE")
            self.assertFalse(row.worktree_clean)
            self.assertFalse(row.delete_safe)
        finally:
            self.g("worktree", "remove", "--force", wt_dir)

    def test_recent_ci_run_is_keep_active(self):
        self.make_branch("ci-hot")
        gh = FakeGh(prs=[{"number": 95, "headRefName": "ci-hot",
                          "baseRefName": "main", "state": "MERGED",
                          "mergeCommit": None}],
                    runs={"ci-hot": [{"status": "in_progress",
                                      "createdAt": "now"}]})
        row = self.by(self.manifest(gh), "ci-hot")
        self.assertEqual(row.category, "KEEP_ACTIVE")

    # -- hard failures --------------------------------------------------

    def test_github_unavailable_is_hard_error(self):
        self.make_branch("any")
        with self.assertRaises(bh.GithubUnavailable):
            bh.build_manifest(self.git, FakeGh(fail=True), make_args())

    def test_render_refuses_candidates_on_fetch_failure(self):
        # fetch_refs failure surfaces as FetchFailed with exit code 3
        class BrokenGit(bh.RealGit):
            pass

        def broken_fetch(_git):
            raise bh.FetchFailed("network down")

        self.assertEqual(bh.FetchFailed.exit_code, 3)
        with self.assertRaises(bh.FetchFailed):
            broken_fetch(BrokenGit(self.repo))

    def test_no_pr_contained_legacy_branch_is_safe(self):
        self.make_branch("legacy-sync", new_commit=False)
        row = self.by(self.manifest(FakeGh()), "legacy-sync")
        self.assertEqual(row.category, "SAFE_TO_DELETE")
        self.assertTrue(row.content_verified)

    def test_base_branch_never_candidate_even_without_ci(self):
        gh = FakeGh(prs=[{"number": 3, "headRefName": "integration",
                          "baseRefName": "main", "state": "OPEN",
                          "mergeCommit": None}])
        row = self.by(self.manifest(gh), "integration")
        self.assertEqual(row.category, "KEEP_ACTIVE")
        self.assertFalse(row.delete_safe)


        gh = FakeGh(prs=[{"number": 1, "headRefName": "main",
                          "baseRefName": "x", "state": "MERGED",
                          "mergeCommit": None}])
        row = self.by(self.manifest(gh), "main")
        self.assertEqual(row.category, "PROTECTED")
        self.assertFalse(row.delete_safe)

    def test_manifest_is_the_only_source_for_candidates(self):
        # render() must not invent branches: candidates == rows marked safe
        rows = self.manifest(FakeGh(prs=[
            {"number": 2, "headRefName": "merged-lane", "baseRefName": "main",
             "state": "MERGED", "mergeCommit": None}]))
        self.make_branch("merged-lane")
        rows = self.manifest(FakeGh(prs=[
            {"number": 2, "headRefName": "merged-lane", "baseRefName": "main",
             "state": "MERGED", "mergeCommit": None}]))
        text = bh.render(rows)
        marked = {r.branch for r in rows if r.delete_safe}
        for line in text.splitlines():
            if line.strip().startswith("git push origin --delete"):
                self.assertIn(line.split()[-1], marked)
        self.assertEqual(len([r for r in rows if r.delete_safe]),
                         text.count("git push origin --delete"))


if __name__ == "__main__":
    unittest.main()
