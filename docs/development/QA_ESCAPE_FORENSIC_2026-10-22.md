# Forensic audit: why the QA system missed owner-found defects — 2026-10-22

## Defect 1: `project_source_xml_preview` not registered (P0)

**User action**: Click the XML source tab → "Command project_source_xml_preview not found"

**Causal chain**:
- USER ACTION → click XML tab → frontend calls `project_source_xml_preview`
- DEFECT → command implemented in contract_adapter.rs:526 but never added to
  either `generate_handler!` or `LIVE_COMMANDS` in main.rs
- MISSED ASSERTION → no test asserted the command is registered; the existing
  `generate_handler!` lists are manually maintained and nothing cross-checks
  them against the adapter or the frontend
- TOOL/SKILL GAP → no command-registry parity test exists (ChatGPT's
  `check_tauri_handler_parity.py` was the first, still not in CI)
- CI GAP → CI tests Rust compilation (the adapter function compiles) and
  frontend build (the invoke call compiles) but never tests the runtime
  bridge between them
- CORRECTION → (a) command registered in both handlers + LIVE_COMMANDS
  (commit 0034297); (b) CI parity check needed: enumerate every frontend
  invoke name and assert it exists in LIVE_COMMANDS

## Defect 2: Entries pane not filling width (~300px gap) (P1)

**Causal chain**:
- USER ACTION → open workspace, observe huge empty strip between list and editor
- DEFECT → `.entries-pane` missing `flex: 1` inside `.ws-center` (the React
  bridge rule `.ws-panes [data-panel] > .entries-pane` didn't match because
  the DOM structure used `.ws-center` as the direct parent)
- MISSED ASSERTION → the visual audit suite checked contrast, WCAG, overflow,
  and theme rendering but NEVER checked layout geometry (element widths vs
  container widths)
- TOOL/SKILL GAP → no geometry/layout probe exists; the visual-audit spec
  measures contrast and visibility but not space utilization
- CI GAP → no layout regression test at standard window sizes
- CORRECTION → (a) CSS fix `.ws-center > .entries-pane { flex: 1; min-width: 0 }`
  (commit 0034297); (b) geometry probe needed: assert entries-pane width
  approximates ws-center width minus file-tree width

## Defect 3: AI mode labels showing English in Russian mode (P1)

**Causal chain**:
- USER ACTION → set UI to Russian → open AI Translation → mode buttons show
  "Translate automatically", "Bring your API key" etc. in English
- DEFECT → `AiTranslate.tsx:203` compared `t('uiLocale') === 'ru'`; but
  `uiLocale` is NOT a dictionary key — `t()` returns the key itself
  (`'uiLocale'`), never `'ru'`, so the English label always won
- MISSED ASSERTION → the i18n runtime audit walked every screen checking for
  raw i18n keys and English button text, but the AI Translate screen's mode
  buttons were rendered with hardcoded English/Russian pairs through a broken
  comparison — the audit's English-button regex checked for common verbs
  (Save/Cancel/etc.) but not domain phrases like "Translate automatically"
- TOOL/SKILL GAP → the runtime i18n audit's English detection pattern was
  too narrow; it didn't check ALL visible text against the RU dictionary
- CI GAP → no localization completeness check that verifies RU rendering
  of ALL interactive controls per screen
- CORRECTION → (a) `getLocale()` instead of `t('uiLocale')` (commit 4737669);
  (b) i18n audit should compare visible text against dictionary values

## Root cause summary

All three defects share the same systemic gap: **the QA system tested that
code compiles and screens render, but never tested the runtime connection
between frontend intent and backend capability**. Specifically:

1. No parity check: "what the frontend calls" vs "what Tauri registers"
2. No geometry check: "what the CSS promises" vs "what the layout delivers"
3. No locale-fidelity check: "what the dictionary says" vs "what renders"

## Systemic corrections needed

1. **Tauri command parity test** (Rust, CI-mandatory): extract every invoke
   name from the frontend transport map, assert each exists in LIVE_COMMANDS
2. **Layout geometry probe** (WDIO): at 1528px width, measure entries-pane
   width vs ws-center; assert no unexplained gap > 20px
3. **Locale fidelity check**: for each screen, verify that interactive
   control text matches RU dictionary values when locale=ru
