<!-- mandate_id: g4-sprint | wave: G4 | scope: GUI implementation sprint: home, wizard, workspace, settings, palette -->
GUI IMPLEMENTATION SPRINT (G4) — мандат владельца, 2026-09-24
======================================================================
===============================================================================
GUI IMPLEMENTATION SPRINT — TURN THE SPEC INTO A REAL PRODUCT MOCK
===============================================================================

The current GUI Style Lab is still mostly a visual skeleton.

Do NOT ask the owner to choose the final visual style yet.

The current Style Lab does not contain enough representative product surfaces
to make a meaningful style decision.

Continue backend J/K/L independently.

In parallel, launch a dedicated GUI implementation sprint against mocks only.

NO backend contract binding yet.

===============================================================================
1. IMPORTANT: IMPLEMENT, DO NOT ONLY WRITE MORE SPEC
===============================================================================

Previous GUI work produced useful:

- GUI_DESIGN_SPEC
- Style Lab
- design tokens
- i18n foundation
- mock editor

Now convert the approved information architecture into actual interactive
frontend screens.

Do not spend this sprint mainly expanding design documents.

The owner must be able to CLICK THROUGH a realistic RimLoc product.

===============================================================================
2. USE PARALLEL GUI SUBAGENTS
===============================================================================

Use multiple isolated frontend/design workers where safe.

Suggested ownership:

GUI-A — Home + Quick Translate + Existing Translation flow

GUI-B — Workspace + Review + Build/result

GUI-C — Settings + AI Provider Manager + Help/Diagnostics + Command Palette

GUI-D — visual polish / motion / accessibility / responsive review

Lead frontend integrator:
- owns shared design tokens/components
- integrates
- prevents duplicate component systems
- performs final visual review

All frontend workers remain inside the mock/frontend layer.

NO Rust/backend changes.

===============================================================================
3. HOME — MAKE IT FEEL LIKE A REAL APPLICATION
===============================================================================

Implement a polished Home screen.

First-run state:

RimLoc

short useful product description

[ Translate a new mod ]
[ Open / update existing translation ]

Secondary subtle actions where useful:
- Base game / DLC localization
- Help

Do not expose advanced concepts.

Returning-user state:

Recent projects

Each project may show:

- mod/content name
- source → target language
- progress
- sourceChanged/issues if relevant
- last modified

[ Continue ]

Do not make Home an analytics dashboard.

===============================================================================
4. QUICK TRANSLATE — FULL CLICKABLE WIZARD
===============================================================================

Implement a realistic mocked wizard:

STEP 1 — What do you want to translate?

Default beginner path:
RimWorld mod

Advanced alternatives:
Base game
DLC
Language pack

STEP 2 — Select content

- auto-detected installed mods
- choose folder
- drag/drop where appropriate

STEP 3 — Languages

Source:
English (auto-detected)

Target:
Russian

Make clear these are PROJECT languages, not UI language.

STEP 4 — Translation method

Options:

Manual
Use existing translation / TM
Integrated AI
External AI

Use plain language.

STEP 5 — Preflight

Example:

1,842 translatable entries
1,219 reusable
623 need translation
14 need attention

STEP 6 — Translate / progress

Show:
- current phase
- progress
- pause/cancel where appropriate
- useful details without log spam

STEP 7 — Result

Example:

4,603 validated
173 need review
36 errors

Primary actions:

[ Review problems ]
[ Open editor ]
[ Build translation ]

===============================================================================
5. EXTERNAL AI FLOW
===============================================================================

Implement the UI mock for users who want to use ChatGPT / Claude / GLM without
connecting an API.

Flow:

Select entries
→ Translate with external AI
→ RimLoc prepares batch
→ options:

[ Copy prompt ]
[ Export JSON ]
[ Import AI response ]

Show clearly that RimLoc will validate imported AI output.

Do NOT implement actual provider/backend behavior yet.

Use realistic mock states.

===============================================================================
6. AI PROVIDER MANAGER
===============================================================================

Build a polished Settings surface for AI providers.

Example providers:

Z.AI / OpenAI-compatible
Anthropic
Ollama / Local
Custom OpenAI-compatible

Each provider card can show:

Connected / Not configured / Offline

model
connection state

Actions:

Configure
Test connection
Change model

Include UI for:

- credentials stored securely
- base URL where applicable
- privacy explanation
- cost warning

Never display real secrets in mocks.

===============================================================================
7. AI QUALITY MODES
===============================================================================

Design understandable quality modes.

Example product language:

Fast draft
Balanced / Quality
Maximum quality
Suggest only

Each explains:

speed
review behavior
possible cost

Do NOT expose temperature/batch/token parameters on beginner surfaces.

Advanced settings may contain them later.

===============================================================================
8. WORKSPACE — PRODUCTIZE THE CURRENT EDITOR
===============================================================================

