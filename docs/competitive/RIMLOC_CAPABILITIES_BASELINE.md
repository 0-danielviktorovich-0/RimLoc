# RIMLOC CAPABILITIES BASELINE (для competitive audit)

Дата: 2026-10-05. Источник: живой код на main (679f8b9).
Каждая возможность: evidence level по шкале §9 (0=NOT_IMPL...7=IN_GAME).

## SOURCE DISCOVERY
- Mod folder detection: LEVEL 5 (BUILT_APP_E2E) — scan_mod через CLI+GUI
- Core/DLC detection: LEVEL 5 — LoadFolders resolution
- Workshop mods: LEVEL 4 (INTEGRATION_TESTED) — через папку Mods
- Dependencies: LEVEL 2 (SOURCE_CONFIRMED)
- LoadFolders: LEVEL 5 — парсер LoadFolders.xml
- IfModActive: LEVEL 3 (UNIT_TESTED)
- Version selection: LEVEL 5

## EXTRACTION
- DefInjected: LEVEL 5 — основной путь
- Keyed: LEVEL 5
- Strings/RulePack: LEVEL 4
- Backstories: LEVEL 3
- Patches: LEVEL 3
- C# TKey: LEVEL 4
- XML comments: LEVEL 4
- Inheritance/Abstract: LEVEL 2
- Duplicate keys: LEVEL 4
- Source provenance: LEVEL 5 (source_ref в снапшоте)

## EXISTING TRANSLATIONS
- Import translation: LEVEL 5 — import_existing
- Update existing: LEVEL 5 — apply_existing
- New/sourceChanged/obsolete: LEVEL 5
- Ambiguous: LEVEL 5
- Manual preservation: LEVEL 5

## TRANSLATOR WORKSPACE
- Manual editor: LEVEL 5 — React workspace
- Source/target side-by-side: LEVEL 5
- Filters/search: LEVEL 5
- Batch actions: LEVEL 4
- Review state: LEVEL 5
- Source Inspector: LEVEL 5 (source_ref)
- History/revisions: LEVEL 4
- Undo/redo: LEVEL 3
- Keyboard shortcuts: LEVEL 4
- Command palette: LEVEL 5 (Cmd+K, ea45339)
- Multi-target: LEVEL 5 (registry 9 языков)

## TM / GLOSSARY
- Glossary CRUD: LEVEL 5 — project_glossary* (b391ba4)
- TM automatic accumulation: 0 — NOT_IMPLEMENTED (ждёт брифа A/B/C)
- TM import: 0
- TM fuzzy matching: 0
- TM quality states: 0
- TM target-locale isolation: 0 (дизайн готов)

## MACHINE TRANSLATION
- Provider templates (5 шт): LEVEL 2 — UI существует, LLM-вызовы не подключены
- Cloud MT: 0
- LLM/API: 0
- Local/offline MT: 0
- No-API chat workflow: PARTIAL — chatbatch store в Svelte, не портирован

## LANGUAGE-SPECIFIC
- Russian morphology: 0
- WordInfo: LEVEL 2 (концепт, не реализовано)
- Arabic/RTL: LEVEL 1 (locale metadata, не shaping)

## QA / VALIDATION
- Empty values: LEVEL 5 — project_validate
- Lost placeholders: LEVEL 5 — gate_c_tests
- XML validity: LEVEL 5
- Duplicates: LEVEL 5
- Source drift: LEVEL 5 — source_fingerprint
- Automatic repair: 0
- Agentic repair: 0

## FORMATS
- Import PO: LEVEL 5
- Import XLIFF: LEVEL 4
- Export PO/POT/CSV/JSON/XLIFF/XML: LEVEL 5

## BUILD / DELIVERY
- Build translation mod: LEVEL 5 — project_build_mod
- Safe output: LEVEL 5 — guard_output_denied
- Dry run: LEVEL 4
- In-game runtime test: LEVEL 7 — Runtime Bridge v1.0.0 (T6 PASS)

## PROJECT MODEL
- Persistence: LEVEL 5 — project_store v2
- Recent projects: LEVEL 5
- Revisions: LEVEL 5 — durable monotonic revision
- Resume after restart: LEVEL 5
- Autosave: LEVEL 4 (dirty + acked_revision)
- Crash recovery: LEVEL 4

## DIAGNOSTICS / SECURITY
- Support bundle: LEVEL 5 — project_diagnose
- Redaction: LEVEL 5
- Safe paths: LEVEL 5 — guard_output_denied
- Read-only source: LEVEL 5
- Symlink protection: LEVEL 4
- Automation exclusion from prod: LEVEL 5 — release-guard

## ADAPTER / MULTI-GAME
- LocalizationAdapter model: LEVEL 3 (design + RimLocApplication adapter)
- RimWorld adapter: LEVEL 5
- Other game adapters: 0 (architectural только)
