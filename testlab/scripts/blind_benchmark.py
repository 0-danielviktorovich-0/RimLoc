#!/usr/bin/env python3
"""Blind extraction benchmark (§8): RimLoc source inventory vs reference translation.

Protocol: extract EN source inventory with `rimloc scan --lang en`, extract the
reference RU inventory from the SAME mod's Languages/Russian, compare KEY SETS.
Reference = expected entry inventory (provenance: human/tool-assisted — see
TRANSLATION_BENCHMARK.md). Reference may intentionally omit content; mismatches
are adjudicated categories, not automatic failures.

Usage: python3 testlab/scripts/blind_benchmark.py --run
Writes: testlab/benchmarks/extraction-<ts>.json + .md
"""
import argparse
import json
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
MODS = Path("/Applications/RimWorld.app/Mods")

# Curated diverse corpus: (workshop_id, note)
CORPUS = [
    ("1814383360", "VE weapons; DefInjected-heavy; partial human RU; LoadFolders+versions"),
    ("2927850179", "aggregate RU language pack (OW.RU.Furniture); many LoadFolders submods; WordInfo"),
    ("2805854947", "Vanilla Genetics Expanded + RU (127 files; largest local reference)"),
    ("3626232280", "Vanilla Quests Expanded - Ancients RU (64 files; quest/RulePack-heavy)"),
]

PLACEHOLDER_HINTS = ("{", "}", "%s", "%d")


def scan(root: Path, lang: str, out: Path) -> dict[str, str]:
    """Run rimloc scan for a language, return key -> value/source map."""
    out.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            str(REPO / "target/debug/rimloc-cli"),
            "--quiet", "scan", "--root", str(root),
            "--lang", lang, "--format", "json", "--out-json", str(out),
        ],
        check=True, capture_output=True,
    )
    units = json.loads(out.read_text())
    inv: dict[str, str] = {}
    for u in units:
        # dedupe: first occurrence wins (scanner is deterministic)
        inv.setdefault(u["key"], u.get("value") or u.get("source") or "")
    return inv


def categorize(keys: list[str], root: Path, lang_dir_name: str) -> Counter:
    """Rough category breakdown by which folder the keys' files live in."""
    cats: Counter = Counter()
    # Re-scan with paths: cheap second pass using the same JSON is avoided; we
    # instead classify by key shape heuristics (dotted def path vs plain Keyed).
    for k in keys:
        if "." in k and k.split(".", 1)[0][:1].isupper():
            cats["DefInjected"] += 1
        else:
            cats["Keyed"] += 1
    return cats


def benchmark_one(rid: str, note: str, workdir: Path) -> dict:
    root = MODS / rid
    en = scan(root, "en", workdir / f"{rid}-en.json")
    ru = scan(root, "ru", workdir / f"{rid}-ru.json")
    en_keys, ru_keys = set(en), set(ru)
    matched = en_keys & ru_keys
    missed = en_keys - ru_keys          # RimLoc saw it, reference lacks it
    extra = ru_keys - en_keys           # reference has it, RimLoc did not extract

    ref = len(ru_keys)
    precision = len(matched) / len(en_keys) if en_keys else 0.0
    recall = len(matched) / ref if ref else 0.0

    missed_cat = categorize(sorted(missed), root, "Russian")
    extra_cat = categorize(sorted(extra), root, "Russian")
    return {
        "workshop_id": rid,
        "note": note,
        "source_entries": len(en_keys),
        "reference_entries": ref,
        "matched": len(matched),
        "precision_vs_reference": round(precision, 4),
        "recall_vs_reference": round(recall, 4),
        "missed_by_reference": len(missed),
        "extra_in_reference": len(extra),
        "missed_categories": dict(missed_cat),
        "extra_categories": dict(extra_cat),
        "missed_sample": sorted(missed)[:15],
        "extra_sample": sorted(extra)[:15],
        "placeholder_in_source_missed": sum(
            1 for k in missed if any(h in (en.get(k) or "") for h in PLACEHOLDER_HINTS)
        ),
    }


def markdown(results: list[dict]) -> str:
    lines = [
        "# Blind extraction benchmark (RimLoc vs reference inventory)",
        "",
        f"Generated: {datetime.now(timezone.utc).isoformat()}",
        "",
        "Protocol: EN extracted by RimLoc (`scan --lang en`) blind to the reference;",
        "reference = RU files shipped in the same mod (provenance: human/tool-assisted,",
        "see COMPETITOR_MATRIX.md workflow archaeology). Reference may intentionally",
        "omit content — mismatches are adjudication inputs, not automatic failures.",
        "",
        "| Mod | Note | Source | Reference | Matched | Precision | Recall | Ref-omits | Ref-extra |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for r in results:
        lines.append(
            f"| {r['workshop_id']} | {r['note'][:48]} | {r['source_entries']} | "
            f"{r['reference_entries']} | {r['matched']} | "
            f"{r['precision_vs_reference']:.1%} | {r['recall_vs_reference']:.1%} | "
            f"{r['missed_by_reference']} | {r['extra_in_reference']} |"
        )
    lines.append("")
    for r in results:
        lines.append(f"## {r['workshop_id']} — {r['note']}")
        lines.append(f"- Categories of reference-omitted keys: {r['missed_categories']}")
        lines.append(f"- Categories of reference-extra keys: {r['extra_categories']}")
        lines.append(f"- Reference-omitted keys carrying placeholders: {r['placeholder_in_source_missed']}")
        lines.append(f"- Sample reference-extra (RimLoc missed): {r['extra_sample'][:8]}")
        lines.append("")
    return "\n".join(lines)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", action="store_true")
    ap.add_argument("--out-dir", default=str(REPO / "testlab/benchmarks"))
    args = ap.parse_args()
    if not args.run:
        print(__doc__)
        sys.exit(0)

    workdir = Path(args.out_dir) / "raw"
    workdir.mkdir(parents=True, exist_ok=True)
    results = []
    for rid, note in CORPUS:
        print(f"benchmarking {rid} …", flush=True)
        try:
            results.append(benchmark_one(rid, note, workdir))
        except subprocess.CalledProcessError as e:
            print(f"  FAIL {rid}: {e.stderr.decode()[-300:]}", file=sys.stderr)

    ts = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    out = Path(args.out_dir)
    (out / f"extraction-{ts}.json").write_text(json.dumps(results, ensure_ascii=False, indent=1))
    (out / f"extraction-{ts}.md").write_text(markdown(results), encoding="utf-8")
    print(markdown(results))


if __name__ == "__main__":
    main()
