# Independent visual critique — 2026-10-21 (Stage F, product-ui-design step 11)

Fresh-context reviewer, workspace/settings/projects/checks/export/aitranslate,
light+dark. Full text preserved in the agent transcript; verdict scores:
HIERARCHY 7 · SPACING 6 · DENSITY 6 · READABILITY 6 · CONSISTENCY 5.

## Top fixes (reviewer priority)
1. One form measure + whole buttons: shared ~720px max-width on settings AND
   aitranslate; buttons nowrap (export CTAs wrap to two lines today); remove
   the orphan pink strip on aitranslate (~290×20 under the key hint).
2. Workspace hero eats ~250px of the working screen: compress to one row;
   orphan «Русский ⇅» select merges into the TRANSLATION column header;
   remove the full-width empty tabs border.
3. Navigation language/status honesty: «nav.aitranslate» raw key in the
   sidebar; two clipped labels; «0 ошибки/0 предупреждения» wrong plural on
   the checks hero; dark-theme primary button loses weight; footer lacks a
   scrim (content clips under it).

## What got better with wave 1+2 (reviewer-confirmed)
- Settings column bounded at 720px; hairline now matches the column width.
- Capability table right-aligned values — good use of the column.
- Core list/file-tree density is right for the workstation profile.

## Round-2 resolution (2026-10-21, commit follows)

1. **Workspace hero compressed** — h1 30→20px, band paddings on-scale; 7
   entry rows visible at the default window (was 6). AFTER:
   `workspace-hero-after.png`.
2. **Footer scrim** — solid background + soft top shadow; captured in
   `settings-dark-after.png`.
3. **Orphan strip IDENTIFIED — not a defect**: it is the ENABLED primary
   button «Начать перевод» clipped by the viewport fold in the audit
   capture. Scrolled into view it is a normal, clearly-labelled primary
   button (`aitranslate-dark-after.png`; computed label = «Начать перевод»,
   opacity 1). The washed-out disabled variant that started the suspicion
   is fixed by the legible-disabled rule (round-2 CSS).
4. **Disabled controls legible** — flat opacity replaced by a
   saturate/brightness mix + explicit btn-primary disabled fill.

Remaining from the critique (lower priority, open): card alignment on
Projects, Russian plural «0 ошибки», chips RU labels, «Русский ⇅» placement.