Keep the current strong three-column editor foundation, but improve UX.

The user should immediately understand:

- project name
- source → target language
- target RimWorld version
- translation progress
- what requires attention

Evaluate top-level Workspace navigation such as:

Editor
Review
Glossary
Translation Memory
Project

Primary actions:

Validate
Compare / Update
Build

Do not overload the header.

===============================================================================
9. REPLACE TECHNICAL-FIRST NAVIGATION
===============================================================================

Current left navigation exposes:

Keyed
DefInjected
TKey

too prominently for normal users.

Keep technical filters available, but move them under something like:

Advanced structure
or
Technical types.

Primary navigation/filtering should prioritize useful concepts:

All entries

Status:
- Untranslated
- Needs review
- Source changed
- Invalid

Content/context groupings where meaningful.

Professional users can still access exact EntryKind.

===============================================================================
10. STATUS FILTER UX
===============================================================================

The current status-chip row is visually reasonable but will not scale forever.

Design a scalable system.

Keep the most useful quick filters visible.

Move additional filter dimensions into a filter popover/panel.

Show active filter count/state clearly.

Support combined filters.

Do not communicate status only through color.

===============================================================================
11. DETAIL PANEL
===============================================================================

Upgrade the right panel.

Use realistic tabs/sections:

CONTEXT
SUGGESTIONS
VALIDATION
HISTORY

CONTEXT:
- source context
- multiple usages
- file/source
- translator notes
- advanced identity details collapsed

SUGGESTIONS:
- TM result
- glossary
- AI suggestions

VALIDATION:
- placeholders
- grammar
- glossary issues
- WordInfo
- technical validation

HISTORY:
- human edit
- imported translation
- TM
- AI
- sourceChanged history

Use realistic mock data.

===============================================================================
12. REVIEW SCREEN
===============================================================================

Implement a dedicated review/QA experience.

Example overview:

Needs review       173
Errors               36
Source changed        24
Glossary conflicts    11

Issue categories:

Placeholder mismatch
Untranslated suspicious text
Glossary inconsistency
WordInfo problem
Source ambiguity
AI review concern

Selecting an issue should open the relevant entry/context directly.

Provide:

[ Fix ]
[ Ignore with reason ]
[ Mark reviewed ]

Mock only for now.

===============================================================================
13. EXISTING TRANSLATION UI
===============================================================================

Implement a realistic mocked flow:

Open/update existing translation

Choose:
- source mod
- existing language pack

Analysis result:

Reusable             1,421
Source changed           37
New                      58
Obsolete                 21
Invalid/orphan             4
Ambiguous                  2

Provide:

[ Review changes ]
[ Continue translation ]

Do NOT imply old work will be deleted automatically.

===============================================================================
14. BUILD / RESULT SCREEN
===============================================================================

Implement a proper final workflow.

Before build:

Translation coverage
Validation status
Known warnings
Output destination

Actions:

[ Build translation ]

After:

Translation built successfully

Output:
...

[ Open folder ]
[ Install translation ]
[ Test / validate again ]

Use plain language.

Advanced details can be expanded.

===============================================================================
15. SETTINGS
===============================================================================

Implement real Settings navigation.

Sections:

GENERAL
- UI language
- startup behavior
- updates

RIMWORLD
- installations
- Workshop
- versions
- discovery
- future optional RimSort integration

TRANSLATION
- default language pair
- autosave
- review defaults
- TM/glossary

AI
- providers
- quality mode
- privacy/cost

EDITOR
- font
- wrapping
- density
- shortcuts

APPEARANCE
- Light / Dark / System
- Style
- Accent palette
- motion
- optional sounds

ADVANCED
- diagnostics
- logs
- knowledge/rules
- developer options

===============================================================================
16. HELP / DIAGNOSTICS
===============================================================================

Implement:

Help

- Quick start
- Replay onboarding
- Keyboard shortcuts
- Documentation
- Troubleshooting
- Diagnose problem
- Prepare bug report / Copy for AI

Use mocks.

This should visually demonstrate the future L/observability workflow.

===============================================================================
17. COMMAND PALETTE
===============================================================================

Implement mocked Cmd/Ctrl+K.

Example actions:

Translate selected
Validate project
Review errors
Build translation
Open project
Compare versions
Open Settings
Configure AI
Run diagnostics
Show shortcuts

Include fuzzy search behavior if inexpensive.

===============================================================================
18. CONTEXTUAL WORKSPACE ONBOARDING
===============================================================================

Implement a skippable 3–4 step coach overlay.

Example:

1. Select a string to edit.
2. Your translation goes here.
3. Filters help find unfinished/problem entries.
4. Validate and build when ready.

Replay from Help.

===============================================================================
19. WORKFLOW PROGRESS / NEXT ACTION
===============================================================================

