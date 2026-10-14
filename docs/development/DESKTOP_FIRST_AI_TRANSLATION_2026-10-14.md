# Desktop-first RimLoc & AI Translation — product contract (2026-10-14)

**Status: owner product decision / implementation pending; NOT shipped.**
**Protected baseline:** rel26 data-safety candidate `089b424`; Draft PR #90 must remain isolated, unmerged until owner UAT.

## 1. Product truth

RimLoc is a visual, local-first desktop studio for translating RimWorld mods.
The default user's journey must NOT require a terminal, CLI, PO, XML editing,
environment variables or manual technical configuration:

**Open mod → choose target language → translate (manual or AI) → review →
validate → safely build/export → optionally verify in RimWorld.**

React/Tauri GUI is the primary product, onboarding, docs/README/About and
acceptance surface. Shared Rust domain/services own all real behavior. The CLI
is optional automation/expert access to those services, NEVER the public
product's default identity. Agents must not use an arbitrary shell in the GUI
as a substitute for typed shared-service operations.

Do not claim unshipped functions. Older game adapters remain future work.

## 2. Verified existing backend/UI facts

- `frontend-react/src/components/ChatBatch.tsx`: real durable human
  copy/paste AI chat flow, no API calls by design.
- `ProvidersScreen.tsx`, `lib/state/providers.ts`, `rimloc-services/src/providers.rs`:
  provider-instance metadata, OS Keychain secrets, validation, real connection
  probe. **Connection test ≠ automatic translation workflow**.
- `rimloc-llm`: Provider abstraction, mock, HTTP OpenAI-compatible/Anthropic,
  batched TranslationEngine, checkpoint, retries, cost/placeholder validation.
- `rimloc-cli/src/commands/translate.rs`: actual AI engine → canonical
  `ProjectSessionManager::apply`. Currently a CLI workflow that creates a
  managed project — must NOT be launched as a shell child from GUI.
- There is no connected direct translation job in React GUI; do not label
  Provider screen or ChatBatch as working automatic in-app translation.

## 3. GUI information architecture — first-class AI Translation

Add **AI Translation / ИИ-перевод** as a primary discoverable nav item.
Use beginner-oriented choices within ONE screen (existing ChatBatch route/history
must remain backward compatible, not deleted):

1. **Translate automatically** — choose current project, target locale,
   scope (untranslated/selected/changed), provider/model, cost/privacy notice,
   review and explicitly start; show progress, errors, cancel, retry, resume.
2. **ChatGPT plan (no API key)** — OAuth consent, account selector, eligible
   models and plan-usage state, no hidden setup.
3. **Bring your API key** — OpenAI, Anthropic, GLM/Z.ai and supported
   compatible providers; real job, model, limit/spend controls, OS Keychain.
4. **Local models** — optionally Ollama/LM Studio-compatible local models,
   honest runtime availability, model discovery and resource requirements.
5. **AI chat without API** — existing durable chat batch prompt → answer →
   strict preview/apply, simple access for nontechnical users.
6. **History/review/connections** — reusable jobs, account limits, undo,
   provenance and error details. Provider Settings is an advanced subview.

No mandatory terminal or onboarding CLI steps; no artificial functional
parity via fake buttons or embedded terminal.

## 4. Official ChatGPT plan integration for this OSS desktop client

**New important opportunity:** OpenAI published an official *Sign in with
ChatGPT* OSS direct-plan-sharing flow in late September 2026. RimLoc is public,
GPL-3.0 and local-first, making the OSS implementation path worth testing.
Do **not** assume an arbitrary subscription already grants API access, or
that all models/features/accounts/regions are eligible. Plan usage must be
authorized separately from identity sign-in.

Implement OpenAI's documented OSS **dynamic OAuth registration**:
- stable opaque per-host `ext_agent_host_id` (one per device, stored);
- system browser and temporary `http://127.0.0.1:<port>/auth/callback`;
- OAuth state, nonce, PKCE-S256; first client ID `dynamic_agent_client`,
  persist the issued per-account/per-workspace client ID;
- `openid profile email offline_access resource.invoke
  chatgpt.tokens.use.direct` scopes; actual consent and grant verification;
- validate ID token signature/JWKS/issuer/audience/nonce;
- safely store refresh/access tokens in OS Keychain, serialize refresh,
  account switching, reauth, disconnect and revocation;
- use OAuth bearer in supported Responses API route, with `store:false`,
  `stream:true` and explicit array `input`, only allowed fields;
- claim a successful job ONLY after `response.completed`, never on an
  interrupted stream or `response.failed`. Support usage limits, lost
  eligibility and disconnection without silent fallbacks;
- model selector reflects actual available entitlement or a verified
  permitted request, never guesses "ChatGPT Pro => all models".

Official references to read at implementation time:
- https://developers.openai.com/siwc/token-sharing-open-source
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
- https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
- https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions
- https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery
- https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt

