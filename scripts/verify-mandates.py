#!/usr/bin/env python3
"""Verify mandate filename <-> mandate_id identity (G5 correction).

Mandate files must carry a machine-readable identity comment:
  <!-- mandate_id: <id> | wave: <wave> | scope: <scope> -->
File timestamps/order must never be used to identify mandates.
"""
import pathlib, sys

REPO = pathlib.Path(__file__).resolve().parents[1]
EXPECTED = {
    "GUI_SPRINT_MANDATE.md": "g4-sprint",
    "GUI_QA_MANDATE.md": "g5-qa",
    "LANGUAGE_REGISTRY_MANDATE.md": "g5-languages",
    "CHAT_BATCH_MANDATE.md": "g5-chat-batch",
    "MULTI_TARGET_MANDATE.md": "g5-multi-target",
    "MOCK_LIVE_ONBOARDING_MANDATE.md": "g5-mock-live",
    "SOURCE_INSPECTOR_MANDATE.md": "g5-source-inspector",
    "APPEARANCE_PACK_MANDATE.md": "g6-appearance",
    "UI_SDK_MANDATE.md": "g6-ui-sdk",
}
failed = 0
for fname, mid in sorted(EXPECTED.items()):
    p = REPO / "docs" / "development" / fname
    if not p.is_file():
        print(f"FAIL missing: {fname}")
        failed += 1
        continue
    head = p.read_text()[:400]
    if f"mandate_id: {mid}" not in head:
        print(f"FAIL identity mismatch: {fname} (expected {mid})")
        failed += 1
    else:
        print(f"ok  {fname} = {mid}")
sys.exit(1 if failed else 0)
