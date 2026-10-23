# ROUTE × STATE × CONTROL COVERAGE MANIFEST — measured, 2026-10-23 (pass 2)

> **PASS 3 (control oracle, 2026-10-23, latest):** the coverage ledger
> (`control-ledger.json`, 51 route×state×control rows) is now executed by an
> oracle runner (`stage4-oracle.spec.ts`) that asserts a POSTCONDITION after
> every action — a click without its asserted effect is FAIL, never PASS.
> Result: **43/51 PASS (84.3%)**, 5 honest SKIP (native pickers, external
> links, download, mock-server-gated AI cancel/stale), **3 FAIL — all in the
> TM manual-add flow** (add does not surface the record in the list; lookup
> and filter therefore untestable). That TM cluster is a real open defect
> under investigation; it is NOT hidden by the manifest. The hard-coded
> ACTIVATED set and the silently-caught failures of the earlier stage-3
> runner are gone; the denominator is the stable ledger union.


**State-gated pass added:** the runner now selects an entry in the workspace
and activates the controls that only exist in that state (context tabs,
copy-key, target-locale select, project switcher, the enabled checks report,
diagnostics run, provider-form probes). Coverage: **63/94 interactive = 67.0%**
(the interactive set is state-dependent; the first pass measured 113 with the
same fixture — per-route numbers below come from that first full inventory).
Remaining 31: 7 external-action buttons + 5 native pickers + 2 import dialogs
+ 8 adjacent text inputs (po/xliff/tm-locale/status/lm-display) by design or
trivial; 1 AI-job control (`ws.editor-mt`) gated behind provider setup;
`ws.inspector-close` and the tree controls require a further state variant
(honest NOT_TESTED, not PASS).

Method: `stage3-coverage.spec.ts` runs on the packaged automation artifact
(merged main, state: a project is open over a 1-entry fixture). It inventories
every `data-testid` control per route (tag, interactive, visible, disabled),
then ACTIVATES controls click-by-click and re-measures. Machine evidence:
`route-control-coverage.json` (see run log in the private handoff).

## Headline numbers (state: project open)

| Metric | Value |
|---|---|
| Routes measured | 16 of 16 (sidebar 12 + footer about + palette-only providers/diagnostics/selfloc) |
| data-testid controls present | 182 |
| Interactive (visible, enabled) | 113 |
| Activated by journeys (stage 1 + 2) | 21 → **18.6%** |
| After the stage-3 activation pass | **62 → 54.9%** |
| Failed activations | 10 (3 state-gated tabs + 7 external-action buttons — see classification) |

Per-route interactive/activated: workspace 31/18 · projects 11/5 · checks 9/2 ·
existing 14/2 · compare 11/2 · glossary 12/4 · tm 18/5 · aitranslate 17/11 ·
chatbatch 12/5 · export 9/3 · lm 13/5 · settings 12/5 · about 15/2 ·
providers 17/9 · diagnostics 10/1 · selfloc 0/0 (palette entry landed on an
empty state — route needs a re-measure with its fixture).


> **PASS 4 (oracle final, same day): 46/51 = 90.2% verified, 0 FAIL, 5 honest
> skips.** The TM manual-add "defect" was a TEST bug: `tm.add-locale` is a
> free-text input, and the runner's select-setter silently no-op'd on it (the
> product's quiet no-op on an empty locale then masked the real cause). The
> runner now types the locale like a user; locale-agnostic palette matching
> keeps the lane stable in both UI locales. The remaining 5 skips stay
> classified as before.

## Remaining 51 uncovered — honest classification

**By design out of WDIO scope (19):** native OS pickers `be.pick, cmp.old-pick,
cmp.new-pick, diag.pick, ex.pick, ex.po-pick` + import-file inputs `gl.import,
tm.import-payload` (system file dialogs); external-action buttons that open the
browser/documentation `about.action.docs/github/issues/support,
about.support.bmc/sponsors` (href-less buttons by design — verified
present, not clicked to avoid external navigation).

**State-gated — appear only with a selected entry / specific state (10):**
`ws.tab.source, ws.tab.terms, ws.tab.xml, ws.copy-key, ws.editor-mt,
ws.editor-to-chatbatch, ws.entry.0, ws.inspector-close, cb.entry.*,
lm.edit-start` (the sibling `ws.tab.output` IS covered by stage-1 with a
selection; these need the same one-selection pre-step in a follow-up pass).

**Text-input extras on already-covered screens (10):** `ai.locale, diag.outdir,
ex.po-path, ex.po-out, ex.xliff-path, ex.xliff-out, tm.filter-locale,
tm.filter-status, tm.lookup-locale, tm.add-locale` — low-risk inputs adjacent
to covered controls.

**Next-pass candidates (12):** `ss.toggle, ws.target-locale, ws.tree-toggle,
ws.tree.keyed, ws.tree.view-file, ws.tree.view-type, sidebar.project-switcher,
settings.about-link, shell.about-link, prov.local, prov.connect.new,
prov.test.new, diag.run, checks.report, tm.add-status, lm.add-display` —
interactive now; each needs a deliberate activation with its postcondition.

Skip means NOT_TESTED — never PASS. The coverage runner ships in
`e2e/wdio-spike/stage3-coverage.spec.ts`; every future journey grows the
ACTIVATED set and the manifest is re-measured by one command.
