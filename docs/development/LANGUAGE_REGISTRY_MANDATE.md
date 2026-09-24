===============================================================================
(G4/G5 — кумулятивный мандат владельца, 2026-09-24; единый фронтенд-DAG, одна IA)
OWNER HANDS-ON GUI QA — FUNCTIONAL UX CORRECTION PASS
===============================================================================

The owner manually explored the current G4 interactive frontend and provided
real usability feedback.

Treat this as user-testing evidence.

Do NOT replace the current IA from scratch.

The GUI has improved substantially. Preserve what works and fix the observed
product/interaction defects.

Run this work as a parallel frontend QA/productization stream while backend
J/K/L continues.

===============================================================================
1. NEW TRANSLATION NAMING
===============================================================================

The Home action "Translate a new mod" is now too narrow because the creation
flow supports:

- mod
- base game
- DLC
- language pack.

Rename/rethink the primary action.

Preferred direction:

"New translation"
or
"Create translation"

with explanatory subtitle:

Mod, base game, DLC or language pack.

Use Russian/English copy that sounds natural.

Do not advertise unsupported production capability before backend acceptance,
but the mock IA may represent the intended product.

===============================================================================
2. WIZARD MUST BE A REAL STATE MACHINE
===============================================================================

The owner found functional bugs:

- Back does not work correctly in some paths;
- choosing Base Game still shows mod selection;
- choosing DLC still shows mods;
- choosing Language Pack still shows mods.

Fix the wizard architecture.

The selected source type must determine subsequent steps.

MOD:
→ discovered mods
→ manual folder/drop

BASE GAME:
→ detected RimWorld installations
→ choose Core/version

DLC:
→ detected installation
→ choose installed DLC(s)

LANGUAGE PACK:
→ choose/detect existing language pack source
→ appropriate language/source mapping

No copy/paste reuse of a "mods" step under unrelated content types.

Add state-machine/component tests and E2E navigation tests for EVERY source
type including Back/Next.

===============================================================================
3. LANGUAGE PAIR — VERIFY TRUE MULTILINGUAL UI
===============================================================================

Do not assume Russian is the only target merely because current mock defaults
to Russian.

Audit the frontend model.

The source and target controls must be populated from the project locale/language
registry.

Prove the mock UI can create/display multiple pairs, at minimum examples such as:

English → Russian
English → Japanese
German → Ukrainian
Japanese → English

where backend capabilities permit.

UI language RU/EN remains completely independent from project language pair.

Add E2E evidence.

===============================================================================
4. AUTO-DISCOVERY / FRIENDLY MOD NAMES
===============================================================================

The intended real UX is:

RimLoc automatically discovers likely RimWorld installations and mods.

Users should see:

mod title
author
version
packageId where useful

NOT unexplained Workshop numeric folder names.

Workshop folder IDs remain advanced metadata.

Support:

- automatic detection
- manual folder selection
- drag/drop where supported

The current "Choose folder" interaction must work in the mock/E2E and later
through Tauri.

===============================================================================
5. STEAM / GOG / MANUAL INSTALLATIONS
===============================================================================

Current installation cards are mock data and must not be treated as detection
evidence.

Before production binding, test real discovery for:

- Steam
- GOG
- manual/custom installation
- multiple installations
- machine without Steam.

Clearly mark the active/default installation.

Workshop support must not require Steam client assumptions where alternatives
exist.

Evaluate SteamCMD only as an explicit optional Workshop acquisition mechanism,
not hidden automatic behavior.

RimSort integration remains optional.

===============================================================================
6. PROVIDER MANAGER — TEMPLATES + INSTANCES
===============================================================================

Redesign provider architecture in the mock UI around:

PROVIDER TEMPLATES
+
CONFIGURED PROVIDER INSTANCES.

Templates may include:

- Z.AI
- Anthropic
- OpenAI
- Ollama/local
- OpenAI-compatible/custom
- other genuinely supported providers.

Users must be able to create MULTIPLE configured instances.

Examples:

Z.AI Personal
Z.AI Team
Local Ollama
My VPS
Custom Provider A
Custom Provider B

Add:

[ + Add provider ]

Custom provider form should evaluate:

- user-defined name
- API/protocol family
- base URL
- authentication method supported by that provider
- model discovery/manual model
- extra headers/options where safe
- test connection.

Do not imply consumer subscription authentication works as API access unless
the provider officially supports such authentication.

Credentials must remain secure/non-rendered.

===============================================================================
7. PROVIDER LIST MANAGEMENT
===============================================================================

Configured provider instances should support:

- enable/disable
- default provider
- rename
- duplicate
- edit
- remove
- test connection
- choose default model.

Templates and configured instances must be visually distinct.

===============================================================================
8. CONFIGURATION INVENTORY BEFORE GENERIC CONFIG UI
===============================================================================

