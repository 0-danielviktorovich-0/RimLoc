---
title: "RimLoc Spatial Design System — space tokens and layout composition rules"
status: Draft (implementation lane feat/ui-spatial-quality)
date: 2026-10-20
deciders: owner directive (Wealthfolio-quality spatial reference), design lane
related:
  - RIMLOC_UI_R1_PRODUCT_CONTRACT.md
  - LOVABLE_R1_ADAPTATION.md
  - ../development/GUI_DESIGN_TOKENS.md
tags: [topic/design, kind/spec, topic/spatial]
---

# Spatial Design System (SDS)

Profile: **DESKTOP WORKSTATION / LOCALIZATION TOOL** (product contract).
Goal (owner): Wealthfolio-grade spatial organization — visual lightness,
calm, structured navigation, quality distances, grouping — WITHOUT losing
the Lovable identity (OKLCH warm+wine, Literata over Golos, hairline rhythm)
and WITHOUT de-densifying the translation editor.

Reference lessons adopted from the Wealthfolio audit (evidence: live site +
repo main@3.10.0, screenshots at assets.wealthfolio.app): collapsed-capable
narrow sidebar, ONE token source, closed type scale, glass page header,
empty states as first-class, settings as a separate calm layout. NOT
adopted: Flexoki palette (RimLoc keeps warm+wine), dashboard grid editing
(out of scope).

## 1. Space scale (STANDARD REQUIREMENT)

Base grid 4px. Closed scale — values outside the scale require a waiver:

| Token | px | Use |
|---|---|---|
| `--sp-1` | 4 | icon↔label inside a control, tight chips |
| `--sp-2` | 8 | related controls in one group, table cell padding-x tight |
| `--sp-3` | 12 | related items, entry-row grid gap |
| `--sp-4` | 16 | between functional groups inside a panel |
| `--sp-5` | 20 | panel inner padding (compact zones) |
| `--sp-6` | 24 | between functional groups on a page |
| `--sp-7` | 32 | between large page sections, page side padding (overview) |
| `--sp-8` | 40+ | hero/empty-state breathing room |

Rules:
- Page horizontal rhythm: overview screens `--sp-7` side padding; workspace
  keeps its denser `--sp-5` content columns with `--sp-7` only at the
  workspace-heading band.
- Between-groups rule of thumb (owner's guidance): related 8–12, groups
  16–24, sections 24–32 — encoded as `--sp-3` / `--sp-4..6` / `--sp-6..7`.
- Vertical rhythm inside lists is owned by DENSITY (§3), not by the scale.

## 2. Composition rules per zone (DEFAULT unless noted)

- **Sidebar**: 222px expanded (Lovable ADOPT); keep the brand block and
  project card; nav rows 39px; section labels get `--sp-4` top separation.
  No cards-in-cards: nav is a flat list with hairlines, not boxed groups.
- **Topbar / workspace-heading**: glass treatment — `backdrop-filter:
  blur(...)`, border-bottom hairline appears on scroll; heading band bottom
  padding `--sp-5` (was 18px hardcode).
- **Panels (workspace 3-column)**: inner padding `--sp-5`; panel headers
  are eyebrow+meta rows with `--sp-2` internal gap and `--sp-4` before
  content; panel min-widths stay (resizable contract §26).
- **Entry rows (editor)**: `--sp-3` grid gap (12px, unchanged — this IS the
  related-items distance), horizontal padding stays 13px→`--sp-3`+1px is
  FORBIDDEN: use 12px (`--sp-3`), the 1px drift is an UNJUSTIFIED_DRIFT fix.
- **Toolbar/filter rows**: one row, `--sp-2` between controls, `--sp-3`
  before a search field; overflow → "more" menu, never a second row.
- **Dialogs**: padding `--sp-6`, gap between action row and content
  `--sp-5`; actions right-aligned, destructive last.
- **Settings**: Wealthfolio pattern — content column max-width 720px,
  sections separated `--sp-6`, section titles eyebrow-style, groups inside
  `--sp-4`; no nested cards beyond one level.
- **Empty states**: icon-in-circle 80px, title + one muted line, dashed
  border, `--sp-8` top breathing; same in both themes.
- **Cards**: ONE radius scale (existing), `bg-card/70` translucency allowed
  only on glass surfaces (header, palette), not on data tables.

## 3. Information density (adaptive)

| Zone | Mode | Row height / metrics |
|---|---|---|
| Overview (Home, Checks summary) | Comfortable | card padding `--sp-6`, grid gap `--sp-4` |
| Settings | Structured | as §2 |
| Entries table (editor) | **Compact default** | existing `--density-row-pad-y` stays the ONLY density knob; target visible rows ≥ 8 at 720p |
| Entries table (optional) | Comfortable toggle | bumps `--density-row-pad-y` one step; persisted per project |

Compact is the DEFAULT for the editor (workstation profile); Comfortable is
a user choice, not a redesign. No information is removed to "gain air".

## 4. Current-state drift map (baseline audit, BEFORE)

Hardcoded values found in `r1-react.css` (evidence: file scan 2026-10-20):
`8px 14px` buttons, `13px` row padding, `28px` page side padding, `0 28px
18px` heading band, `padding 6px` icon buttons, mixed `12px/16px` gaps.
Migration: introduce `--sp-*` custom properties in `:root` (both themes —
space is theme-invariant), replace hardcodes zone by zone, each zone
verified by BEFORE→AFTER screenshots at 1280×720 and 1440×900, light+dark.

## 5. Verification contract (per skill product-ui-design)

- Same-viewport BEFORE/AFTER pairs; Lovable-original comparison uses the
  §21 parity method (contract-based, no pixel refs in repo — documented).
- Deterministic probes: root-scroll, horizontal-overflow, panel-overlap,
  scroll-ownership, long-text, theme-switch (qa-probes.md).
- Editor density probe: ≥8 visible entry rows at 1280×720 compact.
- WCAG AA unchanged (4.5:1 text) — spacing work must not touch contrast.
- Independent visual critique after implementation (fresh context).
