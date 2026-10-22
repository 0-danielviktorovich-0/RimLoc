#!/usr/bin/env python3
"""Generate testlab manifests from locally installed RimWorld mods.

READ-ONLY: inspects the game's Mods directory and never writes there.
Outputs:
  testlab/manifests/installed-mods.json  — full inventory with recursive
      Languages/<lang> detection (handles versioned layouts like
      1.6/Main/Languages/Russian and submod aggregates).
  testlab/manifests/real-mods.json       — curated deep manifests for the
      canonical regression mods (source-of-truth test cases).

Usage: python3 testlab/scripts/generate_manifests.py [--mods-root PATH] [--out-dir PATH]
"""
import argparse
import hashlib
import json
import os
import re
import sys

# Third-party About.xml reaches this generator as-is, so parsing goes
# through defusedxml unconditionally (the one XML hardening library the
# CodeQL py/xml-bomb query recognizes). No fallback to plain ElementTree:
# a missing dependency must fail loudly, never parse unsafely.
try:
    from defusedxml.ElementTree import fromstring as _defused_fromstring
except ImportError as _exc:  # pragma: no cover - environment guard
    raise SystemExit(
        "defusedxml is required for safe About.xml parsing: pip install defusedxml"
    ) from _exc

ET_ParseError = __import__("xml.etree.ElementTree", fromlist=["ParseError"]).ParseError

DEFAULT_MODS_ROOT = "/Applications/RimWorld.app/Mods"
CURATED = ["1814383360", "2927850179", "2126925929"]
VANILLA_TAR = "/Applications/RimWorld.app/Data/Core/Languages/Russian (Русский).tar"

# Third-party About.xml files reach this generator as-is, so parsing gets a
# hard attack-surface cut (CodeQL py/xml-bomb): refuse DTDs/entities
# outright (billion-laughs needs an entity table) and cap the input size.
# stdlib entity-expansion limits alone are not a defense at old Expat
# versions (<2.7.2), and the tool never needs a DTD for RimWorld About.xml.
MAX_ABOUT_BYTES = 1 * 1024 * 1024


def _load_about_text(about_path):
    size = os.path.getsize(about_path)
    if size > MAX_ABOUT_BYTES:
        raise ET_ParseError(f"About.xml exceeds {MAX_ABOUT_BYTES} bytes ({size})")
    with open(about_path, "rb") as f:
        raw = f.read(MAX_ABOUT_BYTES + 1)
    if len(raw) > MAX_ABOUT_BYTES:
        raise ET_ParseError("About.xml grew past the size cap while reading")
    text = raw.decode("utf-8", "strict")
    if re.search(r"<!\s*(DOCTYPE|ENTITY)", text, re.IGNORECASE):
        raise ET_ParseError("About.xml with a DTD/entity declaration is refused")
    return text


def parse_about(about_path):
    try:
        root = _defused_fromstring(_load_about_text(about_path))
    except (ET_ParseError, UnicodeDecodeError, OSError) as e:
        return {"name": f"<xml-error: {e}>", "packageId": "", "versions": []}
    return {
        "name": (root.findtext("name") or "").strip(),
        "packageId": (root.findtext("packageId") or "").strip(),
        "description": ((root.findtext("description") or "")[:300]).strip(),
        "versions": [v.text.strip() for v in root.findall("supportedVersions/li") if v.text],
        "modVersion": (root.findtext("modVersion") or "").strip() or None,
        "modVersionIgnore": root.find("modVersion") is not None and (root.findtext("modVersion") or "").strip().lower() in {"ignore", ""},
    }


def find_language_dirs(mod_path):
    """All Languages/<lang> directories anywhere in the mod tree (versioned layouts)."""
    found = {}
    for dirpath, dirnames, _ in os.walk(mod_path):
        if os.path.basename(dirpath) == "Languages":
            parent = os.path.relpath(dirpath, mod_path)
            for d in dirnames:
                full = os.path.join(dirpath, d)
                if os.path.isdir(full) and not d.startswith("."):
                    found.setdefault(d, []).append(parent)
    return found


