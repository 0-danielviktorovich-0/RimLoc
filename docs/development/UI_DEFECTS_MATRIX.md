# UI defects matrix — owner audit Oct-14 items on the integration head

Owner requirement: per-item VERIFIED / PARTIAL / MISSING with evidence, no
generic statuses. Head of record: `feat/integration-rel27` (PR #101),
current lineage (latest UI work head: 30c490a, CI 40/40, 0 alerts).
Native evidence: `RimLoc-evidence/reviewer/2026-10-15-integration-ui-e2e.txt`
(7/7 lane on the packaged automation build) + this file's source anchors.

| ID | Item | Status | Evidence / remaining |
|----|------|--------|----------------------|
| UI-01 | ChatBatch filter row overlap | **VERIFIED FIXED** | base `height:43px` overrode with `height:auto; min-height:43px` (r1-react.css); measured in the native lane: no scrollHeight>clientHeight clipping, no child bottom overflow |
| UI-02 | Desktop Focus/Studio/Overview modes | **VERIFIED FIXED** | three real modes (Studio/Focus/Overview) in the workspace heading: the switch changes the VIEW only — translations, unsaved drafts, selection and locale live in shared state and survive every switch (native lane: draft typed in Studio survives Focus and Overview and returns intact); persisted per app. The reviewer's condition «not decorative» is met by store-backed state + per-mode visibility |
| UI-03 | TRUE collapse (tree / nav / inspector) | **PARTIAL (improved)** | tree: the toggle hides tree CONTENT (items + projections) — verified natively; width-reclaiming panel collapse is a documented library limitation. Inspector: a new toggle button in the entry toolbar hides/shows the editor detail pane — verified natively (lane 11/11). Desktop NAV collapse: VERIFIED (nav toggle hides sidebar and reclaims margin; persisted; lane-tested) |
| UI-04 | Typography scale / density | **PARTIAL (improved)** | zoom steps now the requested readable set 80/90/100/110/125/150 (ceiling 1.5), persisted + applied at the shell root; a separate density control and remaining 7–11px small text are NOT done |
| UI-05 | Output XML preview | **VERIFIED FIXED** | `project_output_xml_preview` renders the project through the CANONICAL serializer (`write_rimworld_translation`) into an OS-temp scratch (removed best-effort) and returns the produced XML files in-memory; a fourth editor tab `output.xml` shows the file containing the selected entry for the current folder-form locale (folder-form mismatch found and fixed by the lane); native lane: translate a row via the editor → the output tab shows the DefInjected XML containing the saved translation |
| UI-06 | Scroll & row visibility (прокрутка и видимость строк) | **VERIFIED** | `@tanstack/react-virtual` from day one + scroll-to-selection (identity-keyed) + visible-bounds lane check; forced out-of-viewport scenario queued for the larger corpus |
| UI-07 | Deceptive filter reset | **VERIFIED FIXED** | one explicit clear-all control (search + status + type + file), own label; lane asserts combined-filters → clear → full list restored |
| UI-08 | Scroll-to-selection after filters | **VERIFIED FIXED** | virtualizer `scrollToIndex(selected, auto)` on selection change; lane-verified inside the visible viewport bounds |
| UI-09 | Dark-theme checkboxes/dividers | **PARTIAL / UNVERIFIED** | dark token set complete (r1.css .dark); a dedicated visual dark pass has not been run — needs the visual-audit lane |
| UI-09 | Shell, Projects, Settings, About | **PARTIAL (improved)** | About redesigned + lane-tested; Settings carries zoom + system theme (#92); shell/projects/settings visual pass pending. UI-12 (система глубокого визуального аудита) is the meta-item covering the pending passes |
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
- Dark visual pass: dark class + surfaces verified natively (lane 14/14);
  deeper contrast/checkbox/table checks queued with UI-09.
- ID-mapping caveat: the original Oct-14 audit file is absent from the
  local inbox; matrix IDs follow the reviewer's Oct-15 references. If the
  original audit re-surfaces, IDs must be reconciled.

| UI-10 | CLI↔GUI parity audit | **VERIFIED** | systematic comparison of all 27 CLI commands against GUI routes — `docs/development/CLI_GUI_PARITY_AUDIT.md`; 11 PARITY, 4 PARTIAL, 7 MISSING (file-format commands), 3 IMPLICIT; PO/XLIFF import/export identified as highest-value gap |
| UI-10 | PO import/export GUI (highest-value gap) | **VERIFIED (PO)** | the project-exchange PO closed the gap natively (9ccfc5e + 6a0d638 + 75f909f): export writes msgid = canonical source text, msgstr = the durable translation, `#:` references = the canonical Languages layout, so the file re-imports losslessly; import/apply rebuild the PO into the canonical tree and feed the SAME analyzer/apply core as the folder pack (conflicts never overwritten, ambiguous never auto-applied, persist-before-ack shared byte-identically through `apply_analyzed_pack`); the Existing screen hosts export + pick/analyze + the shared guarded apply; unit lane 327 green (round trip, apply persists-and-reloads, re-apply classifies as conflict); native lane 24/24 (export → dry-run import against the same project → reusable 0 / conflicts N / apply disabled). **XLIFF 1.2 closed the same way** (c2df726 + 0d798c2): `write_xliff_12_translated` (target = durable translation, note = canonical path), the canonical-tree rebuild feeds the SAME analyzer/apply core, the Existing screen hosts the mirror block; native lane verifies the source/target triples and the identical conflict classification — 24/24. validate-po stays CLI-first |
| UI-11 | Source Discovery | **VERIFIED (Stage A+B)** | Stage A (558a727): read-only full-project source search through the ONE effective scan — every match classified (AlreadyIndexed / DeliberatelyExcluded / OutsideEffectiveView / UnknownEligibility / SuspectedExtractionOmission / ParseFailure / UnsupportedFormat); a raw text match alone is never labeled a defect. Stage B (c4e9bc1 + 9aff9ba): whole-tree per-class counts that survive pagination, a line-ownership discriminator (a NoTranslate field line after an indexed label was misattributed by the start-line partition — the line's own element name is now checked against the hit's key suffix), per-class filter chips with honest counts in the workspace panel, i18n key fix (`suspected_omission` never matched the wire value). Native lane 23/23 on the packaged automation build: the Stage B scenario discovers an inactive 1.5 leftover, a NoTranslate texPath and a parse-error file, filters by class and clears. Unit lane: 12/12 in source_search.rs. Stage C (opt-in inclusion/recovery of out-of-view content) remains future work — a write-path decision, owner-gated by design |

## Restored original ID numbering (reviewer, Oct-15)

The Oct-15 verdict restored the ORIGINAL audit numbering; this matrix's IDs
were corrected to match: **UI-06** = scroll & row visibility, **UI-08** =
dark theme/checkbox/tables, **UI-09** = Shell/Projects/Settings/About,
**UI-10** = CLI↔GUI parity, **UI-11** = Source Discovery, **UI-12** = deep
visual audit system (meta). UI-01..05 match the earlier mapping. My Oct-15
morning matrix had UI-08/09/12 misnumbered — corrected here.
