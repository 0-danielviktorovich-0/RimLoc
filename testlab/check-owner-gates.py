#!/usr/bin/env python3
"""OWNER_GATE admission gate (R4 §2) — machine-checkable ledger check.

Каждая запись ownerGates в .rimloc-release-state.json обязана иметь reason
из канонического enum. Технически выполнимая задача owner-gate'ом быть не
может. Fail closed: неизвестный reason или отсутствие reason → exit 1.
"""
import json
import sys
from pathlib import Path

ALLOWED = {
    "SUBJECTIVE_VISUAL",
    "OWNER_CREDENTIAL",
    "EXTERNAL_ACCOUNT_ACTION",
    "SIGNING_NOTARIZATION",
    "PUBLISH_AUTHORIZATION",
    "PHYSICAL_OR_GAME_VISUAL_CONFIRMATION",
}

# Сопоставление известных гейтов каноническим reason'ам (для автопочинки).
KNOWN_MAPPING = {
    "level-7": "PHYSICAL_OR_GAME_VISUAL_CONFIRMATION",
    "level-7 visual": "PHYSICAL_OR_GAME_VISUAL_CONFIRMATION",
    "publish": "PUBLISH_AUTHORIZATION",
    "publish approval": "PUBLISH_AUTHORIZATION",
    "z.ai live e2e": "OWNER_CREDENTIAL",
    "ai provider live e2e": "OWNER_CREDENTIAL",
    "ai-провайдеры live": "OWNER_CREDENTIAL",
    "signing/notarization": "SIGNING_NOTARIZATION",
}


def main() -> int:
    state = Path(__file__).resolve().parent.parent / ".rimloc-release-state.json"
    d = json.loads(state.read_text(encoding="utf-8"))
    gates = d.get("ownerGates", [])
    problems = []
    normalized = []
    for g in gates:
        if isinstance(g, dict):
            name = g.get("name", "")
            reason = g.get("reason", "")
        else:
            name = str(g)
            reason = ""
        key = name.lower().strip()
        if not reason and key in KNOWN_MAPPING:
            reason = KNOWN_MAPPING[key]
        if reason not in ALLOWED:
            problems.append(f"  {name!r}: reason={reason!r} не в enum / отсутствует")
        normalized.append({"name": name, "reason": reason} if reason else {"name": name})
    if problems:
        print("OWNER_GATE LEDGER: FAIL — записи без канонического reason:")
        for p in problems:
            print(p)
        print("Исправьте: каждой записи — reason из:", ", ".join(sorted(ALLOWED)))
        return 1
    print(f"OWNER_GATE LEDGER: PASS ({len(normalized)} запис(и), все с каноническим reason)")
    for n in normalized:
        print(f"  - {n['name']} [{n['reason']}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
