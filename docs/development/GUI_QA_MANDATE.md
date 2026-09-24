<!-- mandate_id: g5-qa | wave: G5-W1/W4/W5 | scope: owner hands-on QA corrections: wizard branching, providers, editors, diagnostics -->
===============================================================================
(G4/G5 — кумулятивный мандат владельца, 2026-09-24; единый фронтенд-DAG, одна IA)
AI TRANSLATION PRODUCT ARCHITECTURE / GUI INFORMATION ARCHITECTURE
===============================================================================

Before finalizing the RimLoc GUI information architecture, explicitly design
what "automatic translation" means.

Do NOT make a generic "AI Translate" button without a clear quality/workflow
model.

RimLoc is fundamentally a localization workstation, not a black-box machine
translator.

===============================================================================
1. TRANSLATION MODES
===============================================================================

The product should support several ways to translate the same canonical
project:

MANUAL
- human edits directly in Workspace

TM / EXISTING KNOWLEDGE
- translation memory
- glossary
- existing language packs
- reusable prior work

INTEGRATED AI
- configured provider through RimLoc

EXTERNAL AI
- ChatGPT / Claude / GLM / other chat/model environments without direct API
  integration

AI AGENT
- future CLI/MCP/Skill workflows with coding/agent environments.

All methods must operate on the SAME canonical project model.

===============================================================================
2. AI OUTPUT IS A DRAFT UNTIL VERIFIED
===============================================================================

Do not assume machine-generated text is production quality.

Pipeline should conceptually be:

source entry
→ context
→ glossary/TM
→ AI translation
→ deterministic validation
→ optional AI review/repair
→ human review when needed
→ accepted translation.

Track provenance.

Do not silently label AI-generated translations as human-reviewed.

===============================================================================
3. TRANSLATION QUALITY MODES
===============================================================================

Design simple user-facing AI quality modes.

Potential concept:

DRAFT / FAST
- fast first pass

QUALITY
- richer context
- stronger validation
- repair pass

MAXIMUM QUALITY
- translation
- independent review
- repair
- QA
- higher cost/time

SUGGEST ONLY
- AI never applies automatically

Use better product wording if UX research finds it.

Hide low-level model parameters in Advanced settings.

===============================================================================
4. EXTERNAL AI WORKFLOW
===============================================================================

Users must be able to use their existing ChatGPT/Claude/GLM subscriptions
without necessarily configuring an API key.

Design an explicit workflow:

Select untranslated entries
→ "Translate with external AI"
→ RimLoc prepares a structured batch
→ Copy / Export
→ user sends it to external AI
→ Import AI response
→ RimLoc maps by stable IDs
→ validates
→ highlights problems.

Provide useful formats such as:

- copyable prompt
- versioned JSON
- other appropriate interchange format.

Do not require users to manually copy individual strings one by one.

===============================================================================
5. AI BATCH CONTEXT
===============================================================================

RimLoc should prepare better translation context than naïve copy/paste.

A structured batch may include:

source locale
target locale
RimWorld version
mod/project name
entry ID
EntryKind
source text
usage/context
glossary terms
TM examples
protected placeholders
RimWorld grammar syntax
related entries where useful.

Keep batches token-efficient.

Do not send irrelevant project data.

===============================================================================
6. PROTECTED CONTENT
===============================================================================

AI translation must preserve required technical constructs.

Examples include:

- placeholders
- TKey-related semantics
- lookup syntax
- grammar tokens
- identifiers
- markup
- protected names.

Validate after generation.

Automatically retry/repair only where safe.

===============================================================================
7. AI REVIEW
===============================================================================

Separate:

TRANSLATOR MODEL

from:

REVIEWER

where useful.

If the same model is used for both, do not overclaim independent review.

For high-quality mode, evaluate:

translation
→ validation
→ reviewer
→ targeted repair

rather than retranslating everything.

===============================================================================
8. REVIEW QUEUE
===============================================================================

