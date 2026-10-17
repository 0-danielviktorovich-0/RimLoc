# Staged creator hub — design prototype (NOT published)

Owner addendum Oct-15 §C: ONE creator account per platform, contextual
project landings, a neutral static creator/portfolio home designed NOW and
published ONLY after an explicit owner decision. This document is the
design of record for the prototype; it ships in the RimLoc repo as a
staged design, NOT as a live site. No GitHub Pages repo is created, no
domain is registered, no links anywhere point at unpublished URLs.

## Information architecture

```
/                         creator identity & public portfolio
  /support/               support the creator's projects overall; the
                          VERIFIED payment options (registry below)
  /projects/rimloc/       RimLoc: product, status, real screenshots, repo/docs links
  /projects/rimloc/support/  focused RimLoc support context — same verified accounts
  /projects/<next>/       added only after the owner approves public disclosure
```

## Content sources (single authority)

- Support links: `docs/development/support-links.json` (the canonical
  registry; the About allowlist and this hub must reconcile against it).
- Copy: `RimLoc-evidence/support-assets/COPY.md` (EN primary, RU companion).
- Visuals: `RimLoc-evidence/support-assets/` (cover 1600×400, Ko-fi 600×300,
  OG 1200×630; real app icon, R1 oklch brand colors, no fake renders).

## Technology posture (staged, not built here)

- A self-contained static-site prototype OUTSIDE the RimLoc repository when
  implemented (it serves other projects; the RimLoc repo must not become the
  permanent authority for creator-wide content). Candidate: a tiny
  zero-JS static generator output committed to a private staging repo first.
- The live RimLoc MkDocs support page stays the stable public RimLoc support
  destination until the owner approves the hub; the hub then links FROM
  project surfaces only after it is actually published.

## Open owner decisions (recorded, not blocking other lanes)

1. Publish the hub at all (new public GitHub Pages repo/domain) — owner-only.
2. Creator brand name beyond "Daniel Viktorovich" — owner-only.
3. Crypto addresses on creator-wide surfaces — requires ownership +
   network/QR verification by the owner; RimLoc docs keep them for now.
4. BMAC membership copy migration (creator-neutral naming) — prices and
   existing member commitments unchanged; owner-only edit in the platform UI.
