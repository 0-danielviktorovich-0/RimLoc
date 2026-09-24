===============================================================================
(G4/G5 — кумулятивный мандат владельца, 2026-09-24; единый фронтенд-DAG, одна IA)
MULTI-TARGET LANGUAGE WORKSPACE / EDITOR UX
===============================================================================

This mandate extends the current GUI QA, Language Registry and hybrid
translation workflow requirements.

The owner's additional hands-on feedback:

Switching target language should be fast and obvious DIRECTLY inside the
translation workspace.

RimLoc should be architected for:

one canonical source inventory
+
one or more target-language translations

rather than forcing independent duplicated projects for every target language.

Do not compromise beginner simplicity.

===============================================================================
1. PROJECT / TARGET MODEL
===============================================================================

Conceptually separate:

PROJECT
- source content
- source locale
- RimWorld/version context
- canonical source inventory

TARGET TRANSLATIONS
- one or more target locales
- independent translation state/progress
- validation state
- provenance
- review state.

Example:

Project:
Vanilla Furniture Expanded
Source: English

Targets:
Russian
Ukrainian
Japanese
German.

The v1 beginner flow may initially create one target.

The canonical architecture should not require project duplication to add another
target later.

===============================================================================
2. ACTIVE TARGET
===============================================================================

Introduce an explicit concept:

active_target_locale

Workspace displays/edits the active target translation.

Changing active target must NOT:

- rescan the mod;
- rebuild source inventory;
- create a separate project;
- lose editor position/state unnecessarily.

It should switch the translation dataset over the same canonical source entries.

===============================================================================
3. WORKSPACE LANGUAGE SWITCHER
===============================================================================

Add a clear target-language selector in the Workspace header.

Conceptual UI:

English → [ Русский ▾ ] [ + ]

Clicking target opens a searchable language switcher.

Example:

Русский           87%   12 issues
Українська        44%    8 issues
日本語             13%   31 issues
Deutsch           92%    3 issues

+ Add target language
+ Manage languages
+ Compare languages

Do not confuse this control with application UI-language selection.

===============================================================================
4. PINNED TARGET TABS
===============================================================================

Evaluate optional pinned language tabs for translators maintaining a small
number of target languages simultaneously.

Example:

SOURCE English

[ Русский 87% ] [ Українська 44% ] [ 日本語 13% ] [+]

Avoid showing an unbounded row of 10+ language tabs.

Allow users to pin a small working set while other target locales remain in the
dropdown/manager.

Use the simplest design that tests well.

===============================================================================
5. ADD LANGUAGE FROM WORKSPACE
===============================================================================

The `+` action beside target locale should allow adding another target language
without leaving the editor.

Flow:

+ Add language
→ search known languages
→ select
or
→ Create custom language.

Then initialize translation state safely.

Do not hide this capability only in deep Settings.

===============================================================================
6. LANGUAGE MANAGER
===============================================================================

Provide a proper project-level Languages management surface.

Show:

SOURCE
English

TARGETS

Russian
progress
issues
last modified

Ukrainian
progress
issues

Japanese
...

Actions may include:

- Open / make active
- Pin to editor
- Compare
- Import translation
- Export translation
- Language-specific settings
- Detach/remove safely

+ Add target language.

===============================================================================
7. CUSTOM LANGUAGE
===============================================================================

Language Registry must support user-defined locales.

Simple dialog:

Display name
Native name
Locale identifier

Advanced fields only when needed:

- RimWorld folder/name
- script
- writing direction
- capability overrides
- normalization behavior.

Assign Generic LanguageCapabilities by default.

Do not prevent translation merely because RimLoc lacks special morphology
support.

===============================================================================
8. LOCALE, NOT ONLY LANGUAGE
===============================================================================

The underlying registry should support locale-level distinctions, not only
coarse language names.

Examples:

pt-BR
pt-PT
zh-Hans
zh-Hant
es-ES
es-419