**No OAuth cookies/scraping or borrowed Codex credentials**. Direct ChatGPT
subscription sharing via this official flow is separate from paid OpenAI API
billing. Subscription use consumes eligible plan usage and may have weekly
limits. Obtain real user consent in browser; automation must NOT approve
permissions or purchase credits.

## 5. Other models and transports

**BYO API keys:** through existing provider metadata/Keychain interface;
never save/log secrets, no key in React state or managed projects. A
connection probe is not permission to start paid translation. Show budget,
anticipated usage where possible and a clear "Start" with cloud-data notice.
No surprise paid fallback when subscriptions run out.

**Local models:** detect existing local runtime and models; never silently
download GBs of models, alter system services or consume unlimited resources.
Model missing/not running/unsupported must be shown honestly. Optional
one-click guided install only with user permission. Avoid requiring Windows.

**Subscription-based official third-party runtimes:** separate research
track; use only supported official integrations under each provider's
terms. Do not assume "Claude/ChatGPT subscription" = an arbitrary API key.
A legitimate CLI/SDK bridge is possible where provider explicitly supports
it; no browser session scraping or opaque auth-token reuse.

**Manual chat:** preserve the no-API workflow even after direct AI is added.

## 6. Shared job service and data safety (NON-NEGOTIABLE)

Refactor the shared Rust layer, not duplicate the `translate` CLI command.
Add a typed service-owned job/attempt with:
- project/session identity, locale, full canonical `SourceEntryId`,
  revision/epoch/source fingerprint, provider account/model/transport;
- optional glossary/TM suggestions, batching, concurrency limits,
  reliable progress, cancellation, persisted checkpoints, retry and resume;
- output validation for placeholders, markup/XML, eligibility, formatting,
  language and identity; model output becomes DRAFT/PROPOSAL until accepted;
- previews with exact changed rows, conflicts, skipped/error reasons and
  clear provenance, manual confirmation and version-stale handling;
- apply via existing canonical `project_apply`; preserve rel26 draft
  autosave, restore, clear, pending-deletion ACK, target-locale isolation.
- optional external request sends only selected source text/context, never
  raw folders/paths/mod secrets. Expose cloud/on-device privacy difference.
- errors and cancellation cannot claim "saved", drop drafts or silently
  overwrite more recent manual edits.

Existing `Provider` trait/engine and Tauri commands should be reused where
sound; long-running network calls belong in a worker async/job service, never
block the GUI main thread. Real per-row money/usage reporting only when
available, marked as estimates otherwise.

## 7. GUI/CLI parity — capability not duplicated shell flags

Create a truth matrix: capability | human GUI entrypoint | Rust shared
service | optional CLI command | current state | real E2E evidence | gap.
All user-relevant scenarios should be accessible by visible GUI; technical
diagnostics may remain CLI-only if justified, but cannot count toward full
GUI parity. A hidden or disconnected control is a GAP, not "implemented".

Reconcile product/docs/README/GitHub About around the GRAPHICAL STUDIO.
Only promise currently functioning capabilities; label future AI modes as
"planned" until packaged application acceptance.

## 8. Acceptance and delivery plan

Stages: A) canonical GUI-first IA & UX + connection choices;
B) direct BYO-API and local mock actual background jobs and review;
C) official OSS ChatGPT OAuth complete sign-in and streaming;
D) no-API route migration, CLI↔GUI audit and comprehensive UAT.

Acceptance MUST exercise:
- newbie real-mod GUI journey without opening terminal;
- no-API structured round-trip EN→RU, clean apply;
- mock BYO API + local mock running through GUI→Tauri→services→provider,
  validated apply and restart;
- OAuth allow/decline/refresh/revoke/multi-account/limit/interrupted-stream
  via deterministic mocks; an optional OWNER-authorized live sign-in;
- no paid live request in unattended CI; no account consent bypass;
- screenshots of packaged React .app at ordinary and increased UI scale,
  light/dark, incl ChatBatch/error/review; actual usability inspection;
- Rust+React+IPC+native WDIO tests on exact source/binary identity;
- GitHub CI/CodeQL/Codecov and **bot comments** on exact PR HEAD;
- UAT acceptance from owner before merge/release/community publication.

Develop in separate worktrees/branches; do not disturb the audited rel26
binary or existing Draft PR #90. Safe commits, push and Draft PR are
authorized. **Merge, release/pre-release, Discord, paid calls and OAuth grant
approval remain owner-controlled.**

## 9. Local agent instruction (the owner does not configure a terminal)

Read this document and `AGENTS.md` and inspect the current worktrees. Use
the existing agent runtime to IMPLEMENT the stages, not merely report.
Reuse stored official authentication if available lawfully; do not invent
credentials or access another account. Install only necessary dev deps in
isolated test environments, run packaged Tauri acceptance, fix bugs and
deliver a macOS ARM64 owner UAT build. Do not ask the owner to type shell
commands or copy API keys. The only unavoidable personal actions are
consenting to official provider sign-in and separately accepting paid
charges/limits, if applicable. Report precisely what is implemented,
verified, unsupported and awaiting owner action.
