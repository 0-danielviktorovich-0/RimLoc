===============================================================================
(G4/G5 — кумулятивный мандат владельца, 2026-09-24; единый фронтенд-DAG, одна IA)
CHAT-BASED AI TRANSLATION / NO-API ROUND-TRIP
===============================================================================

Add a first-class translation workflow for users who have access to consumer AI
chat products but do NOT have, cannot afford, or do not want to configure API
access.

Examples include ordinary browser/app conversations with:

- ChatGPT
- Claude
- Gemini
- GLM
- other general AI chats.

This must NOT require an API key.

Treat this as a supported product workflow, not a hidden JSON export trick.

===============================================================================
1. USER-FACING NAME
===============================================================================

Avoid developer terminology such as "external provider" on beginner surfaces.

Evaluate wording such as:

Russian:
"Через чат с ИИ — без API"

English:
"Use AI chat — no API key"

with a concise explanation:

RimLoc prepares the text and context.
The user sends it to their AI chat and imports the response back.

Advanced documentation may call this:

External AI batch workflow.

===============================================================================
2. SAME CANONICAL PROJECT MODEL
===============================================================================

Chat-based translation must use the SAME canonical SourceEntry / Translation
model as:

- manual editing
- integrated AI
- TM
- existing translations
- future MCP/agents.

Do NOT create a separate chat-translation project representation.

===============================================================================
3. BASIC USER JOURNEY
===============================================================================

Implement/design the beginner workflow:

Select untranslated/problem entries
→ Translate with AI chat
→ RimLoc prepares a batch
→ Copy prompt / Export batch
→ user sends it to ChatGPT/Claude/GLM/etc.
→ user receives response
→ Paste response / Import response file
→ RimLoc validates and maps results
→ preview conflicts/problems
→ apply valid translations
→ review invalid/uncertain entries.

No manual per-string copy/paste should be required.

===============================================================================
4. BATCH MANAGER
===============================================================================

Large projects must be split into manageable resumable batches.

Track per batch:

NOT STARTED
EXPORTED / COPIED
WAITING FOR RESPONSE
IMPORTED
PARTIAL
NEEDS REVIEW
DONE.

Show progress such as:

Batch 3 of 8
72 / 76 accepted
4 need review.

Provide:

[ Copy next batch ]

after successful import.

Users should be able to stop today and continue tomorrow.

===============================================================================
5. STABLE BATCH IDENTITY
===============================================================================

Each batch must have stable structured identity.

At minimum evaluate:

project_id
target_locale
batch_id
schema_version
entry IDs
entry/source revisions
relevant project/source fingerprint.

Never rely on response order alone.

===============================================================================
6. ENTRY PAYLOAD
===============================================================================

Include only useful structured context.

Potential fields:

entry_id
source_revision
source text
source locale
target locale
EntryKind
RimWorld/mod context
usage/context
glossary constraints
TM examples where valuable
protected placeholders/tokens
related terminology
current translation where appropriate.

Do not include unnecessary entire project/mod content.

Keep token usage efficient.

===============================================================================
7. PROMPT GENERATOR
===============================================================================

Generate a high-quality provider-neutral prompt automatically.

Requirements should include:

- translate to target locale;
- preserve every entry ID;
- do not skip entries silently;
- preserve protected tokens exactly;
- respect glossary;
- preserve RimWorld grammar constructs;
- return structured response only;
- mark uncertainty rather than hallucinating.

Allow provider-specific prompt variants later if they materially improve
reliability.

The normal user should not need to write a translation prompt manually.

===============================================================================
8. RESPONSE FORMAT
===============================================================================

Define a stable versioned response schema.

Conceptually:

batch_id
entries:
  id
  translation
  needs_review
  optional note/reason.

Do not require models to echo large source/context fields unnecessarily.

Keep response compact and robust.

===============================================================================
9. HUMAN-FRIENDLY IMPORT
===============================================================================

Normal user import options:

- Paste AI response
- Import response file.

Do not require the user to understand JSON.

If parsing fails, explain what is wrong and offer:

- retry/copy corrected response instruction
- manual review
- partial recovery where safe.

