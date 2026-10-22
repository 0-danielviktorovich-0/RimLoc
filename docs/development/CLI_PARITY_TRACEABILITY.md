# CLI → capability → IPC → GUI traceability (Issue #5)

Owner-visible capability map of the beta candidate. Sources: the CLI
parser (`rimloc-cli`), the live IPC registry
(`gui/tauri-app/src-tauri/src/contract_adapter.rs` COMMAND lists), the
React routes (`lib/routes.ts`), the native lanes. Status legend:
**PARITY** (GUI covers the user story) · **GUI-FIRST** (the GUI is the
product; the CLI subcommand is automation sugar) · **CLI-ONLY** (see
the decisions below) · **IMPLICIT** (covered by a wider flow, no
1:1 surface).

## Decisions on the three CLI-only commands (P2 closure)

- **validate-po** → CLI-ONLY BY DESIGN for the beta. The GUI validates
  every PO it imports itself (same `validate_export_payload` /
  strict-parse gates); a standalone "validate an arbitrary external
  PO" surface has no user story in the beta scope (import already
  refuses bad files with typed errors). Deferred with the Advanced
  Tools decision below.
- **morph** → CLI-ONLY (wordform inflection helper for translators'
  glossaries). Power-tool; the GUI glossary covers authoring. Deferred.
- **schema** → CLI-ONLY (XML schema drift report between game
  versions). Power-tool for modders; version-diff surface exists in
  the GUI (`contract_version_diff`, Compare screen). Deferred.
- **Advanced Tools UI**: intentionally NOT built for the beta — the
  three deferrals above are its entire candidate set, none is
  beta-blocking, and a progressive-disclosure surface solely for them
  adds review area without a user ask. Revisit post-beta if users
  request it (Issue #5 allows this: "implement bounded missing
  capabilities ... as safe typed operations").

## Traceability table (20 CLI command modules)

| CLI command | User capability | Typed IPC (live) | GUI surface (route) | Status / native evidence |
|---|---|---|---|---|
| scan | Inspect a mod's translatable inventory | project_create / project_snapshot | wizard + workspace (#/workspace) | PARITY — wizard lane, 12k corpus lane |
| init | Create a translation project | project_create | wizard | GUI-FIRST — wizard lane |
| translate (mock) | Translate via mock provider | ai_job_start/status/cancel/accept(+selected) | AI translate (#/aitranslate) | PARITY — 19 ai_job unit regressions + lane |
| provider_test | Probe a provider connection | contract_provider_instance_test | Providers screen | PARITY — typed outcomes, mock+ollama lanes |
| validate | Validate project consistency | project_validate | Checks screen (#/checks) | PARITY — checks route in visual audit |
| compare / version_diff | Diff two mod versions | contract_version_diff | Compare screen (#/compare) | PARITY — route in visual audit |
| export_po | Export project exchange PO | project_export_po | Existing screen (#/existing) | PARITY — native PO lane 24/24 round trip |
| import_po | Import/apply a PO pack | project_import_po / project_apply_po | Existing screen | PARITY — same lane |
| export_xliff | Export XLIFF 1.2 | project_export_xliff | Existing screen | PARITY — native lane |
| import_xliff | Import XLIFF | project_import_xliff / project_apply_xliff | Existing screen | PARITY — native lane |
| build_mod | Build the translation mod | project_build_mod + project_export | Export screen (#/export) | PARITY — guarded export, reparse check |
| lang_update | Pull upstream language zip | (services only) | n/a | CLI-ONLY (upstream corpus maintenance) |
| learn_defs / learn_patches | Feed the allowlist audit | (dev tooling) | n/a | CLI-ONLY (dev pipeline; produced the 171-rule pack) |
| annotate | Emit translation sidecars | (services only) | n/a | IMPLICIT — the editor's per-row review is the product surface |
| diff_xml | Structured XML diff | project_output_xml_preview | editor tabs | IMPLICIT — preview parity (UI-05 lane) |
| doctor | Environment diagnostics | project_diagnose + build_identity | Diagnostics (#/diagnostics) | PARITY — route in visual audit |
| wordinfo | Word usage stats | (services only) | n/a | CLI-ONLY (translator power tool) |
| xml_health | Corpus XML health | project_validate + source_search ParseFailure class | Checks + Source Discovery panel | IMPLICIT — parse-error classification is surfaced in the search panel |
| morph | Glossary inflection | none | n/a | CLI-ONLY — decision above |
| schema | Version schema drift | contract_version_diff | Compare screen | IMPLICIT for users, CLI for modders — decision above |

Non-CLI product flows (Issue #5's second table, summarized):
chat batch (#/chatbatch), provider instances (settings), TM
(#/tm), glossary (#/glossary), existing-pack import (#/existing),
compare, selfloc (#/selfloc), project switching (sidebar) — all live
and covered by the native lanes / visual audit; per-row AI review and
the full-scope acknowledgement are the 2026-10-19 additions.