The owner saw import/export/config concepts but could not understand what a
"config" represents.

Audit ALL user-editable configuration types in the product.

Classify them by domain:

- app settings
- project settings
- provider profiles
- glossaries
- translation memory
- knowledge/rule packs
- shortcuts
- appearance profiles
- other real config types.

Do NOT create one generic JSON "Config Editor" merely because multiple things
serialize to configuration files.

Each domain should have an appropriate dedicated editor.

===============================================================================
9. PROFILES
===============================================================================

Where several complete settings configurations are genuinely useful, evaluate
named Profiles.

Example:

Default
Russian Translator
Offline Local
Maximum Quality AI
Team Profile

Users should be able to switch among imported/created profiles where semantics
make sense.

Do not merge unrelated domain data into a giant opaque profile without careful
design.

===============================================================================
10. GLOSSARY EDITOR
===============================================================================

The current Glossary screen is only a skeleton.

Design/implement a realistic editor mock:

- search
- add
- edit
- delete
- source term
- target term
- notes/context
- language pair
- scope
- variants
- case behavior where relevant
- import/export
- conflict view.

Support project/user/global scopes only where the canonical backend eventually
supports them.

===============================================================================
11. TRANSLATION MEMORY EDITOR
===============================================================================

The current TM screen is also a skeleton.

Design a useful professional interface:

- search
- source
- target
- language pair
- similarity
- origin/project
- provenance
- quality/review state
- use/apply
- edit/delete where allowed
- import/export
- duplicate/conflict handling.

Do not expose thousands of rows without filtering/virtualization planning.

===============================================================================
12. KNOWLEDGE / RULE EDITOR
===============================================================================

Gate J introduces declarative Eligibility/Knowledge rules.

Design an ADVANCED editor/inspector for these rules.

Normal users should not need it.

Capabilities may include:

- explain why an entry is translatable/non-translatable
- inspect applied rule
- project/mod-scoped override
- user rule
- import/export rule pack
- conflicts
- provenance
- scope.

Do not expose raw JSON as the only way to edit rules.

Provide raw/advanced view optionally.

===============================================================================
13. ONBOARDING REPLAY BUG
===============================================================================

The owner clicked:

Replay tips / Show tips

and did not see the onboarding.

Treat this as a bug.

Add E2E:

fresh workspace
→ onboarding visible

skip/complete
→ persistent state

Help → Replay onboarding
→ onboarding visibly reappears

No silent button.

===============================================================================
14. KEYBOARD SHORTCUT EDITOR
===============================================================================

Upgrade Help shortcut display into configurable keyboard shortcuts.

Use familiar platform conventions by default.

Examples:

Cmd/Ctrl+S — Save
Cmd/Ctrl+Z — Undo
Cmd+Shift+Z / suitable Windows equivalent — Redo
Cmd/Ctrl+F — Search
Cmd/Ctrl+K — Command palette
Esc — close/cancel

RimLoc-specific actions:

next untranslated
previous/next issue
mark reviewed
translate selected
validate
build where appropriate.

Allow remapping.

Detect conflicts.

Provide reset-to-default.

Handle macOS vs Windows/Linux display conventions correctly.

===============================================================================
15. SIDEBAR / PANEL COLLAPSE
===============================================================================

Add editor-quality panel controls.

Users should be able to:

toggle left sidebar
toggle right context panel

Use familiar panel-layout icons.

Remember preference where appropriate.

Provide keyboard commands via Command Palette/shortcuts.

On narrow windows collapse panels intelligently.

===============================================================================
16. DIAGNOSTICS MUST BE ROOT-CAUSE USEFUL
===============================================================================

"Diagnose problem" and support bundles must maximize debugging usefulness.

Capture structured causal context, not only final error strings.

Where relevant include:

operation ID
operation stage
preceding relevant events
project/mod metadata
RimWorld version
source/target locale
provider state
validator state
error chain
expected vs actual
affected entry IDs
timings
reproduction steps.

Avoid irrelevant log floods.

===============================================================================
17. SUPPORT BUNDLE REDACTION
===============================================================================

Implement/plan strict sanitization.

Never include by default:

- API keys
- Authorization headers
- tokens
- passwords
- Keychain values
- signing credentials
- unrelated environment variables
- unnecessary personal paths
- unrelated user files
- complete copyrighted mods when a minimized reproduction is enough.

Normalize/redact home paths where practical.

Provide a PREVIEW before export:

Included:
...
Redacted:
...

Allow safe user review.

===============================================================================
18. DIAGNOSTIC BUNDLE QUALITY TEST
===============================================================================

Create a controlled known bug and prove:

support bundle
→ independent AI/debug agent
→ enough evidence to reproduce/root-cause the problem.

