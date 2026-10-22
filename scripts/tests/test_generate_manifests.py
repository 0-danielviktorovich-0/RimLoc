"""Tests for testlab/scripts/generate_manifests.py About.xml hardening.

Run: python3 -m unittest discover -s scripts/tests
Adversarial fixtures are built inline (tempfiles); nothing external.
Layout note: scripts/ has no __init__ packaging; we import by path.
"""
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

_SCRIPT = Path(__file__).resolve().parents[2] / "testlab" / "scripts" / "generate_manifests.py"
_spec = importlib.util.spec_from_file_location("generate_manifests", _SCRIPT)
gm = importlib.util.module_from_spec(_spec)
sys.modules.setdefault("generate_manifests", gm)
_spec.loader.exec_module(gm)

GOOD_ABOUT = """<?xml version="1.0" encoding="utf-8"?>
<ModMetaData>
  <name>Canary Mod</name>
  <packageId>canary.mod</packageId>
  <supportedVersions><li>1.6</li></supportedVersions>
</ModMetaData>
"""

BOMB_ABOUT = """<?xml version="1.0"?>
<!DOCTYPE lolz [<!ENTITY lol "lol"><!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;">]>
<ModMetaData><name>&lol2;</name></ModMetaData>
"""


class AboutParsingHardening(unittest.TestCase):
    def _write(self, text: str, *, name: str = "About.xml") -> Path:
        d = tempfile.TemporaryDirectory()
        self.addCleanup(d.cleanup)
        p = Path(d.name) / name
        p.write_text(text, encoding="utf-8")
        return p

    def test_good_about_still_parses(self):
        meta = gm.parse_about(str(self._write(GOOD_ABOUT)))
        self.assertEqual(meta["packageId"], "canary.mod")
        self.assertEqual(meta["versions"], ["1.6"])

    def test_entity_bomb_is_refused_before_parsing(self):
        meta = gm.parse_about(str(self._write(BOMB_ABOUT)))
        self.assertTrue(meta["name"].startswith("<xml-error:"))
        self.assertEqual(meta["packageId"], "")

    def test_doctype_without_entities_is_refused(self):
        meta = gm.parse_about(
            str(self._write('<?xml version="1.0"?>\n<!DOCTYPE ModMetaData>\n<ModMetaData/>'))
        )
        self.assertTrue(meta["name"].startswith("<xml-error:"))

    def test_oversized_about_is_refused(self):
        meta = gm.parse_about(str(self._write("<ModMetaData>" + "x" * (gm.MAX_ABOUT_BYTES + 10))))
        self.assertTrue(meta["name"].startswith("<xml-error:"))

    def test_growing_file_is_refused(self):
        meta = gm.parse_about(str(self._write("<ModMetaData>" + "y" * gm.MAX_ABOUT_BYTES)))
        self.assertTrue(meta["name"].startswith("<xml-error:"))


if __name__ == "__main__":
    unittest.main()
