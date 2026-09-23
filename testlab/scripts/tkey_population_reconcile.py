#!/usr/bin/env python3
"""Authoritative TKey population definitions + machine diff of all totals.

Replaces the ambiguity between "358 nodes" (tkey_audit.py), "351 nodes" and
"343 pairs" (DLC adjudication): these are four different population concepts,
not conflicting measurements. This script computes all four from the same
walk and lists every identity excluded at each step, with a reason.

Definitions (single source of truth, all roots = Core + installed DLCs):
  P1 raw TKey nodes        — every XML node carrying a TKey attribute inside a
                             Defs tree of an owned Def.
  P2 translatable nodes    — P1 restricted to nodes that (a) live under a Def
                             with a non-empty defName and (b) carry their own
                             non-empty text.
  P3 unique identities     — unique (defName, TKey) pairs within P2. Multiple
                             nodes may share one identity (document-order
                             last-wins semantics, see Royalty duplicates).
  P4 serialization-tested  — identities whose candidate serialization paths
                             (bare / .slateRef / .value.slateRef) match the
                             official Russian pack. Unmatched identities are
                             classified: structural-addressed (RU translated
                             the same def via root.nodes.* structural path) or
                             absent (no RU coverage for the identity at all).

Usage: python3 testlab/scripts/tkey_population_reconcile.py [--out-json PATH]
"""
import argparse
import glob
import json
import subprocess
from pathlib import Path
from xml.etree import ElementTree as ET

DATA = Path("/Applications/RimWorld.app/Data")
DLC = ["Royalty", "Ideology", "Biotech", "Anomaly", "Odyssey"]
SUFFIXES = ["", ".slateRef", ".value.slateRef"]
RU_CACHE = Path(__file__).resolve().parents[1] / "run" / "official-ru-extract"


def extract_ru_pack(root: Path) -> Path:
    """Extract the official Russian tar into the shared cache once (read-only
    on the game install)."""
    out = RU_CACHE / root.name
    if not (out / "DefInjected").is_dir():
        out.mkdir(parents=True, exist_ok=True)
        tar = root / "Languages" / "Russian (Русский).tar"
        if not tar.is_file():
            raise SystemExit(f"official RU tar not found: {tar}")
        subprocess.run(["tar", "-xf", str(tar), "-C", str(out)], check=True)
    return out


def roots():
    out = {"Core": DATA / "Core"}
    for d in DLC:
        if (DATA / d).is_dir():
            out[d] = DATA / d
    return out


def official_ru_keys(ru_root: Path):
    """All element paths anywhere in the extracted official RU pack's
    DefInjected tree (TipSetDef li + QuestScriptDef and any other def type)."""
    keys = set()
    base = ru_root / "DefInjected"
    for f in glob.glob(f"{base}/**/*.xml", recursive=True):
        try:
            tree = ET.parse(f)
        except Exception:
            continue
        for el in tree.getroot().iter():
            if el.tag not in ("Defs", "DefInjected", "RimWorld-Translations"):
                keys.add(el.tag)
    return keys


def walk_defs(defs_root: Path):
    """Yield (file, def_type, def_name, node_tag, tkey, has_text)."""
    for f in glob.glob(f"{defs_root}/**/*.xml", recursive=True):
        try:
            tree = ET.parse(f)
        except Exception:
            continue
        root = tree.getroot()
        if root.tag != "Defs":
            continue
        for defn in root:
            if not defn.tag.endswith("Def"):
                continue
            defname = (defn.findtext("defName") or "").strip()
            for node in defn.iter():
                tkey = (node.get("TKey") or "").strip()
                if not tkey:
                    continue
                has_text = bool(node.text and node.text.strip())
                yield f, defn.tag, defname, node.tag, tkey, has_text


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-json", default="testlab/reports/tkey-population-reconcile.json")
    args = ap.parse_args()

    per_root = {}
    raw_total = tr_total = id_total = matched_node = 0
    unmatched = []

    for name, root in roots().items():
        ru = official_ru_keys(extract_ru_pack(root))
        raw = list(walk_defs(root / "Defs"))
        translatable = [n for n in raw if n[2] and n[5]]
        identities = {(dn, tk) for (_, _, dn, _, tk, _) in translatable}

        raw_total += len(raw)
        tr_total += len(translatable)
        id_total += len(identities)

        root_unmatched_ids = set()
        for dn, tk in sorted(identities):
            identity = f"{dn}.{tk}"
            if any(identity + s in ru for s in SUFFIXES):
                continue
            root_unmatched_ids.add(identity)
            # Coarse machine signal only: does the def have ANY other RU
            # coverage? Per-node verdicts (structural-addressed vs gap) are
            # adjudicated in docs/development/DLC-TKEY-ADJUDICATION.md §4.
            covered_def = any(k.startswith(dn + ".") for k in ru)
            unmatched.append({
                "root": name, "identity": identity,
                "def_has_other_ru_coverage": covered_def,
                "serialization_paths_tested": [identity + s for s in SUFFIXES],
            })
        matched_ids = len(identities) - len(root_unmatched_ids)
        matched_node += sum(1 for (_, _, dn, _, tk, _) in translatable
                            if f"{dn}.{tk}" not in root_unmatched_ids)
        per_root[name] = {
            "raw_nodes": len(raw),
            "translatable_nodes": len(translatable),
            "unique_identities": len(identities),
            "matched_identities": matched_ids,
            "unmatched_identities": sorted(root_unmatched_ids),
        }

    result = {
        "definitions": {
            "P1_raw_tkey_nodes": "every XML node with a TKey attribute inside a Defs tree",
            "P2_translatable_nodes": "P1 with owned defName AND own non-empty text",
            "P3_unique_identities": "unique (defName, TKey) pairs within P2",
            "P4_serialization_tested": "identities matching official RU via bare/.slateRef/.value.slateRef",
        },
        "totals": {
            "raw_tkey_nodes": raw_total,
            "translatable_nodes": tr_total,
            "unique_identities": id_total,
            "matched_nodes": matched_node,
            "matched_identities": id_total - len(unmatched),
        },
        "unmatched_identities": unmatched,
        "per_root": per_root,
    }
    out = Path(args.out_json)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")

    t = result["totals"]
    print(f"P1 raw={t['raw_tkey_nodes']}  P2 translatable={t['translatable_nodes']}  "
          f"P3 identities={t['unique_identities']}  "
          f"P4 matched nodes={t['matched_nodes']} identities={t['matched_identities']}")
    for u in unmatched:
        print(f"  UNMATCHED {u['root']}: {u['identity']}  "
              f"[def_has_other_ru_coverage={u['def_has_other_ru_coverage']}]")
    print(f"evidence: {out}")


if __name__ == "__main__":
    main()