Do not discard correctly parsed entries because one entry is malformed if
partial safe import is possible.

===============================================================================
10. UNTRUSTED RESPONSE BOUNDARY
===============================================================================

Treat AI chat output as untrusted data.

It must NEVER be interpreted as:

- file paths to write arbitrarily
- shell commands
- configuration commands
- rule-pack code
- executable instructions.

Only accept translation fields defined by the response schema.

Unknown fields must not grant capabilities.

===============================================================================
11. ID VALIDATION
===============================================================================

Reject or flag:

- unknown entry IDs
- duplicate IDs
- wrong batch ID
- wrong project
- wrong target locale
- missing required entries
- entries from another batch.

Never map by text/order alone when stable ID exists.

===============================================================================
12. REVISION / STALE RESPONSE PROTECTION
===============================================================================

A batch captures source/translation revision.

If the user edits an entry after batch export:

AI response MUST NOT silently overwrite the newer human work.

Conceptually:

export translation revision N

if current revision == N:
safe to apply

if current revision > N:
conflict
→ AI result becomes suggestion
→ preserve current human text.

Apply per target locale.

Add regression tests.

===============================================================================
13. SOURCE CHANGE PROTECTION
===============================================================================

If source text changed after batch export:

do not blindly import the old translation.

Mark:

SOURCE_CHANGED / STALE_BATCH

and require review/retranslation.

===============================================================================
14. STRUCTURAL VALIDATION
===============================================================================

Before applying imported response validate:

- placeholders
- TKey/lookup constructs
- RulePack/grammar tokens
- identifiers
- markup
- XML-sensitive content
- glossary constraints where deterministic
- expected entry completeness.

Clean entries may apply.

Problems enter Review.

===============================================================================
15. PREVIEW / APPLY
===============================================================================

Before applying a batch show a concise result:

76 responses

71 ready to apply
3 structural problems
1 stale human edit
1 missing response

Actions:

[ Apply 71 safe translations ]
[ Review 5 problems ]

Do not make users choose entry by entry when most are clean.

===============================================================================
16. PROVENANCE
===============================================================================

Imported translations should retain provenance such as:

AI_CHAT

plus optionally:

declared provider/model
batch ID
timestamp
review state.

Do not pretend provider/model is verified when the user merely selects/types
it manually.

User may optionally label:

ChatGPT
Claude
GLM
Gemini
Other/Unknown.

===============================================================================
17. MULTIPLE AI CHATS / PROVIDERS
===============================================================================

One project may use different chat products for different batches.

Example:

Batch 1–3 → ChatGPT
Batch 4–5 → Claude
Batch 6–8 → GLM.

RimLoc should not care as long as response schema is valid.

Preserve per-entry/batch provenance.

===============================================================================
18. COPY / DOWNLOAD OPTIONS
===============================================================================

Offer appropriate options:

[ Copy prompt ]
[ Copy data + prompt ]
[ Export AI batch ]

Advanced export may produce:

prompt.md
batch.json

or a self-contained bundle.

Do not require files for the simplest workflow.

===============================================================================
19. IMPORT OPTIONS
===============================================================================

Offer:

[ Paste response ]
[ Import response file ]

Future integration may support direct share/open workflows.

Do not require clipboard access where platform policies prevent it.

===============================================================================
20. TOKEN-AWARE BATCHING
===============================================================================

Automatically choose reasonable batch sizes based on:

- source length
- context size
- glossary
- target model/chat constraints where known.

Default:

Auto.

Advanced settings may allow:

smaller / standard / larger context batches.

Do not ask beginners for token counts.

===============================================================================
21. STRUCTURAL FINGERPRINT / REPEATED CONTEXT OPTIMIZATION
===============================================================================

Avoid repeating identical context/glossary data unnecessarily in every entry.

Where format allows, provide shared batch-level context.

Optimize for consumer-chat context limits.

Do not sacrifice clarity/reliability for extreme compression.

===============================================================================
22. DIFFICULT-ENTRY CHAT WORKFLOW
===============================================================================

