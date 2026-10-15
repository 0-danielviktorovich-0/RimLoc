# UI defects matrix — owner audit Oct-14 items on the integration head

Owner requirement: per-item VERIFIED / PARTIAL / MISSING with evidence, no
generic statuses. Head of record: `feat/integration-rel27` (PR #101).
Native evidence: `RimLoc-evidence/reviewer/2026-10-15-integration-ui-e2e.txt`
(7/7 lane on the packaged automation build) + this file's source anchors.

| ID | Item | Status | Evidence / remaining |
|----|------|--------|----------------------|
| UI-01 | ChatBatch filter row overlap | **VERIFIED FIXED** | base `height:43px` overrode with `height:auto; min-height:43px` (r1-react.css); measured in the native lane: no scrollHeight>clientHeight clipping, no child bottom overflow |
| UI-02 | Desktop Focus/Studio/Overview modes | **MISSING** | `.layout-console`/`.layout-focus` CSS classes exist in r1.css but the React Workspace renders one fixed layout; no mode switch UI. Next C-lane item |
| UI-03 | TRUE collapse (tree / nav / inspector) | **PARTIAL (improved)** | tree: the resizable panel itself now collapses/expands via `panelRef` (width reclaims; lane-tested next run). Desktop NAV collapse: still MISSING (mobile-only menu). Right-inspector collapse: MISSING |
| UI-04 | Typography scale / density | **PARTIAL** | persisted UI zoom 0.8–1.4 applied at the shell root (Settings + `rimloc.ui.zoom`, #92); the requested 90/100/110/125/150 steps and a density control are NOT implemented; some 7–11px text remains by design |
| UI-05 | Output XML preview | **PARTIAL** | source XML read-only tab VERIFIED (#92, IPC-tested); a distinct OUTPUT XML preview via the canonical serializer is MISSING (export feature exists, preview does not) |
| UI-06 | Virtual scrolling | **VERIFIED (pre-existing)** | `@tanstack/react-virtual` entry list from day one (Workspace.tsx useVirtualizer) |
| UI-07 | Deceptive filter reset | **VERIFIED FIXED** | one explicit clear-all control (search + status + type + file), own label; lane asserts combined-filters → clear → full list restored |
| UI-08 | Scroll-to-selection after filters | **VERIFIED FIXED** | virtualizer `scrollToIndex(selected, auto)` on selection change; lane check added |
| UI-09 | Dark-theme checkboxes/dividers | **PARTIAL / UNVERIFIED** | dark token set complete (r1.css .dark); a dedicated visual dark pass has not been run — needs the visual-audit lane |
| UI-12 | Settings/About polish | **PARTIAL (improved)** | About redesigned + lane-tested (hero/actions/support/folded identity); Settings carries zoom + system theme (#92); full settings visual pass pending |
| UI-13 | Fake file tree | **VERIFIED FIXED** | dual honest projections: by entry type AND by real `source_ref.file` with an explicit no-source-file bucket; per-file filtering lane-tested; no invented paths |
| UI-14 | Invisible selected editor row | **VERIFIED FIXED** | stored selection wins only while visible, otherwise first visible row; stored key restored on clear; lane-tested |

Other Oct-12 owner defects: single global new-project wizard, split nav and
system theme are present at source (#92-verified by the owner); native
runtime verification for those is queued with the UI-02/03 work.
