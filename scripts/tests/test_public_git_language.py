#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts" / "check-public-git-language.py"
spec = importlib.util.spec_from_file_location("public_git_language", CHECKER)
assert spec and spec.loader
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


class PublicGitLanguageTests(unittest.TestCase):
    def assertClean(self, text: str, surface: str = "commit-subject") -> None:
        self.assertEqual([], mod.check_text(text, surface))

    def assertRejected(self, text: str, surface: str = "commit-subject") -> None:
        self.assertTrue(mod.check_text(text, surface), text)

    def test_normal_engineering_subjects_pass(self) -> None:
        self.assertClean("feat(gui): show live source provenance")
        self.assertClean("fix(llm): support keychain-only provider builds")
        self.assertClean("docs(release): reconcile rel23 verification evidence")
        self.assertClean("build(macos): package the app as a zip")
        self.assertClean("docs(ui): document the UI R1 product contract")
        self.assertClean("fix(parser): preserve wave-shaped source text")

    def test_internal_protocol_subjects_fail(self) -> None:
        self.assertRejected("docs(review): DO_NOW iteration 1 evidence fixes")
        self.assertRejected("docs(release): close owner-gates")
        self.assertRejected("docs(process): R4 audit findings")
        self.assertRejected("chore(repo): wave integration 2")
        self.assertRejected("docs(repo): reviewer bridge iteration 3")
        self.assertRejected("docs(repo): novyj mandat dlya proverki")

    def test_cyrillic_public_titles_fail(self) -> None:
        self.assertRejected("fix(gui): исправить переключатель языка")
        self.assertRejected("Обновить релиз", "pr-title")

    def test_public_branch_names(self) -> None:
        self.assertClean("feat/source-inspector", "branch-name")
        self.assertClean("fix/provider-feature-gates", "branch-name")
        self.assertRejected("release/wave-integration-2", "branch-name")
        self.assertRejected("wf-ui-final", "branch-name")

    def test_changelog_uses_product_language(self) -> None:
        self.assertClean(
            "- [gui] Provider status now reflects a real connection test.",
            "changelog",
        )
        self.assertRejected(
            "- [gui] Owner mandate wave 4 provider gate is complete.",
            "changelog",
        )

    def test_ui_r1_is_not_treated_as_an_internal_phase(self) -> None:
        self.assertClean("docs(ui): update UI R1 accessibility guidance")


if __name__ == "__main__":
    unittest.main()