Allow a translator to send only selected difficult entries to external AI.

Example editor action:

Ask AI chat about selected entry

RimLoc prepares:

source
current translation
context
glossary
validation issue
related terms.

User can import the suggestion safely.

===============================================================================
23. NO PROVIDER LOCK-IN
===============================================================================

Generated batch format should be provider-neutral.

Do not make the user choose ChatGPT-specific format merely to use consumer chat.

Provider-specific prompt tuning may exist as an optional presentation layer.

===============================================================================
24. EXTERNAL CHAT QUALITY MODES
===============================================================================

Where useful, prompt generation may support:

Draft
Quality
Maximum quality
Review only.

For Maximum quality, RimLoc may generate separate prompts:

Translation pass
Review pass

without assuming the same chat/model is independent evidence.

Explain expected extra work.

===============================================================================
25. OFFLINE / MANUAL COMPATIBILITY
===============================================================================

The project remains fully usable even if the user never completes a chat batch.

They can always:

- edit manually
- use TM
- import existing translation
- validate/build.

Chat workflow is optional.

===============================================================================
26. GUI PLACEMENT
===============================================================================

Expose chat-based AI in:

Quick Translate strategy
and
Workspace translation actions.

Possible Workspace actions:

Translate untranslated with AI chat
Prepare selected for AI chat
Import AI response.

Do not hide it only inside Settings.

===============================================================================
27. BATCH HISTORY
===============================================================================

Project should retain useful batch history:

batch ID
target
entries
export time
import status
provenance.

Allow users to see:

which batches are unfinished.

Do not store the entire chat transcript unless user explicitly imports it.

===============================================================================
28. PRIVACY PREVIEW
===============================================================================

Before export/copy, explain that selected source text/context will be sent by the
user to an external AI service.

Allow preview of what RimLoc generated.

Do not include:

API keys
user secrets
irrelevant filesystem paths
private project notes not needed for translation

unless explicitly selected.

===============================================================================
29. "COPY NEXT BATCH" UX
===============================================================================

Optimize repeated workflow.

After import:

Batch 3/8 complete

[ Review problems ]
[ Copy next batch ]

Avoid forcing user back through the wizard.

===============================================================================
30. IMPORT ROBUSTNESS TEST CORPUS
===============================================================================

Test responses containing:

- reordered entries
- missing entry
- duplicate entry
- unknown ID
- malformed JSON
- Markdown code fences
- commentary before/after JSON
- altered placeholders
- stale revision
- wrong target locale
- partial response
- valid multilingual Unicode.

Parser may be user-friendly but must remain strict about semantic identity.

===============================================================================
31. COPY-PASTE ACCEPTANCE TEST
===============================================================================

Run an actual end-to-end controlled acceptance:

RimLoc project
→ create AI-chat batch
→ copy prompt/data
→ give it to an independent chat/model
→ receive response
→ paste/import
→ validate
→ apply
→ build.

No API integration may be used in this acceptance.

Document usability problems.

===============================================================================
32. AI SKILL / AGENT COMPATIBILITY
===============================================================================

Use the same batch schema for future RimLoc AI skills/agents where useful.

A coding/AI agent may consume/export the structured batch directly without
manual copy/paste.

Do not create a separate incompatible agent translation format.

===============================================================================
33. DOCUMENTATION
===============================================================================

Later public docs should have a beginner guide:

"Translate with ChatGPT/Claude/another AI chat without an API key."

Explain:

copy
→ send
→ import
→ review

in a short visual tutorial.

Do not require API terminology.

===============================================================================
34. ACCEPTANCE
===============================================================================

Before calling chat-based translation ready prove:

- no API key required;
- user can translate several batches across multiple sessions;
- model response order does not matter;
- unknown IDs cannot modify project;
- stale AI response cannot overwrite human edits;
- sourceChanged batch is blocked/reviewed;
- placeholders are validated;
- partial valid import is recoverable;
- target locale cannot be mixed;
- provenance remains;
- real translated output builds successfully;
- workflow is understandable to a nontechnical user.

Continue autonomously.

No push.
No release.
No stash/branch deletion.