# UI defects matrix — owner audit Oct-14 items on the integration head

Owner requirement: per-item VERIFIED / PARTIAL / MISSING with evidence, no
generic statuses. Head of record: `feat/integration-rel27` (PR #101),
current `8ea7b96` (UI-05 output XML preview landed; security barrier verified — 0 open code-scanning alerts).
Native evidence: `RimLoc-evidence/reviewer/2026-10-15-integration-ui-e2e.txt`
(7/7 lane on the packaged automation build) + this file's source anchors.

| ID | Item | Status | Evidence / remaining |
|----|------|--------|----------------------|
| UI-01 | ChatBatch filter row overlap | **VERIFIED FIXED** | base `height:43px` overrode with `height:auto; min-height:43px` (r1-react.css); measured in the native lane: no scrollHeight>clientHeight clipping, no child bottom overflow |
| UI-02 | Desktop Focus/Studio/Overview modes | **VERIFIED FIXED** | three real modes (Studio/Focus/Overview) in the workspace heading: the switch changes the VIEW only — translations, unsaved drafts, selection and locale live in shared state and survive every switch (native lane: draft typed in Studio survives Focus and Overview and returns intact); persisted per app. The reviewer's condition «not decorative» is met by store-backed state + per-mode visibility |
| UI-03 | TRUE collapse (tree / nav / inspector) | **PARTIAL (improved)** | tree: the resizable panel itself now collapses/expands via `panelRef` (width reclaims; lane-tested next run). Desktop NAV collapse: still MISSING (mobile-only menu). Right-inspector collapse: MISSING |
| UI-04 | Typography scale / density | **PARTIAL (improved)** | zoom steps now the requested readable set 80/90/100/110/125/150 (ceiling 1.5), persisted + applied at the shell root; a separate density control and remaining 7–11px small text are NOT done |
| UI-05 | Output XML preview | **VERIFIED FIXED** | `project_output_xml_preview` renders the project through the CANONICAL serializer (`write_rimworld_translation`) into an OS-temp scratch (removed best-effort) and returns the produced XML files in-memory; a fourth editor tab `output.xml` shows the file containing the selected entry for the current folder-form locale (folder-form mismatch found and fixed by the lane); native lane: translate a row via the editor → the output tab shows the DefInjected XML containing the saved translation |
| UI-06 | Virtual scrolling | **VERIFIED (pre-existing)** | `@tanstack/react-virtual` entry list from day one (Workspace.tsx useVirtualizer) |
| UI-07 | Deceptive filter reset | **VERIFIED FIXED** | one explicit clear-all control (search + status + type + file), own label; lane asserts combined-filters → clear → full list restored |
| UI-08 | Scroll-to-selection after filters | **VERIFIED FIXED** | virtualizer `scrollToIndex(selected, auto)` on selection change; lane-verified inside the visible viewport bounds |
| UI-09 | Dark-theme checkboxes/dividers | **PARTIAL / UNVERIFIED** | dark token set complete (r1.css .dark); a dedicated visual dark pass has not been run — needs the visual-audit lane |
| UI-12 | Settings/About polish | **PARTIAL (improved)** | About redesigned + lane-tested (hero/actions/support/folded identity); Settings carries zoom + system theme (#92); full settings visual pass pending |
| UI-13 | Fake file tree | **VERIFIED FIXED (improved)** | dual honest projections plus two-level directory grouping (real dir prefix → file, root files under `·`); per-file filtering lane-tested; a full hierarchical catalog view beyond two levels remains future work |
| UI-14 | Invisible selected editor row | **VERIFIED FIXED** | stored selection wins only while visible, otherwise first visible row; stored key restored on clear; lane-tested |

Other Oct-12 owner defects: single global new-project wizard, split nav and
system theme are present at source (#92-verified by the owner); native
runtime verification for those is queued with the UI-02/03 work.

## Review carry-overs (verdict 2026-10-15, worked 3m23s)

- UI-01 measurements extended to 100% AND 125% app zoom via the live zoom
  event (no reload); RU corpus; window-size and dark-theme sweeps queued.
- UI-07 scenario dirties all four dimensions before the single clear —
  lane-passing.
- UI-14 scenario now hides the selection while OTHER rows stay visible —
  caught and codified the exact banned behavior.
- UI-13 hierarchical directory grouping implemented; deeper catalog
  hierarchy evaluation queued.

## Review carry-overs (verdict Tranche 1, CONDITIONAL_PASS_UI_TRANCHE_1)

- UI-07: status filter now has a stable testid (`ws.filter.issues`) and the
  scenario PROVES all four dimensions are dirty (active classes + search
  value asserted) before the clear.
- UI-14: the scenario explicitly selects Beta, filters to Alpha-only, and
  asserts the three required states (Beta shown → Alpha visible + editor
  follows → clear restores Beta).
- Scroll effect keys on the selected IDENTITY (not just the index); the
  forced out-of-viewport scenario needs the larger corpus fixture — queued.
- ID-mapping caveat: the original Oct-14 audit file is absent from the
  local inbox; matrix IDs follow the reviewer's Oct-15 references. If the
  original audit re-surfaces, IDs must be reconciled.