def count_files(path, exts=(".xml", ".txt")):
    n = 0
    for dp, _, fs in os.walk(path):
        n += sum(1 for f in fs if f.endswith(exts))
    return n


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def deep_manifest(mod_path, entry):
    """Curated manifest entry: submod structure, RU translation layout, counts."""
    langs = find_language_dirs(mod_path)
    ru_locations = [
        {"root": loc, "files": count_files(os.path.join(mod_path, loc, "Russian"))}
        for loc in langs.get("Russian", [])
    ]
    en_locations = [
        {"root": loc, "files": count_files(os.path.join(mod_path, loc, "English"))}
        for loc in langs.get("English", [])
    ]
    has_loadfolders = os.path.isfile(os.path.join(mod_path, "LoadFolders.xml"))
    def_dirs = sorted(
        d for d in os.listdir(mod_path)
        if os.path.isdir(os.path.join(mod_path, d, "Defs")) or d == "Defs"
    )
    patches = sum(
        1 for dp, _, fs in os.walk(mod_path)
        for f in fs if f.endswith(".xml") and "Patches" in dp
    )
    entry.update({
        "languages_recursive": {k: v for k, v in sorted(langs.items())},
        "russian_translation": ru_locations,
        "english_source": en_locations,
        "load_folders": has_loadfolders,
        "defs_roots": def_dirs[:10],
        "patch_files": patches,
        "about_sha256": sha256_file(os.path.join(mod_path, "About", "About.xml")),
        "total_files": sum(len(fs) for _, _, fs in os.walk(mod_path)),
        "known_special_features": special(mod_path),
    })
    return entry


def special(mod_path):
    feats = []
    if os.path.isdir(os.path.join(mod_path, "Patches")):
        feats.append("patches")
    if os.path.isdir(os.path.join(mod_path, "Common", "Patches")):
        feats.append("patches-versioned")
    for dp, dirnames, _ in os.walk(mod_path):
        if any(f == "LoadFolders.xml" for f in _) or "WordInfo" in dirnames:
            if "WordInfo" in dirnames:
                feats.append("wordinfo")
                break
    if any("ModSettings" in f or "XmlExtensions" in f
           for dp, _, fs in os.walk(mod_path) for f in fs if f.endswith(".xml")):
        feats.append("settings-framework-usage")
    return feats


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mods-root", default=DEFAULT_MODS_ROOT)
    ap.add_argument("--out-dir", default=os.path.join(os.path.dirname(__file__), "..", "manifests"))
    args = ap.parse_args()
    mods_root = os.path.expanduser(args.mods_root)
    out_dir = os.path.abspath(args.out_dir)
    os.makedirs(out_dir, exist_ok=True)

    if not os.path.isdir(mods_root):
        print(f"ERROR: mods root not found: {mods_root}", file=sys.stderr)
        sys.exit(1)

    inventory = []
    for wid in sorted(os.listdir(mods_root)):
        p = os.path.join(mods_root, wid)
        about = os.path.join(p, "About", "About.xml")
        if not (os.path.isdir(p) and os.path.isfile(about)):
            continue
        info = parse_about(about)
        info.update({
            "workshop_id": wid,
            "languages": sorted(find_language_dirs(p).keys()),
            "ru_files_top": count_files(os.path.join(p, "Languages", "Russian"))
            if os.path.isdir(os.path.join(p, "Languages", "Russian")) else 0,
            "is_vanilla_expanded": bool(re.search(r"vanilla", info["name"], re.I)),
        })
        inventory.append(info)

    with open(os.path.join(out_dir, "installed-mods.json"), "w") as f:
        json.dump({"generated": "local", "mods_root": mods_root, "count": len(inventory),
                   "mods": inventory}, f, ensure_ascii=False, indent=1)

    curated = [deep_manifest(os.path.join(mods_root, wid),
                             next(m for m in inventory if m["workshop_id"] == wid).copy())
               for wid in CURATED]
    reference = {"vanilla_ru_tar": {"path": VANILLA_TAR,
                                    "sha256": sha256_file(VANILLA_TAR) if os.path.isfile(VANILLA_TAR) else None}}
    with open(os.path.join(out_dir, "real-mods.json"), "w") as f:
        json.dump({"curated": curated, "reference": reference}, f, ensure_ascii=False, indent=1)

    ve = [m for m in inventory if m["is_vanilla_expanded"]]
    ru = [m for m in inventory if m["ru_files_top"] > 0]
    print(f"inventory: {len(inventory)} mods, {len(ve)} Vanilla Expanded, {len(ru)} with top-level RU")
    print(f"curated manifests: {[m['name'] for m in curated]}")
    print(f"vanilla reference sha256: {reference['vanilla_ru_tar']['sha256'][:16]}…")


if __name__ == "__main__":
    main()