This is the acceptance standard.

Not:

"bundle contains logs".

===============================================================================
19. UI INSTALLATION DETECTION MOCK LABELS
===============================================================================

Mock data must be visually distinguishable in development where confusion is
possible.

The current RimWorld screen explicitly says `(mock)` in one place; preserve a
development-only indication.

Never let mock version values become production capability claims.

===============================================================================
20. WORKSPACE LIFECYCLE TERMINOLOGY
===============================================================================

Review the current sequence:

Translate
Review
Validation
Build

"Review" vs "Validation" may be conceptually unclear to beginners.

Test alternatives.

Possible simplification:

Translate
→ Check
→ Build

where Check contains:

Language/quality review
Technical validation

OR retain separate stages with clearer wording, such as:

Translate
→ Review translation
→ Technical check
→ Build.

Choose based on usability testing.

Do not optimize terminology for developers.

===============================================================================
21. REVIEW COUNTER SCOPE
===============================================================================

Current Review mock mixes project-wide counters with session-loaded issue
queues.

This distinction is confusing.

For production UX, prefer one consistent project-wide mental model.

Lazy loading/virtualization may happen internally without changing the apparent
scope.

If scopes differ, label them extremely clearly.

Do not make users understand implementation/session loading.

===============================================================================
22. PROJECT SCREEN PRODUCTIZATION
===============================================================================

Current Project screen is too sparse.

Develop it into a useful project overview/settings surface.

Potential sections:

- content/mod
- source/target languages
- RimWorld version
- source path
- output path
- translation progress
- project health
- source-update status
- imports/exports
- project-specific glossary/TM/rules references
- dangerous/reset/archive actions in clearly separated area.

Do not duplicate global Settings.

===============================================================================
23. HOME / RECENT PROJECTS
===============================================================================

Current Home improvement is good.

Continue refining:

- friendly project names
- language pair
- progress
- issues/sourceChanged
- last modified
- Continue

Consider auto-discovered untranslated mods only if it remains useful and not
noisy.

===============================================================================
24. ABOUT / CREDITS
===============================================================================

Add an About surface under Help or Settings.

Include later:

- RimLoc version
- project description
- founder/lead maintainer
- GitHub/project link
- license
- contributors
- third-party notices.

Owner information may identify:

Daniel Kamyshan
as founder/lead maintainer if that remains the desired public presentation.

Prefer contributors derived from project history/release data where possible
rather than a manually stale list.

Do not expose private account information.

===============================================================================
25. VISUAL POLISH
===============================================================================

Do not redesign again from zero.

The current component system is substantially improved.

Continue polishing:

- reduce unexplained empty space
- strengthen hierarchy on sparse screens
- consistent card/content widths
- clearer interactive states
- better selected/focus state
- typography
- dark-theme parity
- responsive/narrow-window behavior
- subtle motion
- reduced motion.

Preserve information clarity.

===============================================================================
26. STYLE SELECTION STILL WAITS
===============================================================================

Do not ask the owner to choose the final Precision/Aurora/Workshop/Editorial
style yet.

First ensure representative product screens and interactions are functionally
correct.

Then produce the planned identical-screen comparison package.

===============================================================================
27. AUTOMATED REGRESSION FOR OWNER-FOUND BUGS
===============================================================================

Every owner-reported functional GUI bug from this review should become a
frontend/component/E2E regression where practical.

At minimum:

- Back navigation
- content-type wizard branching
- onboarding replay
- arbitrary target language selection
- provider add/multiple instances
- sidebar/panel toggles.

Do not only fix manually.

===============================================================================
28. HUMAN GUI CAPABILITY PARITY AUDIT
===============================================================================

Inventory the current human-meaningful CLI/service capabilities.

Map each to:

GUI EXISTS
COMMAND PALETTE
ADVANCED GUI
AUTOMATION-ONLY BY DESIGN
NOT YET REPRESENTED.

Every capability useful to a human should have an understandable GUI path or
an explicit documented reason not to.

Do NOT create a button for every CLI flag.

===============================================================================
29. PRODUCT QA PASS
===============================================================================

After implementing these corrections, run a fresh-user walkthrough again:

ordinary player
professional translator
existing-translation maintainer
AI-assisted user
offline/no-AI user
Windows-oriented user.

Record hesitation/failure points.

Fix P0/P1 UX issues before calling GUI productized.

===============================================================================
30. EXECUTION
===============================================================================

Use GUI/frontend subagents in parallel where ownership does not conflict.

This work remains mock/service-adapter based until backend freeze.

Backend J/K/L continues independently.

Do not block backend work waiting for visual preferences.

Do not stop merely to report these fixes.

Continue until the GUI mock is coherent enough for a real user test and final
style comparison.

No push.
No release.
No stash/branch deletion.