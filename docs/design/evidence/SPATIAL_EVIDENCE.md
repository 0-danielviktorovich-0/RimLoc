# Spatial wave evidence (2026-10-21)

Build: feat/ui-spatial-quality @ 4842e94a (waves 1+2 on the token scale),
automation bundle rebuilt from this tree, run through the standard
visual-audit lane: 36/36 checks PASS, both themes, 16 surfaces.

## Deterministic probe (live app)

.narrow-page computed maxWidth = 720px (offsetWidth 718 at a 980px
viewport) — the settings cap is live; probe PASS.

## BEFORE → AFTER → WHY BETTER

| Surface | BEFORE | AFTER | Why better |
|---|---|---|---|
| Settings (wide window) | fields stretched full width (~900px+), title/capability rows reading distance unbounded | 720px calm column, structured sections, air to the right (settings-wide-after.png) | reading distance bounded; Wealthfolio-grade composition; scannability of the capability table up |
| Entry rows | 13px off-scale horizontal padding | 12px (--sp-3) | one grid step; density preserved (compact default, ≥8 rows at 720p) |
| Heading band | 18px bottom padding off-scale | --sp-5 (20px) | rhythm consistent with panel padding |
| Footer/error bands | 28px sides off-scale | --sp-7 (32px) | aligns with page rhythm |
| Empty/passed states | 40/45px paddings | --sp-8 (40px) breathing | first-class empties |
| Dialogs | 30px padding, 18px action gap | --sp-6 / --sp-5 | on-scale, consistent |

## Honest notes

- At the default 980×640 window the settings change is a no-op (content
  already narrower than 720px) — the improvement shows on wide windows;
  BEFORE evidence: shots-before/settings-light.png (visual parity with old
  build), AFTER: shots/settings-wide-after.png at 1440×900.
- The visual-audit suite itself is contrast/structure-first; spacing is
  verified by probes + wide-window captures, not by pixel diff (Lovable
  pixel refs do not exist in the repo; contract-based comparison per
  UI_R1_VISUAL_PARITY method).
- Shots: /tmp/rimloc-int/shots/ (after) and /tmp/rimloc-int/shots-before/
  (before), 32+2 files each; report: visual-audit-after.json (36 PASS).