Do not force all variants of a language into one target identity.

Use human-friendly names in the GUI.

===============================================================================
9. RTL READINESS
===============================================================================

Where inexpensive architecturally, avoid assumptions that all target languages
are LTR.

User-defined/future locales may be RTL.

Do not require full RTL product localization immediately, but keep editor
language metadata capable of expressing text direction.

Test a representative RTL text rendering case later.

===============================================================================
10. TARGET-SPECIFIC STATE
===============================================================================

Switching target language must switch all appropriate target-specific state:

- translated text
- statuses
- validation
- review state
- glossary
- TM
- AI provenance
- notes if target-specific
- sourceChanged review state
- progress.

Source context remains shared.

===============================================================================
11. LANGUAGE-PAIR TM / GLOSSARY
===============================================================================

Workspace should make the active language pair clear.

TM and glossary lookup must use:

source locale
+
active target locale.

Example:

EN→RU glossary

must not silently appear as:

EN→JA.

Professional UI may visibly label resource scope.

===============================================================================
12. LANGUAGE-SPECIFIC AI CONFIGURATION
===============================================================================

Allow project default AI settings.

Also architect optional target-language overrides.

Example:

Project default:
GLM

Japanese:
Claude override

Russian:
project default

Do not expose this complexity in beginner flow unless needed.

Language settings may offer:

Use project default
or
Override.

===============================================================================
13. TARGET SWITCH DURING AI JOBS
===============================================================================

If an AI translation job is running for Russian, the user should be allowed to
switch to Japanese or another target where technically safe.

Clearly show background activity:

Russian
AI translating 1,842 / 5,200

Switching UI target must not cancel or corrupt the existing job.

Background jobs must identify:

project
target locale
operation ID.

===============================================================================
14. MULTIPLE TARGET JOBS
===============================================================================

Architect jobs so independent target-language operations can eventually run in
parallel where provider/resource limits allow.

Example:

Russian translation job
Japanese validation job

without state collision.

Do not implement uncontrolled concurrency merely because architecture allows it.

Use job/resource limits.

===============================================================================
15. STALE AI WRITE PROTECTION PER TARGET
===============================================================================

The previously required revision protection must be target-specific.

AI result for:

SourceEntry X
Target RU
Revision 17

must never overwrite:

Target RU
Revision 18 human edit.

It must also never accidentally write into Target JA.

Add regression tests.

===============================================================================
16. SOURCE LANGUAGE CHANGES
===============================================================================

Changing SOURCE locale is much more consequential than changing target.

Do not present it as an equally casual switch after project initialization.

If future workflows allow changing source language:

- explain impact;
- rebuild/reconcile source inventory;
- preserve target work only when safe.

Target switching should be cheap.
Source switching should be deliberate.

===============================================================================
17. LANGUAGE COMPARISON MODE
===============================================================================

Evaluate an advanced multi-target comparison view.

Example:

SOURCE
English

TARGET A
Russian

TARGET B
Ukrainian

Use cases:

- terminology consistency
- closely related languages
- translation-team review
- reference comparison.

Do not make this the default editor.

Potential layouts:

Source | RU | UK

or

Source
RU
UK

depending on width.

Support 2 target comparisons initially rather than arbitrary 10-column tables.

===============================================================================
18. REFERENCE LANGUAGE
===============================================================================

Allow an existing other-language translation to be used as a REFERENCE without
changing canonical source identity.

Example:

English source
Russian target
existing German translation as secondary reference.

This can help human translators.

Do NOT automatically translate RU from German while pretending English was the
semantic source.

Mark reference language clearly.

===============================================================================
19. CREATE TARGET FROM EXISTING TRANSLATION
===============================================================================

When adding a target locale offer initialization options conceptually:

- Empty
- Existing language pack
- Translation Memory
- Import file
- optional reference from another target
- AI later.