After an automated translation run, present a clear result:

translated
validated clean
needs review
failed
preserved intentionally.

Do not dump users immediately into thousands of rows.

Provide:

"Review problems"

sorted by severity/confidence.

Potential reasons:

- placeholder mismatch
- glossary inconsistency
- suspicious untranslated text
- source ambiguity
- unknown eligibility
- WordInfo issue
- semantic review concern.

===============================================================================
9. PROVENANCE IN EDITOR
===============================================================================

Every translation should be able to expose origin:

Human
TM
AI
Imported
AI-reviewed
Unknown

Where valuable retain:

provider/model
timestamp
review status

without cluttering beginner UI.

===============================================================================
10. PROVIDER MANAGER
===============================================================================

Design a dedicated AI Provider settings experience.

Potential providers:

- OpenAI-compatible
- Z.AI
- Anthropic
- Ollama/local
- additional future compatible providers

Support:

provider status
model
connection test
secure credential storage
base URL where applicable
privacy/cost explanation.

Do not expose secrets.

Normal translation UX should not require users to understand API architecture.

===============================================================================
11. ZERO-API WORKFLOW
===============================================================================

RimLoc must remain fully useful without any AI provider.

Manual translation
TM
existing translation
PO/interchange
validation
build

must all work offline where technically possible.

AI is an enhancement.

===============================================================================
12. COST UX
===============================================================================

Where an integrated provider can cost money:

show an estimate or at least a meaningful cost warning before a large job.

Never send a paid request merely because credentials exist.

Respect the existing explicit-approval rule for agent/testing environments.

===============================================================================
13. QUICK TRANSLATE WIZARD
===============================================================================

Design the beginner Quick Translate flow.

Conceptually:

STEP 1
Choose/detect content

STEP 2
Source / target language

STEP 3
Translation method

STEP 4
Preflight analysis

STEP 5
Translation/progress

STEP 6
Review/result

Use automatic defaults aggressively where safe.

Advanced settings should remain collapsed.

===============================================================================
14. CONTENT TYPES
===============================================================================

Architect the project-creation UI to support:

- Mod
- Existing translation
- Base game
- DLC
- Language pack

Do not overwhelm the beginner Home page.

"Translate a mod" may remain the primary shortcut.

Advanced project creation can expose other content sources.

===============================================================================
15. WORKSPACE NAVIGATION
===============================================================================

Make the localization lifecycle understandable.

Evaluate a user-centered navigation model such as:

Editor
Review
Glossary
Translation Memory
Project

with primary actions:

Validate
Compare / Update
Build

or a better equivalent.

Users must always understand:

where they are
what remains
what next.

===============================================================================
16. SETTINGS INFORMATION ARCHITECTURE
===============================================================================

Design Settings intentionally.

Candidate sections:

GENERAL
- UI language
- appearance
- updates

RIMWORLD
- installations
- Workshop paths
- versions
- mod discovery
- optional RimSort integration

TRANSLATION
- default source/target
- autosave
- TM
- glossary
- review behavior

AI
- providers
- models
- quality mode
- privacy/cost

EDITOR
- font
- size
- density
- wrapping
- shortcuts

APPEARANCE
- Light / Dark / System
- visual style
- curated palette
- motion
- optional sounds

ADVANCED / DIAGNOSTICS
- logs
- doctor
- rules/knowledge
- developer options.

Do not make every CLI flag a Settings toggle.

===============================================================================
17. COMMAND PALETTE
===============================================================================

Evaluate Cmd/Ctrl+K as the power-user bridge.

It may expose:

- open project
- translate
- validate
- build
- compare
- source update
- WordInfo
- diagnostics
- settings
- advanced tools.

This allows broad capability without toolbar clutter.

===============================================================================
18. GUI CAPABILITY PARITY
===============================================================================

Human-meaningful capabilities available in CLI/services should eventually be
reachable from GUI.