Introduce clear lifecycle cues.

The UI must answer:

Where am I?
What remains?
What should I do next?

Example:

Translate
→ Review
→ Validate
→ Build

Do NOT force a rigid wizard on professional users.

Use contextual CTAs.

===============================================================================
20. STYLE LAB MUST NOW USE REPRESENTATIVE SCREENS
===============================================================================

The owner should NOT choose between Precision/Aurora/Workshop/Editorial based
only on Home and one editor mock.

Update Style Lab so visual candidates can be compared on the SAME screens:

- Home
- Quick Translate
- Workspace
- Review
- Settings
- AI Provider Manager
- Build/result
- Help/Diagnostics

Each candidate must use the same IA/component behavior.

===============================================================================
21. STYLE VS LAYOUT
===============================================================================

Visual styles should NOT become different products.

Keep:

ONE information architecture
ONE component behavior
ONE navigation logic

Styles may vary curated:

- palette
- surface treatment
- radius
- border strength
- shadow/depth
- limited typography treatment
- motion character

Do not relocate core actions between themes.

===============================================================================
22. STYLE CANDIDATES
===============================================================================

Continue exploring:

Precision
Aurora
Workshop
Editorial

But improve them beyond recoloring.

Examples:

Precision:
- flatter surfaces
- tighter radius
- restrained border/depth
- highly professional dense workspace

Aurora:
- refined layered surfaces
- subtle controlled gradient/light treatment
- slightly softer geometry
- premium Home/onboarding
- restrained inside data-heavy editor

Workshop:
- warm technical/workbench character
- tactile but clean panels
- no faux-metal/gimmick aesthetics

Editorial:
- strongest typography and reading rhythm
- excellent long-text comparison
- subtle content-first visual hierarchy

Do not sacrifice consistency.

===============================================================================
23. LIGHT / DARK / SYSTEM
===============================================================================

All finalist styles must look intentionally designed in:

Light
Dark
System

Do not derive weak light themes from dark themes mechanically.

Test both.

===============================================================================
24. PALETTES
===============================================================================

Use a small curated palette selection.

Examples:

Indigo
Blue
Emerald
Amber

Do not create arbitrary color customization yet.

Semantic status colors remain consistent/readable across palettes.

===============================================================================
25. MOTION IMPLEMENTATION
===============================================================================

Start implementing actual motion in the mock UI.

Use motion tokens.

Add restrained motion for:

- wizard transitions
- panels
- dialogs
- filter state
- selection
- context panel
- saves
- validation results
- progress
- toasts
- onboarding

Do not animate long table row lists individually.

Respect prefers-reduced-motion.

===============================================================================
26. OPTIONAL SOUND MOCK
===============================================================================

Only prototype minimal sound preferences if inexpensive.

Potential events:

long job complete
important failure
successful build

No ordinary button click sounds.

Default conservative/off.

Do not spend meaningful time on audio before core UX is good.

===============================================================================
27. REALISTIC DATA
===============================================================================

Replace simplistic 30-row-only demonstrations with richer mock scenarios.

Use:

- long text
- placeholders
- CJK
- Cyrillic
- multiple contexts
- sourceChanged
- validation errors
- untranslated
- imported
- TM
- AI
- ambiguous states.

No backend dependency.

===============================================================================
28. ACCESSIBILITY AND KEYBOARD
===============================================================================

Preserve/expand:

aria semantics
focus indicators
keyboard navigation
Enter save
Esc cancel
Tab next untranslated
shortcuts.

Test theme contrast.

===============================================================================
29. PRODUCT VISUAL REVIEW
===============================================================================

After representative screens exist, use independent design reviewers.

At least:

visual-design reviewer
accessibility reviewer
professional-workspace reviewer
beginner UX reviewer.

Do not allow the same designer to be the only judge of its work.

===============================================================================
30. OWNER VISUAL SELECTION PACKAGE
===============================================================================

Only after this sprint, present the owner a visual selection package.

For each finalist:

- same Home
- same Quick Translate
- same Workspace
- same Review
- same Settings

Light and Dark.

Short tradeoffs.

Do NOT ask for final style selection before this package exists.

===============================================================================
31. DO NOT WAIT FOR BACKEND
===============================================================================

Continue this entire sprint against mock adapters.

Backend J/K/L continues independently.

Do not modify backend APIs.

After BACKEND FREEZE, replace mocks with the canonical service adapter.

===============================================================================
32. OUTPUT
===============================================================================

Commit implementation in the dedicated frontend branch/worktree.

Update:

GUI_DESIGN_SPEC.md
GUI_STYLE_COMPARISON.md
GUI_MOTION_SYSTEM.md
GUI_UX_ACCEPTANCE.md

But working interactive frontend is the primary deliverable.

Do not stop at documentation.

Continue autonomously.
No push/release/stash/branch deletion.