Do not force AI.

===============================================================================
20. LANGUAGE PROGRESS
===============================================================================

Each target should expose independent progress such as:

translated
needs review
source changed
validation errors
obsolete.

Use concise summaries in switcher/Language Manager.

Avoid information overload.

===============================================================================
21. TARGET REMOVAL SAFETY
===============================================================================

Removing a target translation can destroy significant work.

Default behavior should avoid permanent loss.

Evaluate:

Detach from active project
Export first
Archive
Permanent delete

Destructive delete should:

- be explicit
- explain amount of affected work
- require confirmation.

Do not delete source inventory.

===============================================================================
22. KEYBOARD / COMMAND PALETTE
===============================================================================

Provide fast language switching for professional users.

Command Palette actions:

Switch target → Russian
Switch target → Japanese
Add target language
Manage languages
Compare languages.

Evaluate a remappable keyboard shortcut for opening the language switcher.

Do not hardcode a shortcut that conflicts with common OS/editor conventions.

===============================================================================
23. PROJECT HEADER
===============================================================================

Workspace header should make source/active-target relationship obvious without
clutter.

Example:

TestMod
English → Russian
RimWorld 1.6

The target portion may be interactive.

Do not duplicate the full language manager in the header.

===============================================================================
24. BEGINNER FLOW
===============================================================================

Quick Translate remains simple.

Normal player:

choose content
→ choose ONE target
→ translate.

Do not force multi-target concepts into beginner onboarding.

After project creation advanced users can add targets.

===============================================================================
25. WHOLE-GAME / DLC TRANSLATOR USE CASE
===============================================================================

Multi-target architecture is especially valuable for base-game/DLC localization
teams.

One source inventory can serve multiple maintained language packs.

Do not make this capability mod-only.

===============================================================================
26. PSEUDO-LOCALE READINESS
===============================================================================

Evaluate supporting a development pseudo-locale later.

Purpose:

- detect hardcoded UI strings
- detect clipping
- stress long text
- verify Unicode/layout.

Do not prioritize above real target languages.

Keep Language Registry flexible enough that pseudo locales are possible.

===============================================================================
27. UI LANGUAGE REMAINS SEPARATE
===============================================================================

Repeat as an invariant:

Application UI locale
≠
Project source locale
≠
Active target locale.

Switching RU/EN interface language must NOT alter project translation language.

Add E2E regression.

===============================================================================
28. PERSISTENCE
===============================================================================

Persist:

- project target list
- active target
- pinned languages
- per-target settings
- per-target progress/state.

Reopening the project must restore the user's language workspace.

===============================================================================
29. GUI TESTS
===============================================================================

Add tests for:

- switch RU → JA target
- translation data changes appropriately
- source remains identical
- TM/glossary scope changes
- add custom language
- custom language persists
- UI locale switch does not affect target
- stale RU AI result cannot modify JA
- remove/detach target is safe
- reopen restores active target
- keyboard/command palette switch.

===============================================================================
30. BACKEND CONTRACT
===============================================================================

Do NOT invent a frontend-only target-language store that diverges from the
canonical project model.

During mock phase use a clean adapter matching the intended backend semantics.

After backend freeze bind it to the canonical Project/Translation model.

===============================================================================
31. VISUAL DESIGN
===============================================================================

Language switching must remain visually lightweight.

Do not introduce a giant toolbar solely for target management.

Target switcher should feel comparable to changing:

branch
workspace
document locale

in a professional tool.

Use subtle progress/issue indicators.

===============================================================================
32. DELIVERABLE
===============================================================================

Extend:

GUI_DESIGN_SPEC.md
GUI_UX_ACCEPTANCE.md

and implement the interactive mock for:

- Workspace target switcher
- Add language
- Language Manager
- target progress
- persistence mock
- command-palette language actions.

Continue in parallel with other GUI QA work.

No push.
No release.
No stash/branch deletion.