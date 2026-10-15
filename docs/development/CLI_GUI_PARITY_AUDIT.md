# CLI↔GUI Parity Audit (UI-10) — integration head 14acc73, 2026-10-15

Systematic comparison of every CLI command (rimloc-cli 27 commands) against
the React GUI routes. The shared Rust services layer (`rimloc-services`)
owns the real behavior for both — parity means the GUI exposes the same
operation through its IPC contract.

## Parity matrix

| CLI command | GUI route / surface | Parity | Notes |
|---|---|---|---|
| `scan` | Wizard open → project create | **PARITY** | GUI creates a project with the same scanner pipeline |
| `merge-keyed` | Existing import | **PARTIAL** | GUI imports existing packs; CLI merge-keyed has EN-comment and UNUSED-section options not exposed |
| `validate` | Checks page | **PARITY** | Same validator engine, same findings |
| `validate-po` | — | **MISSING** | PO-placeholder validation is CLI-only |
| `export-po` | — | **MISSING** | PO export is CLI-only |
| `export-xliff` | — | **MISSING** | XLIFF export is CLI-only |
| `import-po` | — | **MISSING** | PO import is CLI-only |
| `import-xliff` | — | **MISSING** | XLIFF import is CLI-only |
| `init` | Wizard create | **PARITY** | Same project scaffolding |
| `lang-update` | Language Manager | **PARITY** | Same language update logic |
| `learn-defs` | — | **IMPLICIT** | The scanner auto-learns Defs fields; no explicit GUI control |
| `learn-keyed` | — | **IMPLICIT** | Auto-learned; same pipeline |
| `learn-patches` | — | **IMPLICIT** | Patch learning is automatic |
| `morph` | — | **MISSING** | Morph command is CLI-only |
| `provider-test` | Providers screen → Test | **PARITY** | Same provider test via IPC |
| `build-mod` | Build/Export page | **PARITY** | Same writer + reparse check |
| `compare` | Compare page | **PARITY** | Same version diff |
| `coverage` | Workspace counters | **PARTIAL** | GUI shows coverage counters; CLI `coverage` has detailed per-file breakdown not exposed |
| `diff-xml` | Compare page | **PARITY** | Same diff logic |
| `doctor` | Diagnostics page | **PARITY** | Same diagnostic checks |
| `schema` | — | **MISSING** | Schema generation is CLI-only |
| `translate` | AI Translate screen | **PARITY** | Same engine, provider factory, cancellation |
| `version-diff` | Compare page | **PARITY** | Same diff |
| `word-info` | Glossary | **PARTIAL** | Glossary shows terms; CLI word-info has morphology details not exposed |

## Summary

| Parity level | Count | Commands |
|---|---|---|
| **PARITY** (GUI equivalent exists and works) | 11 | scan, validate, init, lang-update, provider-test, build-mod, compare, diff-xml, doctor, translate, version-diff |
| **PARTIAL** (GUI has related but not full functionality) | 4 | merge-keyed, coverage, word-info, xml-health |
| **MISSING** (CLI-only, no GUI surface) | 7 | validate-po, export-po, export-xliff, import-po, import-xliff, morph, schema |
| **IMPLICIT** (auto-learned, no explicit GUI control needed) | 3 | learn-defs, learn-keyed, learn-patches |
| **N/A** (help) | 1 | help |

## Priority for closing gaps

The MISSING items are all **file-format conversion** commands (PO/XLIFF
import/export, schema, morph). These are power-user workflows that are
low-priority for the GUI-first product (ordinary players don't need PO/XLIFF
round-tripping). The PARTIAL items are enhancements to existing GUI surfaces.

**Recommended next steps:**
1. Add PO import/export to the Existing import page (medium effort, high
   value for mod authors using CAT tools)
2. Add XLIFF import/export alongside PO (medium effort, same audience)
3. Schema generation — low priority (developer tool)
4. Morph — low priority (niche feature)