This is CAPABILITY parity, not 1:1 flag/control parity.

CLI remains better for automation.

GUI must present operations in human/product language.

===============================================================================
19. DETAIL PANEL
===============================================================================

Evolve the current right-side panel into a useful professional tool.

Evaluate sections/tabs:

CONTEXT
SUGGESTIONS
VALIDATION
HISTORY

Context:
- usages
- source location
- multiple contexts
- notes

Suggestions:
- TM
- glossary
- AI suggestions

Validation:
- structural / language / WordInfo issues

History:
- human / TM / AI / imported / sourceChanged provenance.

Use progressive disclosure.

===============================================================================
20. CONTEXTUAL ONBOARDING
===============================================================================

Beyond first-run onboarding, provide a small first-workspace coach flow.

Examples:

1. Select a string.
2. Edit or accept a suggestion.
3. Use statuses/filters/context.
4. Validate/build when ready.

Skippable.
Replayable from Help.

===============================================================================
21. WORKFLOW CALLS TO ACTION
===============================================================================

Important states should suggest the next logical action.

Examples:

translation complete
→ Validate

validation clean
→ Build

source changed
→ Review changes

issues found
→ Review issues.

Do not require users to infer the workflow.

===============================================================================
22. OPTIONAL SOUND DESIGN
===============================================================================

Evaluate subtle optional application sounds only for meaningful events such as:

- long operation completed
- important error
- successful build

Do NOT add click sounds to normal controls.

Default should remain conservative.

Provide separate sound settings.

System notifications may be more useful than sound alone.

===============================================================================
23. SYSTEM NOTIFICATIONS
===============================================================================

For long background operations evaluate native notifications such as:

"RimLoc finished translating 3,812 entries."

Respect OS/user notification settings.

Do not spam.

===============================================================================
24. VISUAL STYLE STRATEGY
===============================================================================

For a productivity desktop application, prefer ONE stable information
architecture and component grammar.

Visual directions should generally behave as themes/characters rather than
entirely different layouts.

It is acceptable for themes to vary:

- colors
- surfaces
- depth
- radii
- shadows
- limited typography treatment

but feature locations and interaction behavior should remain consistent.

This controls QA complexity.

===============================================================================
25. CURRENT STYLE DIRECTION
===============================================================================

Based on current mock exploration, evaluate a hybrid direction:

Precision:
- professional layout/density

Aurora:
- modern color/depth/polish

Editorial:
- typography/readability improvements

Workshop:
- optional warmer character/theme

Do not treat this recommendation as final owner approval.

===============================================================================
26. HOME PRODUCTIZATION
===============================================================================

Keep Home simple but useful.

First run:
- two clear primary actions
- short explanation

Returning user:
- recent projects
- progress
- quick continue.

Do not leave permanent empty space merely for visual minimalism.

Do not turn Home into a metric dashboard.

===============================================================================
27. BEGINNER TECHNICAL TYPES
===============================================================================

Do not make Keyed / DefInjected / TKey the main conceptual navigation for
ordinary users.

Retain them for advanced/professional inspection.

Prefer user-oriented grouping where meaningful.

Technical type filters can live under an Advanced/Structure section.

===============================================================================
28. NO FINAL GUI LOCK BEFORE BACKEND FREEZE
===============================================================================

Continue implementing this information architecture against mocks/adapters.

Do NOT deeply bind to legacy backend structures before Gate I/J/K backend
contracts are accepted.

===============================================================================
29. DELIVERABLE
===============================================================================

Update the GUI design specification so it contains the complete intended
product information architecture:

- Home
- New project/Quick Translate
- Existing translation
- Workspace
- Review
- Build
- Settings
- Provider management
- Help/Diagnostics
- Command Palette
- whole-game/DLC entry points
- AI/external-AI workflows.

Then apply the previously assigned UX Productization Pass.

Continue autonomously.

No push.
No release.
No stash/branch deletion.