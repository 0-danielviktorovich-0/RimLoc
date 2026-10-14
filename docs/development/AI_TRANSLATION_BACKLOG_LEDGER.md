# AI Translation — remaining product backlog ledger (2026-10-14)

Companion to `DESKTOP_FIRST_AI_TRANSLATION_2026-10-14.md`. Classifications:
`IMPLEMENTED` / `NOT_IMPLEMENTED` / `PARTIAL` / `EXTERNAL_CREDENTIAL_REQUIRED`
/ `OWNER_DECISION_REQUIRED`. Owner gates are never used as a substitute for
engineering work — anything coded is marked on its own line.

| # | Item | Status | Evidence / what is missing |
|---|------|--------|----------------------------|
| 1 | Cancellation of AI jobs (review 2026-10-14 P1) | **IMPLEMENTED** | `EngineOptions::cancel` checked at every engine boundary; terminal `Cancelled` guarded in the worker (`ai_job.rs`); 11 deterministic regressions (engine + service) + native GUI cancel lane |
| 2 | Mock gating in production | **IMPLEMENTED** | backend env gate `RIMLOC_ALLOW_MOCK_JOBS`; production frontend omits mock from every mode (asserted in the native lane) |
| 3 | Truthful apply outcomes | **IMPLEMENTED** | `accept_job` maps `cancelled` / `applied=0` / partial / empty-intent to Failed+reasons or honest partial Applied; real-session all-refused + partial regressions |
| 4 | Provider-instance `prov-*` credentials → AI job factory | **NOT_IMPLEMENTED** | `start_job` builds the provider from `provider_id` + env/keychain Auto (`rimloc-llm/src/build.rs`); the ProvidersScreen instances (keychain-backed, `contract_provider_instance_*`) are not plumbed into `ai_job_start` — BYO instances cannot yet drive jobs |
| 5 | Durable job records, restart-resume, job-id collision safety | **PARTIAL** | checkpoint resume works at engine level (`.checkpoint.jsonl`); the `{job_id}.job.json` record path is reserved but NOT written (`ai_job.rs` `_record_path`); jobs live in the app process — an app restart loses them and `JOB_SEQ` restarts at 1 (cross-restart id collision is unresolved, Stage C) |
| 6 | Per-row accept/reject UI | **NOT_IMPLEMENTED** | `accept_job` applies every `translated` row as one batch; the AI screen has no per-row accept/reject controls |
| 7 | Real BYO API / local provider UAT | **EXTERNAL_CREDENTIAL_REQUIRED** | journey verified only with the demo echo or a refused/localhost provider; a real OpenAI/Anthropic/Z.AI key or a local Ollama server is required for the production user journey — owner credentials, not code |
| 8 | Official ChatGPT OAuth (Stage C) | **NOT_IMPLEMENTED** | the OAuth spec (PKCE-S256, loopback callback, scopes) is documented in `DESKTOP_FIRST_AI_TRANSLATION`; the frontend «Sign in» button is disabled placeholder; no token exchange code exists |
| 9 | Source Discovery Stage B/C (recovery/audit actions) | **NOT_IMPLEMENTED** | Stage A search shipped (PR #91); B/C stages remain design-only |
| 10 | GUI↔CLI capability parity audit | **PARTIAL** | draft-safety and AI-screen parity tested by lanes; a full feature-by-feature parity matrix (chat batch, compare, selfloc …) is not assembled |
| 11 | Lovable visual acceptance | **OWNER_DECISION_REQUIRED** | source audit shipped earlier; practical visual acceptance session with the Lovable lane awaits the owner |
| 12 | Owner RimWorld UAT | **OWNER_DECISION_REQUIRED** | isolated-profile UAT builds produced; in-game acceptance is the owner's |
| 13 | CodeQL «Tool Status» configuration banner | **NOT_VERIFIED** | CodeQL workflow runs are green on PR heads; the separate repository-level CodeQL configuration banner state has not been checked against docs — TODO as part of the next security sweep |
| 14 | Cancellation boundary beyond the translation loop (review round 2, non-blocking note) | **DOCUMENTED_LIMITATION** | the product guarantee is «no NEW translation requests after Cancel»; the provider factory's optional `/models` preflight (before the engine starts) does not take the cancellation token, and cooperative cancellation never aborts an HTTP request already in flight — a provider may still account for that one request |

Honesty note: `IMPLEMENTED` above means deterministic tests prove the behavior
on the recorded SHA; it does NOT mean the production user journey on a real
provider has run (see #7). AI Translation must not be declared complete while
#7 stays unexecuted.
