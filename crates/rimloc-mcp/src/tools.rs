//! Tool registry + handlers: the declared Phase 1 v1 surface
//! (`docs/development/MCP_SERVER_PHASE1.md`, таблица «Поверхность
//! инструментов v1»).
//!
//! BOTH layers are live. Each handler is a thin pass-through into
//! `rimloc-services` — the same pattern as the GUI shell adapter
//! (`gui/tauri-app/src-tauri/src/contract_adapter.rs`): open the session,
//! call ONE typed manager/service method, surface the result. No second
//! translation/export engine exists here. Every answer is
//! `structuredContent` + a short text summary; bounds are explicit and
//! truncation is flagged in the payload, never silent.
//!
//! Stateless sessions: handlers that need trusted project state build a
//! fresh `ProjectSessionManager` PER CALL, `open` the project, run the
//! operation and drop the manager — no cached sessions between requests
//! (the services layer has no session-close API, so dropping the manager
//! is closing the session). A caller-supplied `session_epoch` is optional
//! on reads and REQUIRED on every controlled write; a mismatching epoch is
//! refused `stale_epoch` — the same wire discipline as the GUI contract.
//! The one piece of process state is the AI-job registry owned by the
//! runtime (jobs live in the app process, per the services contract); the
//! durable `<job_id>.job.json` records are the restart-honest fallback.
//!
//! The forbidden surface stays out by construction: no `run_shell`, no
//! arbitrary file read/write (the one bounded source-file window enforces
//! allow-direction containment into the session's source root), no
//! keychain/secret access (ai_job.start takes a provider-instance ID only;
//! the key never rides the wire in either direction), no network listener,
//! no source-tree writes (export.build writes ONLY the caller's explicit
//! absolute out dir, which the service guards against the source tree and
//! the managed root), no apply without pinned epoch/revision, and no
//! generic "execute any contract method" — the registry is a fixed table,
//! dispatch is by exact name match only.
//!
//! Service mapping:
//!   rimloc.status             → local runtime state (mode, lock, versions)
//!   rimloc.capabilities       → rimloc_services::contract::capability_report
//!   rimloc.projects.list      → ProjectSessionManager::list_report
//!                               (lock-free disk scan in other_writer mode)
//!   rimloc.project.snapshot   → ProjectSessionManager::open + bounded mirror
//!   rimloc.entries.search     → rimloc_services::source_search::search_sources
//!   rimloc.entry.context      → bounded source-file window (the
//!                               `project_source_xml_preview` semantics of
//!                               the GUI adapter rel27; no rimloc-services
//!                               method exists — containment + caps live here)
//!   rimloc.validation.run     → ProjectSessionManager::validate_project
//!   rimloc.export.preview     → ProjectSessionManager::project_output_xml_preview
//!   rimloc.jobs.status        → rimloc_services::ai_job::job_status_with_record
//!                               (durable-record read in other_writer mode)
//!   rimloc.project.open       → ProjectSessionManager::open          (write layer)
//!   rimloc.translation.propose→ ProjectSessionManager::save_drafts_session
//!                               (durable DRAFT sidecar, merge semantics,
//!                               never an apply)                      (write layer)
//!   rimloc.translation.apply  → ProjectSessionManager::apply          (write layer)
//!   rimloc.ai_job.start       → rimloc_services::ai_job::start_job    (write layer)
//!   rimloc.ai_job.status      → ai_job::job_status_with_record        (write layer)
//!   rimloc.ai_job.cancel      → rimloc_services::ai_job::cancel_job   (write layer)
//!   rimloc.ai_job.accept_selected → ai_job::accept_selected with the
//!                               EXPLICIT rejected ledger             (write layer)
//!   rimloc.export.build       → ProjectSessionManager::export_project
//!                               (guarded explicit out dir)           (write layer)

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use rimloc_services::contract::{ContractError, ContractErrorCode, ProjectSnapshot};
use rimloc_services::source_search::{search_sources, SearchScope, SourceSearchRequest};

use crate::error::RpcError;
use crate::protocol::{ToolAnnotations, ToolDefinition, ToolsListResult};
use crate::runtime::{self, Runtime};

/// One input argument of a tool, flattened into the generated JSON Schema.
pub struct Arg {
    pub name: &'static str,
    /// JSON Schema type keyword ("string", "integer", "array").
    pub ty: &'static str,
    pub required: bool,
    pub description: &'static str,
}

/// A registered tool: identity, honest annotations, and the argument
/// table used to generate `inputSchema`.
pub struct ToolDef {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub read_only: bool,
    pub destructive: bool,
    pub idempotent: bool,
    /// True only where the tool can reach the outside world (paid
    /// translation providers).
    pub open_world: bool,
    pub args: &'static [Arg],
}

const EPOCH_ARG_DESC: &str = "Optional stale-session guard: the stateless server opens a \
                              fresh session per call; a provided epoch that does not match \
                              is refused (stale_epoch)";

/// The nine wired read-only tools (the spec's read-only layer, in table
/// order). `rimloc.ai_job.status` sits in the write list below (it is
/// annotated read-only and requires no lock in other_writer mode), so this
/// list stays exactly the lock-free + session-read layer.
pub static WIRED_READ_ONLY: &[&str] = &[
    "rimloc.status",
    "rimloc.capabilities",
    "rimloc.projects.list",
    "rimloc.project.snapshot",
    "rimloc.entries.search",
    "rimloc.entry.context",
    "rimloc.validation.run",
    "rimloc.export.preview",
    "rimloc.jobs.status",
];

/// The seven wired controlled-write tools that actually mutate state (the
/// spec's second layer, minus the read-only job-status alias). Every one
/// goes through `require_writer` — in the degraded modes they refuse typed
/// (`other_writer_active` under a contended lock, `profile_shared` on the
/// default/refused profile).
pub static WIRED_WRITES: &[&str] = &[
    "rimloc.project.open",
    "rimloc.translation.propose",
    "rimloc.translation.apply",
    "rimloc.ai_job.start",
    "rimloc.ai_job.cancel",
    "rimloc.ai_job.accept_selected",
    "rimloc.export.build",
];

/// The spec's write-layer group also names `rimloc.ai_job.status`; it is a
/// READ-ONLY alias of `rimloc.jobs.status` (annotated and executed as a
/// read, lock-free in the degraded modes too) — tracked separately so the
/// executable-counts stay honest.
pub static WIRED_READ_ONLY_ALIASES: &[&str] = &["rimloc.ai_job.status"];

/// The full declared v1 surface — read-only layer first, controlled
/// writes second (spec order). Read-only handlers are live; the write
/// layer answers a typed `not_implemented` until its wiring ask lands.
pub static REGISTRY: &[ToolDef] = &[
    // ---- read-only layer ---------------------------------------------------
    ToolDef {
        name: "rimloc.status",
        title: "RimLoc status",
        description: "Protocol + schema versions, headless/other_writer/shared_profile mode, \
                      lock owner. Source: local runtime state, no service call. \
                      In the degraded modes (other_writer OR shared_profile — the default \
                      GUI-shared or refused profile) write tools and project-scoped reads \
                      refuse; lock-free disk diagnostics (projects.list, jobs.status) stay up.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[],
    },
    ToolDef {
        name: "rimloc.capabilities",
        title: "RimLoc capabilities",
        description: "Supported vs unsupported slice operations with reasons. \
                      Source: rimloc_services::contract::capability_report (pure).",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[],
    },
    ToolDef {
        name: "rimloc.projects.list",
        title: "List managed projects",
        description: "Managed projects with unloadable diagnostics. \
                      Source: ProjectSessionManager::list_report; in the degraded modes \
                      (other_writer OR shared_profile) a lock-free disk scan of the atomic \
                      managed envelopes answers instead.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[],
    },
    ToolDef {
        name: "rimloc.project.snapshot",
        title: "Project snapshot",
        description: "Read-only bounded mirror of the open project state: session \
                      coordinates (revision/epoch/dirty/acked), context, counts, and the \
                      first 200 entries (entries_truncated flags the cut; full inventory \
                      via entries.search). A fresh session is opened per call.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: false,
                description: EPOCH_ARG_DESC,
            },
        ],
    },
    ToolDef {
        name: "rimloc.entries.search",
        title: "Search source entries",
        description: "Read-only full-project source search reconciled against the canonical \
                      inventory. Source: rimloc_services::source_search::search_sources over \
                      the session's source root. Paginated: offset/max_results, page size \
                      defaults to 200 and is hard-capped at HARD_MAX_RESULTS=1000; \
                      `truncated` + exact total_matches flag the cut.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: false,
                description: EPOCH_ARG_DESC,
            },
            Arg {
                name: "query",
                ty: "string",
                required: true,
                description: "Source text to search for (case-insensitive, min 2 chars)",
            },
            Arg {
                name: "scope",
                ty: "string",
                required: false,
                description: "\"standard\" (default) or \"all_text\"",
            },
            Arg {
                name: "offset",
                ty: "integer",
                required: false,
                description: "Pagination offset (default 0)",
            },
            Arg {
                name: "max_results",
                ty: "integer",
                required: false,
                description: "Page size, hard-capped at HARD_MAX_RESULTS=1000",
            },
        ],
    },
    ToolDef {
        name: "rimloc.entry.context",
        title: "Source file context",
        description: "Read-only bounded window of a project source file (allow-direction \
                      containment into the session's source root: absolute paths, `..` and \
                      unresolvable paths refuse; 2 MiB read cap; binary files refuse; \
                      before/after context capped at 200 lines each; truncated_head/tail \
                      flagged). Source: the project_source_xml_preview semantics of the GUI \
                      adapter — no rimloc-services method exists, the bounds live here.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: false,
                description: EPOCH_ARG_DESC,
            },
            Arg {
                name: "file",
                ty: "string",
                required: true,
                description: "Source-root-relative file path",
            },
            Arg {
                name: "line",
                ty: "integer",
                required: false,
                description: "Target line, 1-based (default 1)",
            },
            Arg {
                name: "before",
                ty: "integer",
                required: false,
                description: "Context lines before the target (capped at 200)",
            },
            Arg {
                name: "after",
                ty: "integer",
                required: false,
                description: "Context lines after the target (capped at 200)",
            },
        ],
    },
    ToolDef {
        name: "rimloc.validation.run",
        title: "Run validation",
        description: "The existing validator over trusted session state; read-only, never \
                      mutates the project. Source: ProjectSessionManager::validate_project. \
                      `status` is \"failed\" only when error-severity findings exist.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: false,
                description: EPOCH_ARG_DESC,
            },
            Arg {
                name: "locale",
                ty: "string",
                required: false,
                description: "In-memory locale filter",
            },
        ],
    },
    ToolDef {
        name: "rimloc.export.preview",
        title: "Preview export XML",
        description: "Canonical serializer rendered in-memory into a scratch dir (MAX_FILES=60, \
                      384KB per file); the `truncated` flag surfaces the cut. \
                      Source: ProjectSessionManager::project_output_xml_preview.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: false,
                description: EPOCH_ARG_DESC,
            },
            Arg {
                name: "locale",
                ty: "string",
                required: true,
                description: "Target locale to preview (RimWorld folder form, e.g. \"Russian\")",
            },
        ],
    },
    ToolDef {
        name: "rimloc.jobs.status",
        title: "AI job status",
        description: "Bounded job status; restored Interrupted jobs answer with their last \
                      persisted truth. Source: rimloc_services::ai_job::job_status_with_record; \
                      in other_writer mode the durable record answers directly (lock-free).",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[Arg {
            name: "job_id",
            ty: "string",
            required: true,
            description: "AI translation job id",
        }],
    },
    // ---- controlled writes (second layer; every handler goes through
    //      require_writer + the services' own epoch/revision guards) -------
    ToolDef {
        name: "rimloc.project.open",
        title: "Open project session",
        description: "Open (or re-open) a managed project session and return its WRITE \
                      coordinates: revision, session_epoch, dirty, acked_revision, \
                      source_changed — pass them back to every controlled write. A fresh \
                      open supersedes earlier epochs (stale_session discipline). \
                      Source: ProjectSessionManager::open.",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        args: &[Arg {
            name: "project_id",
            ty: "string",
            required: true,
            description: "Managed project id (proj-*)",
        }],
    },
    ToolDef {
        name: "rimloc.translation.propose",
        title: "Propose translations (draft)",
        description: "Store translations as DURABLE DRAFTS — never an apply: no revision \
                      bump, no provenance write, nothing becomes a committed translation. \
                      Source: ProjectSessionManager::save_drafts_session (sidecar \
                      `<project_id>.drafts.json`, atomic, MERGE semantics — entries not \
                      mentioned by this call are never dropped). Draft key convention: \
                      `<locale>::<display_identity()>` (e.g. \
                      `Russian::keyed·Fixture_Greet`), so parallel locales keep parallel \
                      slices.",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: true,
                description: "Epoch of the caller's open session (stale_epoch refused)",
            },
            Arg {
                name: "locale",
                ty: "string",
                required: true,
                description: "Target locale prefix of the draft keys (RimWorld folder form, \
                              e.g. \"Russian\")",
            },
            Arg {
                name: "entries",
                ty: "array",
                required: true,
                description: "Draft rows as {identity, text} objects; identity is the FULL \
                              structural identity {kind, key, def_type?} (kind: keyed | \
                              def_injected | t_key | strings | backstories | patch_derived)",
            },
        ],
    },
    ToolDef {
        name: "rimloc.translation.apply",
        title: "Apply translations",
        description: "Apply EXPLICIT identities with lost-update protection: session_epoch + \
                      expected_revision are mandatory; a stale epoch/revision is a typed \
                      refusal (stale_epoch / stale_revision), never a blind overwrite — the \
                      service resolves every identity against the trusted inventory and \
                      refuses unknown/non-translatable ones per intent (skipped[] carries \
                      the reasons; the rest of the batch still applies). Provenance is \
                      stamped by the service. \
                      Source: ProjectSessionManager::apply over the typed \
                      rimloc_services::contract::ApplyIntentsRequest.",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: true,
                description: "Epoch of the caller's open session (stale_epoch refused)",
            },
            Arg {
                name: "expected_revision",
                ty: "integer",
                required: true,
                description: "Revision the caller based its edits on (stale_revision refused)",
            },
            Arg {
                name: "intents",
                ty: "array",
                required: true,
                description: "Typed intents {entry: {kind, key, def_type?}, locale, action, \
                              text?} — action: set_translation | mark_todo | clear_translation \
                              (rimloc_services::contract::TranslationIntent)",
            },
            Arg {
                name: "origin",
                ty: "string",
                required: false,
                description: "Provenance claim: human (default) | llm | tm | import",
            },
        ],
    },
    ToolDef {
        name: "rimloc.ai_job.start",
        title: "Start AI translation job",
        description: "Start an AI translation job for a target locale. The launch target is \
                      defined SOLELY by an existing provider instance (prov-*): its preset, \
                      model, base_url and keychain key — the server never requests, stores \
                      or returns secrets; only identifiers cross the wire. \
                      PAID (non-local) providers are disabled via the external MCP until an \
                      explicit per-run consent mechanism exists: a non-local instance \
                      refuses typed `paid_providers_disabled_via_mcp` BEFORE any secret \
                      resolution or worker spawn; local providers (ollama/loopback) launch. \
                      Source: rimloc_services::ai_job::start_job (worker thread; poll \
                      rimloc.ai_job.status; the durable record outlives the process).",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: true,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: true,
                description: "Epoch of the caller's open session (stale_epoch refused \
                              BEFORE any paid work)",
            },
            Arg {
                name: "locale",
                ty: "string",
                required: true,
                description: "Target locale (RimWorld folder contract, e.g. \"Russian\")",
            },
            Arg {
                name: "provider_instance_id",
                ty: "string",
                required: true,
                description: "Existing providers-screen instance (prov-*): unknown instance \
                              or any non-local (paid) instance refuses typed; the stored \
                              key never rides the wire",
            },
            Arg {
                name: "batch_char_budget",
                ty: "integer",
                required: false,
                description: "Batch character budget (default per services: 6000)",
            },
        ],
    },
    ToolDef {
        name: "rimloc.ai_job.status",
        title: "AI job status (alias of rimloc.jobs.status)",
        description: "Job status with the durable record; restored Interrupted jobs are \
                      visible. Identical handler to rimloc.jobs.status — the in-process \
                      registry answers first, the durable record is the restart-honest \
                      fallback. \
                      Source: rimloc_services::ai_job::job_status_with_record.",
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
        args: &[Arg {
            name: "job_id",
            ty: "string",
            required: true,
            description: "AI translation job id",
        }],
    },
    ToolDef {
        name: "rimloc.ai_job.cancel",
        title: "Cancel AI job",
        description: "Flag a running job for cancellation at its next batch/request/retry \
                      boundary — the same flag the translation engine observes; a late \
                      worker completion can never resurrect a cancelled job. \
                      Source: rimloc_services::ai_job::cancel_job (unknown job: typed \
                      refusal; jobs live in the app process).",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        args: &[Arg {
            name: "job_id",
            ty: "string",
            required: true,
            description: "AI translation job id",
        }],
    },
    ToolDef {
        name: "rimloc.ai_job.accept_selected",
        title: "Accept selected job rows",
        description: "Per-row review: apply ONLY the identities in `selected` (empty \
                      selection is a typed refusal); `rejected` is the EXPLICIT refusal \
                      ledger — unselected-but-translated rows stay reviewable, never \
                      silently rejected. The pinned base revision and stale-epoch guards \
                      refuse the whole operation when the project moved on (manual edits \
                      during inference are never overwritten); the preview ledger (bounded \
                      at PREVIEW_ROW_LIMIT=400 rows) applies a selection regardless of \
                      size, but the FULL preview is only applicable with an explicit \
                      full-scope acknowledgement (rimloc.ai_job semantics, \
                      acknowledge_full_scope). \
                      Source: rimloc_services::ai_job::accept_selected.",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        args: &[
            Arg {
                name: "job_id",
                ty: "string",
                required: true,
                description: "AI translation job id (must be a job of THIS server process — \
                              a restart-interrupted record is visible via status but not \
                              acceptable)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: true,
                description: "Epoch of the caller's open session (stale_epoch refused)",
            },
            Arg {
                name: "selected",
                ty: "array",
                required: true,
                description: "Row identity strings to apply (the `identity` field of preview \
                              rows, e.g. `keyed·Fixture_Greet`); MUST be non-empty",
            },
            Arg {
                name: "rejected",
                ty: "array",
                required: false,
                description: "Row identity strings to mark rejected in the ledger (default \
                              [])",
            },
        ],
    },
    ToolDef {
        name: "rimloc.export.build",
        title: "Build export",
        description: "Guarded export of the session state into an EXPLICIT caller-owned \
                      output directory: absolute path REQUIRED (relative is a typed refusal \
                      before any filesystem access), never the read-only source mod tree \
                      and never the managed root (both are service-guard denies), \
                      reparse + strict-XML verified BEFORE the ack. \
                      Source: ProjectSessionManager::export_project.",
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
        args: &[
            Arg {
                name: "project_id",
                ty: "string",
                required: true,
                description: "Managed project id (proj-*)",
            },
            Arg {
                name: "session_epoch",
                ty: "integer",
                required: true,
                description: "Epoch of the caller's open session (stale_epoch refused)",
            },
            Arg {
                name: "out_dir",
                ty: "string",
                required: true,
                description: "ABSOLUTE caller-owned output directory (inside the source mod \
                              tree or the managed root refuses: guard_output_denied)",
            },
            Arg {
                name: "locale",
                ty: "string",
                required: true,
                description: "Target locale to export (RimWorld folder form, e.g. \"Russian\")",
            },
        ],
    },
];

impl ToolDef {
    /// JSON Schema (draft 2020-12) for the tool input, generated from the
    /// argument table — no hand-written schema drift.
    fn input_schema(&self) -> Value {
        let mut properties = serde_json::Map::new();
        let mut required: Vec<Value> = Vec::new();
        for arg in self.args {
            properties.insert(
                arg.name.to_string(),
                json!({ "type": arg.ty, "description": arg.description }),
            );
            if arg.required {
                required.push(Value::from(arg.name));
            }
        }
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": required,
            "additionalProperties": true,
        })
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.to_string(),
            title: self.title.to_string(),
            description: self.description.to_string(),
            input_schema: self.input_schema(),
            annotations: ToolAnnotations {
                title: self.title.to_string(),
                read_only_hint: self.read_only,
                destructive_hint: self.destructive,
                idempotent_hint: self.idempotent,
                open_world_hint: self.open_world,
            },
        }
    }
}

/// `tools/list` payload: the whole registry on one page (no pagination in
/// Phase 1 — `nextCursor` is absent rather than faked).
pub fn tools_list_result() -> Value {
    let tools: Vec<ToolDefinition> = REGISTRY.iter().map(ToolDef::definition).collect();
    match serde_json::to_value(ToolsListResult { tools }) {
        Ok(value) => value,
        // Unreachable for these plain shapes; a runtime path degrades to a
        // surfaced error instead of a panic.
        Err(err) => json!({ "tools": [], "serialization_error": err.to_string() }),
    }
}

// ---------------------------------------------------------------------------
// Argument shapes (wire → typed; serde refusals are typed invalid_params)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ProjectRefArgs {
    project_id: String,
    #[serde(default)]
    session_epoch: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SearchArgs {
    project_id: String,
    #[serde(default)]
    session_epoch: Option<u64>,
    query: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ContextArgs {
    project_id: String,
    #[serde(default)]
    session_epoch: Option<u64>,
    file: String,
    #[serde(default)]
    line: Option<usize>,
    #[serde(default)]
    before: Option<usize>,
    #[serde(default)]
    after: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ValidateArgs {
    project_id: String,
    #[serde(default)]
    session_epoch: Option<u64>,
    #[serde(default)]
    locale: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PreviewArgs {
    project_id: String,
    #[serde(default)]
    session_epoch: Option<u64>,
    locale: String,
}

#[derive(Debug, Deserialize)]
struct JobStatusArgs {
    job_id: String,
}

// -- controlled-write layer argument shapes --------------------------------

/// `rimloc.project.open` — no epoch argument: open MINTS the fresh epoch
/// the caller then carries.
#[derive(Debug, Deserialize)]
struct ProjectOpenArgs {
    project_id: String,
}

/// One draft row of `rimloc.translation.propose`. The identity is the FULL
/// structural identity (the service resolves eligibility and provenance;
/// the display form is derived HERE for the durable draft key, never
/// trusted from the client).
#[derive(Debug, Deserialize)]
struct ProposeEntryArgs {
    identity: rimloc_domain::canonical::SourceEntryId,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ProposeArgs {
    project_id: String,
    session_epoch: u64,
    locale: String,
    entries: Vec<ProposeEntryArgs>,
}

#[derive(Debug, Deserialize)]
struct ApplyArgs {
    project_id: String,
    session_epoch: u64,
    expected_revision: u64,
    intents: Vec<rimloc_services::contract::TranslationIntent>,
    #[serde(default)]
    origin: Option<rimloc_services::contract::ApplyOrigin>,
}

#[derive(Debug, Deserialize)]
struct AiJobStartArgs {
    project_id: String,
    session_epoch: u64,
    locale: String,
    provider_instance_id: String,
    #[serde(default)]
    batch_char_budget: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AcceptSelectedArgs {
    job_id: String,
    session_epoch: u64,
    selected: Vec<String>,
    #[serde(default)]
    rejected: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExportBuildArgs {
    project_id: String,
    session_epoch: u64,
    out_dir: String,
    locale: String,
}

// ---------------------------------------------------------------------------
// Shared session seam (stateless: open per call, drop after)
// ---------------------------------------------------------------------------

/// Open a FRESH project session for one handler call and verify the
/// optional caller epoch. The manager is returned so the caller can run
/// further manager methods; dropping it closes the session.
fn open_session(
    rt: &Runtime,
    tool: &str,
    args: &ProjectRefArgs,
) -> Result<
    (
        rimloc_services::session::ProjectSessionManager,
        ProjectSnapshot,
    ),
    RpcError,
> {
    runtime::require_writer(rt, tool)?;
    let manager = runtime::new_manager(rt, tool)?;
    let snap = manager
        .open(&args.project_id)
        .map_err(|err| runtime::contract_error(err, tool))?;
    if let Some(want) = args.session_epoch {
        if want != snap.session_epoch {
            return Err(runtime::contract_error(
                ContractError::stale_epoch(want, snap.session_epoch),
                tool,
            ));
        }
    }
    Ok((manager, snap))
}

fn source_root_of(snap: &ProjectSnapshot, tool: &str) -> Result<PathBuf, RpcError> {
    snap.source_root
        .as_ref()
        .map(|r| PathBuf::from(&r.path))
        .ok_or_else(|| {
            runtime::contract_error(
                ContractError::new(
                    ContractErrorCode::InvalidConfig,
                    "project has no source root — the source mod is unavailable",
                ),
                tool,
            )
        })
}

/// Allow-direction containment (mirrors the services `is_within_allow`
/// semantics; the util module is private to rimloc-services): containment
/// must be PROVEN on the canonical (symlink-resolved) view — an
/// unresolvable path refuses instead of guessing.
fn is_within_allow(candidate: &Path, root: &Path) -> bool {
    match (candidate.canonicalize(), root.canonicalize()) {
        (Ok(c), Ok(r)) => c.starts_with(&r),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn status(rt: &Runtime) -> Result<Value, RpcError> {
    let report = rimloc_services::contract::capability_report();
    let (app, owner_pid, held_by_us) = match rt.mode() {
        runtime::AppMode::HeadlessWriter => ("self", Some(std::process::id()), true),
        runtime::AppMode::OtherWriter { owner } => ("other_writer", owner.pid, false),
        // Fail-closed shared profile (default profile / refused
        // RIMLOC_DATA_DIR): no lock is held, no writer exists — reported
        // verbatim (audit п.2 honesty).
        runtime::AppMode::SharedProfile { .. } => ("shared_profile", None, false),
    };
    let profile_source = match std::env::var("RIMLOC_DATA_DIR") {
        Ok(dir) if !dir.is_empty() => "RIMLOC_DATA_DIR",
        _ => "default_app_data",
    };
    // All nine wired read-only tools execute under our own lock; in the
    // degraded modes (other_writer OR shared_profile) the five
    // project-scoped ones refuse (session open would run the manager
    // constructor's recovery write), four disk diagnostics remain — plus
    // the read-only ai_job.status alias, whose durable-record read stays
    // lock-free in EVERY mode. The seven controlled writes ALL need the
    // writer machinery — they refuse in both degraded modes wholesale.
    const PROJECT_SCOPED_READS: usize = 5;
    let read_only_executable = if rt.is_diagnostics_only() {
        WIRED_READ_ONLY.len() - PROJECT_SCOPED_READS + WIRED_READ_ONLY_ALIASES.len()
    } else {
        WIRED_READ_ONLY.len() + WIRED_READ_ONLY_ALIASES.len()
    };
    let writes_executable = if rt.is_diagnostics_only() {
        0
    } else {
        WIRED_WRITES.len()
    };
    let gui_sharing = if rt.is_shared_profile() {
        match rt.shared_profile_reason() {
            Some(runtime::SharedProfileReason::DefaultProfile) => {
                "shared_profile (fail-closed): the default profile belongs to the live \
                 RimLoc GUI and rimloc-mcp.lock does not guard it; writes and \
                 project-scoped reads refuse; set RIMLOC_DATA_DIR to isolate"
                    .to_string()
            }
            Some(runtime::SharedProfileReason::Refused { requested, cause }) => {
                format!(
                    "shared_profile (fail-closed): RIMLOC_DATA_DIR `{requested}` was \
                     refused ({cause}); writes and project-scoped reads refuse; \
                     set RIMLOC_DATA_DIR to isolate"
                )
            }
            None => "shared_profile".to_string(),
        }
    } else if profile_source == "default_app_data" {
        "warning: default profile is shared with the RimLoc GUI; if the GUI is \
         running, stop it or set RIMLOC_DATA_DIR to isolate"
            .to_string()
    } else {
        "isolated".to_string()
    };
    let structured = json!({
        "app": app,
        "mode": "headless",
        "protocol_version": crate::protocol::PROTOCOL_VERSION,
        "supported_protocol_versions": crate::protocol::SUPPORTED_PROTOCOL_VERSIONS,
        "ui_contract_version": report.contract_version,
        "managed_root": rt.managed_root().display().to_string(),
        "profile_source": profile_source,
        "lock": {
            "path": rt.managed_root().join(runtime::LOCK_FILE_NAME).display().to_string(),
            "held_by_us": held_by_us,
            "owner_pid": owner_pid,
            // Honesty about scope: the lock is shared between rimloc-mcp
            // PROCESSES only. A live RimLoc GUI on the same profile does
            // not take this lock and is not detected until the Phase 2
            // broker — isolation from the GUI today is by profile
            // (RIMLOC_DATA_DIR), not by this lock; on the default profile
            // the runtime does not even become the writer (shared_profile).
            "isolation": "mcp_processes_only",
        },
        "gui_sharing_profile": gui_sharing,
        "tools": {
            "declared": REGISTRY.len(),
            "read_only_executable": read_only_executable,
            "writes_executable": writes_executable,
        },
        "project_scoped_reads": if rt.is_shared_profile() {
            "refused_profile_shared"
        } else if rt.is_other_writer() {
            "refused_other_writer"
        } else {
            "available"
        },
    });
    let summary = if rt.is_shared_profile() {
        format!(
            "shared profile (fail-closed): no writer lock on `{}` — it is shared with \
             the live RimLoc GUI or the requested profile was refused; write tools and \
             project-scoped reads refuse (profile_shared); rimloc.status / capabilities / \
             projects.list / jobs.status answer from disk; set RIMLOC_DATA_DIR to isolate",
            rt.managed_root().display()
        )
    } else if rt.is_other_writer() {
        format!(
            "other_writer: the exclusive lock on `{}` is held by another process (pid {:?}); \
             write tools and project-scoped reads refuse; rimloc.status / capabilities / \
             projects.list / jobs.status answer from disk",
            rt.managed_root().display(),
            owner_pid
        )
    } else {
        format!(
            "headless writer: lock held by this process (pid {owner_pid:?}); protocol {}, \
             ui contract v{}; {}/{} tools executable ({} read-only + {} controlled writes)",
            crate::protocol::PROTOCOL_VERSION,
            report.contract_version,
            read_only_executable + writes_executable,
            REGISTRY.len(),
            read_only_executable,
            writes_executable
        )
    };
    runtime::tool_ok(structured, summary)
}

fn capabilities() -> Result<Value, RpcError> {
    let report = rimloc_services::contract::capability_report();
    let structured = serde_json::to_value(&report).map_err(|err| {
        RpcError::internal(format!("capability report serialization failed: {err}"))
    })?;
    let summary = format!(
        "ui contract v{}: {} supported, {} unsupported operation(s)",
        report.contract_version,
        report.supported.len(),
        report.unsupported.len()
    );
    runtime::tool_ok(structured, summary)
}

fn projects_list(rt: &Runtime) -> Result<Value, RpcError> {
    let tool = "rimloc.projects.list";
    let report = if rt.is_other_writer() {
        // Lock-free disk diagnostics: managed envelopes are atomic, so the
        // disk truth is consistent without the writer lock.
        runtime::list_from_disk(rt.managed_root())
    } else {
        runtime::new_manager(rt, tool)?.list_report()
    };
    let summary = format!(
        "{} managed project(s), {} unloadable{}",
        report.projects.len(),
        report.unloadable.len(),
        // The degraded modes get distinct honest reasons: in shared_profile
        // mode NO ONE holds the lock (the GUI never takes it) — claiming
        // "another writer" would be false.
        if rt.is_shared_profile() {
            " (disk-only view: the profile is shared with the live RimLoc \
             GUI — fail-closed diagnostics)"
        } else if rt.is_other_writer() {
            " (disk-only view: another writer holds the lock)"
        } else {
            ""
        }
    );
    let structured = serde_json::to_value(&report)
        .map_err(|err| RpcError::internal(format!("list serialization failed: {err}")))?;
    runtime::tool_ok(structured, summary)
}

fn project_snapshot(rt: &Runtime, args: &ProjectRefArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.project.snapshot";
    let (_manager, snap) = open_session(rt, tool, args)?;
    const SNAPSHOT_ENTRY_LIMIT: usize = 200;
    let total_entries = snap.project.entries.len();
    let entries: Vec<Value> = snap
        .project
        .entries
        .iter()
        .take(SNAPSHOT_ENTRY_LIMIT)
        .map(|e| {
            json!({
                "kind": e.id.kind,
                "key": e.id.key,
                "def_type": e.id.def_type,
                "text": e.text,
            })
        })
        .collect();
    let total_translations = snap.project.translations.len();
    let translated = snap
        .project
        .translations
        .iter()
        .filter(|t| t.text.as_deref().is_some_and(|s| !s.trim().is_empty()))
        .count();
    let structured = json!({
        "project_id": snap.project_id,
        "revision": snap.revision,
        "session_epoch": snap.session_epoch,
        "dirty": snap.dirty,
        "acked_revision": snap.acked_revision,
        "source_changed": snap.source_changed,
        "source_root": snap.source_root,
        "context": snap.project.context,
        "counts": {
            "entries": total_entries,
            "translations": total_translations,
            "translated": translated,
        },
        "entries": entries,
        "entries_truncated": total_entries > SNAPSHOT_ENTRY_LIMIT,
        "entry_sample_limit": SNAPSHOT_ENTRY_LIMIT,
    });
    let summary = format!(
        "project {}: revision {}, epoch {}, {} entries, {}/{} translations non-empty{}",
        snap.project_id,
        snap.revision,
        snap.session_epoch,
        total_entries,
        translated,
        total_translations,
        if total_entries > SNAPSHOT_ENTRY_LIMIT {
            " (entries truncated at 200)"
        } else {
            ""
        }
    );
    runtime::tool_ok(structured, summary)
}

fn entries_search(rt: &Runtime, args: &SearchArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.entries.search";
    let (_manager, snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: args.session_epoch,
        },
    )?;
    let root = source_root_of(&snap, tool)?;
    let scope = match args.scope.as_deref() {
        None | Some("standard") => SearchScope::Standard,
        Some("all_text") => SearchScope::AllText,
        Some(other) => {
            return Err(RpcError::invalid_params(format!(
                "invalid scope `{other}`: expected \"standard\" or \"all_text\""
            ))
            .with_data(json!({ "code": "invalid_arguments", "tool": tool })));
        }
    };
    let request = SourceSearchRequest {
        mod_root: root,
        target_version: snap.project.context.target_version.clone(),
        source_lang_dir: snap.project.context.source_dir.clone(),
        query: args.query.clone(),
        scope,
        offset: args.offset.unwrap_or(0),
        // 0 = "service default" (200), the service clamps at HARD_MAX 1000.
        max_results: args.max_results.unwrap_or(0),
        // Stage C reconciliation, same as the GUI adapter: rows already in
        // the managed state classify as indexed on the next search.
        manual_ids: snap.project.entries.iter().map(|e| e.id.clone()).collect(),
    };
    let report =
        search_sources(&request, &|| false).map_err(|err| runtime::contract_error(err, tool))?;
    let summary = format!(
        "{} match(es) total; page offset {} shows {} result(s); truncated: {}",
        report.total_matches,
        report.offset,
        report.matches.len(),
        report.truncated
    );
    let structured = serde_json::to_value(&report)
        .map_err(|err| RpcError::internal(format!("search report serialization failed: {err}")))?;
    runtime::tool_ok(structured, summary)
}

/// Bounds of the source-file window (GUI adapter `project_source_xml_preview`
/// semantics, plus an explicit read cap so a huge file can never be pulled
/// into memory through the wire).
const CONTEXT_MAX_BEFORE_AFTER: usize = 200;
const CONTEXT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

fn entry_context(rt: &Runtime, args: &ContextArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.entry.context";
    let (_manager, snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: args.session_epoch,
        },
    )?;
    let root = source_root_of(&snap, tool)?;
    let rel = Path::new(&args.file);
    let escapes = rel.is_absolute()
        || rel.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
    if escapes {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root",
            ),
            tool,
        ));
    }
    let full = root.join(rel);
    // The read-target must EXIST before the allow-direction check can be
    // proven (canonical containment): a missing file is an honest
    // invalid_config, not a guard refusal — while a symlink pointing out
    // of the tree DOES resolve and refuses right here, before any read.
    let meta = std::fs::metadata(&full).map_err(|err| {
        runtime::contract_error(
            ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {err}", args.file),
            ),
            tool,
        )
    })?;
    if !is_within_allow(&full, &root) {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root",
            ),
            tool,
        ));
    }
    if meta.len() > CONTEXT_MAX_FILE_BYTES {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "source file is {} bytes; the preview read cap is {CONTEXT_MAX_FILE_BYTES} — \
                     use entries.search to locate the region first",
                    meta.len()
                ),
            ),
            tool,
        ));
    }
    // Read through the canonical path: `is_within_allow` proved containment
    // on the RESOLVED path, so the read must not re-traverse any symlink a
    // local process may have swapped in between the check and this read.
    let canonical = full.canonicalize().map_err(|err| {
        runtime::contract_error(
            ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {err}", args.file),
            ),
            tool,
        )
    })?;
    if !is_within_allow(&canonical, &root) {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root",
            ),
            tool,
        ));
    }
    let bytes = std::fs::read(&canonical).map_err(|err| {
        runtime::contract_error(
            ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {err}", args.file),
            ),
            tool,
        )
    })?;
    // The metadata check above can race a growing file; the wire answer is
    // bounded by what was actually read, not by the pre-check.
    if bytes.len() as u64 > CONTEXT_MAX_FILE_BYTES {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "source file grew past the read cap ({CONTEXT_MAX_FILE_BYTES} bytes) \
                     while opening — retry",
                ),
            ),
            tool,
        ));
    }
    if bytes.iter().take(8192).any(|b| *b == 0) {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::InvalidConfig,
                "source preview: file looks binary",
            ),
            tool,
        ));
    }
    let content = String::from_utf8_lossy(&bytes);
    let total_lines = content.lines().count();
    let target = args.line.unwrap_or(1).max(1).min(total_lines.max(1));
    let before = args.before.unwrap_or(0).min(CONTEXT_MAX_BEFORE_AFTER);
    let after = args.after.unwrap_or(0).min(CONTEXT_MAX_BEFORE_AFTER);
    let start = target.saturating_sub(before).max(1);
    let end = (target + after).min(total_lines.max(1));
    let lines: Vec<Value> = content
        .lines()
        .skip(start - 1)
        .take(end - start + 1)
        .enumerate()
        .map(|(i, text)| json!({ "n": start + i, "text": text }))
        .collect();
    let structured = json!({
        "file": args.file,
        "target_line": target,
        "total_lines": total_lines,
        "truncated_head": start > 1,
        "truncated_tail": end < total_lines,
        "before_cap": CONTEXT_MAX_BEFORE_AFTER,
        "after_cap": CONTEXT_MAX_BEFORE_AFTER,
        "lines": lines,
    });
    let summary = format!(
        "`{}`: lines {}..{} of {} total (target line {target})",
        args.file, start, end, total_lines
    );
    runtime::tool_ok(structured, summary)
}

fn validation_run(rt: &Runtime, args: &ValidateArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.validation.run";
    let (manager, snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: args.session_epoch,
        },
    )?;
    let resp = manager
        .validate_project(&args.project_id, snap.session_epoch, args.locale.as_deref())
        .map_err(|err| runtime::contract_error(err, tool))?;
    // Bounded surface: findings grow with the project, not with the
    // request, so the wire answer carries a capped window plus the true
    // totals — every other collection on this surface is bounded the same
    // way (snapshot 200, search 1000, preview 60×384KB, job rows 400).
    const MAX_FINDINGS_ON_WIRE: usize = 200;
    let findings_total = resp.findings.len();
    let findings_truncated = findings_total > MAX_FINDINGS_ON_WIRE;
    let summary = format!(
        "validation {}: {} error(s), {} warning(s), {} info finding(s){}{}",
        resp.status,
        resp.error_count,
        resp.warning_count,
        resp.info_count,
        args.locale
            .as_deref()
            .map(|l| format!(" for locale `{l}`"))
            .unwrap_or_default(),
        if findings_truncated {
            format!(
                "; findings capped at {MAX_FINDINGS_ON_WIRE} of {findings_total} — \
                 narrow with a locale filter or the GUI for the full list"
            )
        } else {
            String::new()
        }
    );
    let mut structured = serde_json::to_value(&resp)
        .map_err(|err| RpcError::internal(format!("validation serialization failed: {err}")))?;
    if let Some(obj) = structured.as_object_mut() {
        if findings_truncated {
            if let Some(findings) = obj.get_mut("findings").and_then(|f| f.as_array_mut()) {
                findings.truncate(MAX_FINDINGS_ON_WIRE);
            }
        }
        obj.insert(
            "findings_total".to_string(),
            serde_json::json!(findings_total),
        );
        obj.insert(
            "findings_truncated".to_string(),
            serde_json::json!(findings_truncated),
        );
    }
    runtime::tool_ok(structured, summary)
}

fn export_preview(rt: &Runtime, args: &PreviewArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.export.preview";
    let (manager, snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: args.session_epoch,
        },
    )?;
    let resp = manager
        .project_output_xml_preview(&args.project_id, snap.session_epoch, &args.locale)
        .map_err(|err| runtime::contract_error(err, tool))?;
    let truncated = resp.truncated;
    let file_count = resp.files.len();
    let revision = resp.revision;
    let summary = format!(
        "export preview for `{}`: {} file(s) at revision {revision}, truncated: {truncated}",
        args.locale, file_count
    );
    let structured = serde_json::to_value(&resp)
        .map_err(|err| RpcError::internal(format!("preview serialization failed: {err}")))?;
    runtime::tool_ok(structured, summary)
}

/// Job ids become file names under the managed root
/// (`<job_id>.job.json`): refuse anything that is not a single safe
/// component BEFORE any filesystem access.
fn job_id_is_safe(job_id: &str) -> bool {
    !job_id.is_empty()
        && !job_id.starts_with('.')
        && !job_id.contains(['/', '\\'])
        && !job_id.contains("..")
}

fn jobs_status(rt: &Runtime, tool: &str, args: &JobStatusArgs) -> Result<Value, RpcError> {
    if !job_id_is_safe(&args.job_id) {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::ContractViolation,
                "malformed job id: expected a single path-safe component",
            ),
            tool,
        ));
    }
    let status = if rt.is_other_writer() {
        // Lock-free: read the durable record straight from disk.
        rimloc_services::ai_job::job_status_from_record(rt.managed_root(), &args.job_id)
            .ok_or_else(|| {
                runtime::contract_error(
                    ContractError::new(
                        ContractErrorCode::ProjectNotFound,
                        format!("ai job `{}` has no durable record", args.job_id),
                    ),
                    tool,
                )
            })?
    } else {
        // The runtime's process-owned registry first (a job started by THIS
        // server answers with its live state), then the durable record.
        rimloc_services::ai_job::job_status_with_record(
            rt.ai_jobs(),
            rt.managed_root(),
            &args.job_id,
        )
        .map_err(|err| runtime::contract_error(err, tool))?
    };
    let structured = serde_json::to_value(&status)
        .map_err(|err| RpcError::internal(format!("job status serialization failed: {err}")))?;
    let state = structured["state"].as_str().unwrap_or("unknown");
    let summary = format!(
        "job {}: state {state}, translated {}/{}",
        status.job_id, status.translated, status.total
    );
    runtime::tool_ok(structured, summary)
}

// ---------------------------------------------------------------------------
// Controlled-write handlers (second layer)
// ---------------------------------------------------------------------------

/// Wall-clock milliseconds for draft stamps (the same shape the services
/// `now_ms` produces; that helper is crate-private to rimloc-services).
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn project_open(rt: &Runtime, args: &ProjectOpenArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.project.open";
    runtime::require_writer(rt, tool)?;
    let manager = runtime::new_manager(rt, tool)?;
    let snap = manager
        .open(&args.project_id)
        .map_err(|err| runtime::contract_error(err, tool))?;
    let total_entries = snap.project.entries.len();
    let total_translations = snap.project.translations.len();
    let structured = json!({
        "project_id": snap.project_id,
        "revision": snap.revision,
        "session_epoch": snap.session_epoch,
        "dirty": snap.dirty,
        "acked_revision": snap.acked_revision,
        "source_changed": snap.source_changed,
        "counts": {
            "entries": total_entries,
            "translations": total_translations,
        },
    });
    let summary = format!(
        "project {} open: revision {}, epoch {} (carry these into the write tools), \
         {} entries, dirty: {}",
        snap.project_id, snap.revision, snap.session_epoch, total_entries, snap.dirty
    );
    runtime::tool_ok(structured, summary)
}

fn translation_propose(rt: &Runtime, args: &ProposeArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.translation.propose";
    if args.entries.is_empty() {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::ContractViolation,
                "translation.propose received zero entries — a proposal must name at least \
                 one identity",
            ),
            tool,
        ));
    }
    let (manager, _snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: Some(args.session_epoch),
        },
    )?;
    // Draft keys are `<locale>::<display_identity()>` — the UI convention,
    // derived HERE from the typed identity (the client never supplies the
    // key string). The store merges: entries this call does not mention
    // survive untouched.
    let mut entries = std::collections::BTreeMap::new();
    for row in &args.entries {
        let key = format!("{}::{}", args.locale, row.identity.display_identity());
        entries.insert(
            key,
            rimloc_services::DraftEntry::new(row.text.clone(), now_ms()),
        );
    }
    let proposed = args.entries.len();
    let ack = manager
        .save_drafts_session(&args.project_id, args.session_epoch, entries)
        .map_err(|err| runtime::contract_error(err, tool))?;
    let structured = json!({
        "project_id": args.project_id,
        "proposed": proposed,
        "saved_total": ack.saved,
        "project_revision": ack.project_revision,
        "updated_at": ack.updated_at,
        "committed": false,
        "note": "drafts only — nothing became a committed translation; apply explicitly",
    });
    let summary = format!(
        "proposed {proposed} draft(s) for `{}` at revision {} (store now holds {}); \
         NOT applied",
        args.project_id, ack.project_revision, ack.saved
    );
    runtime::tool_ok(structured, summary)
}

fn translation_apply(rt: &Runtime, args: &ApplyArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.translation.apply";
    let (manager, _snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: Some(args.session_epoch),
        },
    )?;
    let req = rimloc_services::contract::ApplyIntentsRequest {
        project_id: args.project_id.clone(),
        expected_revision: args.expected_revision,
        session_epoch: args.session_epoch,
        intents: args.intents.clone(),
        origin: args.origin,
    };
    let resp = manager
        .apply(&req)
        .map_err(|err| runtime::contract_error(err, tool))?;
    let skipped = resp.skipped.len();
    let structured = serde_json::to_value(&resp)
        .map_err(|err| RpcError::internal(format!("apply serialization failed: {err}")))?;
    let summary = format!(
        "apply {}: {} applied, {skipped} skipped, revision {}{}",
        resp.job_id,
        resp.applied,
        resp.revision,
        if resp.cancelled { ", CANCELLED" } else { "" }
    );
    runtime::tool_ok(structured, summary)
}

fn ai_job_start(rt: &Runtime, args: &AiJobStartArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.ai_job.start";
    let (manager, snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: Some(args.session_epoch),
        },
    )?;
    // PAID GUARD (audit 2026-10, fail-closed): the external MCP surface
    // must not launch paid (non-local) providers until an explicit
    // per-run consent mechanism exists. The `local` flag is read from the
    // REDACTED instance metadata (`ProviderInstanceSummary` carries no
    // secret field) through the SAME manager instance `start_job` below
    // resolves against, so the guard and the launch see one consistent
    // state with no TOCTOU window. The refusal fires BEFORE `start_job` —
    // i.e. before any secret resolution or worker spawn — and in this
    // sink-less process the metadata list cannot touch the OS keychain
    // anyway (`has_key` falls back to the stored presence cache). An
    // unknown instance is not this guard's case: `start_job` refuses it
    // with its own typed error before any spawn. An unreadable settings
    // file propagates as a typed failure, so corruption can never look
    // like "nothing to check" (fail-closed). Ordering note: the stale-
    // epoch guard in `open_session` stays FIRST on purpose — the services
    // document that an epoch problem outranks a provider problem.
    let instances = manager
        .provider_instance_list()
        .map_err(|err| runtime::contract_error(err, tool))?;
    if let Some(instance) = instances
        .instances
        .iter()
        .find(|i| i.id == args.provider_instance_id)
    {
        if !instance.local {
            return Err(RpcError::new(
                crate::error::codes::CONTRACT_ERROR,
                format!(
                    "provider instance `{}` ({} {}) is not local: paid providers are \
                     disabled via the external MCP until an explicit per-run consent \
                     mechanism exists; local providers (ollama/mock/loopback) remain \
                     available",
                    instance.id, instance.preset, instance.model
                ),
            )
            .with_data(json!({
                "code": "paid_providers_disabled_via_mcp",
                "tool": tool,
                "details": {
                    "provider_instance_id": instance.id,
                    "preset": instance.preset,
                    "local": instance.local,
                },
            })));
        }
    }
    // The provider instance FULLY defines the launch target (preset →
    // provider id, model, base_url and the keychain key); `provider_id` is
    // a placeholder the service overwrites during instance resolution.
    // Secrets are resolved in-process from the OS keychain and never cross
    // the wire in either direction.
    let params = rimloc_services::ai_job::AiJobParams {
        project_id: args.project_id.clone(),
        session_epoch: args.session_epoch,
        locale: args.locale.clone(),
        source_locale: snap
            .project
            .context
            .source_locale
            .clone()
            .unwrap_or_default(),
        provider_id: String::new(),
        model: None,
        base_url: None,
        provider_instance_id: Some(args.provider_instance_id.clone()),
        batch_char_budget: args.batch_char_budget.unwrap_or(0),
    };
    let status =
        rimloc_services::ai_job::start_job(rt.ai_jobs(), &manager, rt.managed_root(), params)
            .map_err(|err| runtime::contract_error(err, tool))?;
    let summary = format!(
        "job {} started: project `{}`, locale `{}`, {} untranslated row(s); poll \
         rimloc.ai_job.status for the worker state",
        status.job_id, args.project_id, args.locale, status.total
    );
    let structured = serde_json::to_value(&status)
        .map_err(|err| RpcError::internal(format!("job status serialization failed: {err}")))?;
    runtime::tool_ok(structured, summary)
}

fn ai_job_cancel(rt: &Runtime, args: &JobStatusArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.ai_job.cancel";
    runtime::require_writer(rt, tool)?;
    if !job_id_is_safe(&args.job_id) {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::ContractViolation,
                "malformed job id: expected a single path-safe component",
            ),
            tool,
        ));
    }
    let status = rimloc_services::ai_job::cancel_job(rt.ai_jobs(), &args.job_id)
        .map_err(|err| runtime::contract_error(err, tool))?;
    let structured = serde_json::to_value(&status)
        .map_err(|err| RpcError::internal(format!("job status serialization failed: {err}")))?;
    let state = structured["state"].as_str().unwrap_or("unknown");
    let summary = format!(
        "job {}: cancel requested, state {state} (the worker stops at its next boundary)",
        status.job_id
    );
    runtime::tool_ok(structured, summary)
}

fn ai_job_accept_selected(rt: &Runtime, args: &AcceptSelectedArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.ai_job.accept_selected";
    // The lock verdict dominates: this is a WRITE (rows ledger + a project
    // apply), so in other_writer mode it refuses before anything else.
    runtime::require_writer(rt, tool)?;
    if !job_id_is_safe(&args.job_id) {
        return Err(runtime::contract_error(
            ContractError::new(
                ContractErrorCode::ContractViolation,
                "malformed job id: expected a single path-safe component",
            ),
            tool,
        ));
    }
    // The job owns its project id — resolve it from the process registry
    // first, then from the durable record, so the session below opens the
    // RIGHT project with the caller's epoch guard. The service re-verifies
    // both the epoch and the pinned base revision under its own lock.
    let project_id = rt
        .ai_jobs()
        .get(&args.job_id)
        .map(|handle| {
            // A poisoned status mutex means a worker panicked while holding
            // it; the payload is plain data with no broken invariant, so
            // recover it instead of panicking the whole server.
            handle
                .status
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .project_id
                .clone()
        })
        .or_else(|| {
            rimloc_services::ai_job::job_status_from_record(rt.managed_root(), &args.job_id)
                .map(|st| st.project_id)
        })
        .ok_or_else(|| {
            runtime::contract_error(
                ContractError::new(
                    ContractErrorCode::ProjectNotFound,
                    format!(
                        "ai job `{}` not found (jobs live in the app process)",
                        args.job_id
                    ),
                ),
                tool,
            )
        })?;
    let (manager, _snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id,
            session_epoch: Some(args.session_epoch),
        },
    )?;
    let status = rimloc_services::ai_job::accept_selected(
        rt.ai_jobs(),
        &manager,
        &args.job_id,
        args.session_epoch,
        &args.selected,
        &args.rejected,
    )
    .map_err(|err| runtime::contract_error(err, tool))?;
    let structured = serde_json::to_value(&status)
        .map_err(|err| RpcError::internal(format!("job status serialization failed: {err}")))?;
    let state = structured["state"].as_str().unwrap_or("unknown");
    let summary = format!(
        "job {}: {} row(s) selected for apply, state {state}, translated {}/{}; \
         explicit rejected ledger: {} row(s)",
        status.job_id,
        args.selected.len(),
        status.translated,
        status.total,
        args.rejected.len()
    );
    runtime::tool_ok(structured, summary)
}

fn export_build(rt: &Runtime, args: &ExportBuildArgs) -> Result<Value, RpcError> {
    let tool = "rimloc.export.build";
    let (manager, _snap) = open_session(
        rt,
        tool,
        &ProjectRefArgs {
            project_id: args.project_id.clone(),
            session_epoch: Some(args.session_epoch),
        },
    )?;
    let out_dir = PathBuf::from(&args.out_dir);
    let resp = manager
        .export_project(&args.project_id, args.session_epoch, &out_dir, &args.locale)
        .map_err(|err| runtime::contract_error(err, tool))?;
    let structured = serde_json::to_value(&resp)
        .map_err(|err| RpcError::internal(format!("export serialization failed: {err}")))?;
    let summary = format!(
        "export {}: {} file(s) written to `{}`, {} key(s) reparse-verified{}",
        resp.job_id,
        resp.files_written,
        resp.out_dir.path,
        resp.reparsed_keys,
        if resp.skipped_unknown_type.is_empty() {
            String::new()
        } else {
            format!(
                ", {} unknown-type key(s) skipped",
                resp.skipped_unknown_type.len()
            )
        }
    );
    runtime::tool_ok(structured, summary)
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// `tools/call` dispatch: exact-name lookup, then the handler. Every
/// registered tool is wired (read-only + controlled writes); the fallback
/// arm stays as a typed safety net for future registry entries whose
/// handler has not landed.
pub fn call(name: &str, arguments: Value, rt: &Runtime) -> Result<Value, RpcError> {
    let def = REGISTRY
        .iter()
        .find(|tool| tool.name == name)
        .ok_or_else(|| RpcError::invalid_params(format!("unknown tool: {name}")))?;
    match name {
        "rimloc.status" => status(rt),
        "rimloc.capabilities" => capabilities(),
        "rimloc.projects.list" => projects_list(rt),
        "rimloc.project.snapshot" => runtime::parse_args(name, &arguments)
            .and_then(|args: ProjectRefArgs| project_snapshot(rt, &args)),
        "rimloc.entries.search" => runtime::parse_args(name, &arguments)
            .and_then(|args: SearchArgs| entries_search(rt, &args)),
        "rimloc.entry.context" => runtime::parse_args(name, &arguments)
            .and_then(|args: ContextArgs| entry_context(rt, &args)),
        "rimloc.validation.run" => runtime::parse_args(name, &arguments)
            .and_then(|args: ValidateArgs| validation_run(rt, &args)),
        "rimloc.export.preview" => runtime::parse_args(name, &arguments)
            .and_then(|args: PreviewArgs| export_preview(rt, &args)),
        "rimloc.jobs.status" | "rimloc.ai_job.status" => runtime::parse_args(name, &arguments)
            .and_then(|args: JobStatusArgs| jobs_status(rt, name, &args)),
        // ---- controlled writes -------------------------------------------------
        "rimloc.project.open" => runtime::parse_args(name, &arguments)
            .and_then(|args: ProjectOpenArgs| project_open(rt, &args)),
        "rimloc.translation.propose" => runtime::parse_args(name, &arguments)
            .and_then(|args: ProposeArgs| translation_propose(rt, &args)),
        "rimloc.translation.apply" => runtime::parse_args(name, &arguments)
            .and_then(|args: ApplyArgs| translation_apply(rt, &args)),
        "rimloc.ai_job.start" => runtime::parse_args(name, &arguments)
            .and_then(|args: AiJobStartArgs| ai_job_start(rt, &args)),
        "rimloc.ai_job.cancel" => runtime::parse_args(name, &arguments)
            .and_then(|args: JobStatusArgs| ai_job_cancel(rt, &args)),
        "rimloc.ai_job.accept_selected" => runtime::parse_args(name, &arguments)
            .and_then(|args: AcceptSelectedArgs| ai_job_accept_selected(rt, &args)),
        "rimloc.export.build" => runtime::parse_args(name, &arguments)
            .and_then(|args: ExportBuildArgs| export_build(rt, &args)),
        _ => {
            crate::stderr_log(&format!(
                "tools/call `{name}` -> not_implemented (registry entry without a handler)"
            ));
            Err(RpcError::not_implemented(def.name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::codes;

    /// All Phase 1 v1 tool names, in spec order (MCP_SERVER_PHASE1.md,
    /// «Поверхность инструментов v1») — the oracle the registry is pinned
    /// against. Lives in the test module: nothing in the binary consumes
    /// it.
    const V1_TOOL_NAMES: &[&str] = &[
        "rimloc.status",
        "rimloc.capabilities",
        "rimloc.projects.list",
        "rimloc.project.snapshot",
        "rimloc.entries.search",
        "rimloc.entry.context",
        "rimloc.validation.run",
        "rimloc.export.preview",
        "rimloc.jobs.status",
        "rimloc.project.open",
        "rimloc.translation.propose",
        "rimloc.translation.apply",
        "rimloc.ai_job.start",
        "rimloc.ai_job.status",
        "rimloc.ai_job.cancel",
        "rimloc.ai_job.accept_selected",
        "rimloc.export.build",
    ];

    #[test]
    fn registry_matches_the_phase1_v1_surface_exactly() {
        let registered: Vec<&str> = REGISTRY.iter().map(|t| t.name).collect();
        assert_eq!(registered, V1_TOOL_NAMES.to_vec());
    }

    #[test]
    fn registry_names_are_unique_and_namespaced() {
        let mut names: Vec<&str> = REGISTRY.iter().map(|t| t.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate tool names in the registry");
        assert!(names.iter().all(|n| n.starts_with("rimloc.")));
    }

    #[test]
    fn wired_lists_partition_the_registry_exactly() {
        // Every registry entry is wired on exactly one layer, and all
        // layers together are complete — dispatch has no dead registry
        // entries.
        let mut wired: Vec<&str> = WIRED_READ_ONLY.to_vec();
        wired.extend_from_slice(WIRED_READ_ONLY_ALIASES);
        wired.extend_from_slice(WIRED_WRITES);
        let mut registered: Vec<&str> = REGISTRY.iter().map(|t| t.name).collect();
        wired.sort_unstable();
        registered.sort_unstable();
        assert_eq!(wired, registered);
    }

    #[test]
    fn every_write_tool_is_wired_not_stubbed() {
        // The controlled-write layer must never answer not_implemented: a
        // call with EMPTY arguments fails on the missing REQUIRED
        // arguments (invalid_params) — proof the handler ran.
        for tool in WIRED_WRITES {
            let rt = test_runtime();
            let err = call(tool, json!({}), &rt).expect_err("empty args must refuse");
            assert_eq!(
                err.code,
                codes::INVALID_PARAMS,
                "{tool} must be wired (invalid_params on empty args), got: {err:?}"
            );
        }
    }

    #[test]
    fn session_epoch_is_required_on_every_guarded_write() {
        for tool in [
            "rimloc.translation.propose",
            "rimloc.translation.apply",
            "rimloc.ai_job.start",
            "rimloc.ai_job.accept_selected",
            "rimloc.export.build",
        ] {
            let def = REGISTRY
                .iter()
                .find(|t| t.name == tool)
                .expect("registered");
            let epoch = def.args.iter().find(|a| a.name == "session_epoch");
            assert!(epoch.is_some(), "{tool}: session_epoch arg must exist");
            let epoch = epoch.expect("proven by the assert above");
            assert!(epoch.required, "{tool}: session_epoch must be REQUIRED");
        }
        // And rimloc.project.open carries NO epoch: open mints it.
        let def = REGISTRY
            .iter()
            .find(|t| t.name == "rimloc.project.open")
            .expect("registered");
        assert!(
            def.args.iter().all(|a| a.name != "session_epoch"),
            "project.open mints the epoch — it must not take one"
        );
    }

    #[test]
    fn ai_job_start_takes_an_instance_id_only_never_a_secret() {
        let def = REGISTRY
            .iter()
            .find(|t| t.name == "rimloc.ai_job.start")
            .expect("registered");
        let names: Vec<&str> = def.args.iter().map(|a| a.name).collect();
        assert_eq!(
            names,
            vec![
                "project_id",
                "session_epoch",
                "locale",
                "provider_instance_id",
                "batch_char_budget",
            ]
        );
        let instance = def
            .args
            .iter()
            .find(|a| a.name == "provider_instance_id")
            .expect("instance arg");
        assert!(
            instance.required,
            "paid launches go ONLY through an existing provider instance"
        );
        // No argument may be a secret channel: no secret/key/token arg exists.
        for arg in def.args {
            let n = arg.name;
            assert!(
                !n.contains("secret") && !n.contains("key") && !n.contains("token"),
                "{n} must not be a secret-carrying argument"
            );
        }
        assert!(def.open_world, "the paid launch is the open-world tool");
    }

    #[test]
    fn unknown_tool_is_invalid_params() {
        let rt = test_runtime();
        let err = call("rimloc.does_not_exist", json!({}), &rt)
            .expect_err("unknown tool must be refused");
        assert_eq!(err.code, codes::INVALID_PARAMS);
        assert!(err.message.contains("rimloc.does_not_exist"));
    }

    #[test]
    fn read_only_tools_are_honestly_annotated() {
        for tool in REGISTRY {
            if tool.read_only {
                assert!(tool.idempotent, "{}: reads are idempotent", tool.name);
            }
            // The spec fixes destructiveHint=false for the whole v1 surface.
            assert!(
                !tool.destructive,
                "{}: destructiveHint must be false",
                tool.name
            );
            // Only the paid-provider launch may touch the outside world.
            assert_eq!(
                tool.open_world,
                tool.name == "rimloc.ai_job.start",
                "{}: openWorldHint honest",
                tool.name
            );
        }
    }

    #[test]
    fn tools_list_form_has_schema_and_annotations_for_every_tool() {
        let list = tools_list_result();
        let tools = list["tools"].as_array().expect("tools is an array");
        assert_eq!(tools.len(), REGISTRY.len());
        assert!(list.get("nextCursor").is_none(), "no fake pagination");
        for tool in tools {
            let name = tool["name"].as_str().expect("name is a string");
            assert_eq!(tool["inputSchema"]["type"], "object", "{name}");
            assert!(
                tool["inputSchema"]["properties"].is_object(),
                "{name}: properties is an object"
            );
            assert!(
                tool["inputSchema"]["required"].is_array(),
                "{name}: required is an array"
            );
            let annotations = &tool["annotations"];
            assert!(annotations["readOnlyHint"].is_boolean(), "{name}");
            assert!(annotations["destructiveHint"].is_boolean(), "{name}");
            assert!(annotations["idempotentHint"].is_boolean(), "{name}");
            assert!(annotations["openWorldHint"].is_boolean(), "{name}");
            assert_eq!(
                annotations["destructiveHint"], false,
                "{name}: the v1 surface is never destructive"
            );
        }
    }

    #[test]
    fn entries_search_schema_documents_pagination() {
        let def = REGISTRY
            .iter()
            .find(|t| t.name == "rimloc.entries.search")
            .expect("entries.search is registered");
        let schema = def.input_schema();
        let properties = schema["properties"]
            .as_object()
            .expect("properties is an object");
        for key in [
            "project_id",
            "session_epoch",
            "query",
            "offset",
            "max_results",
        ] {
            assert!(properties.contains_key(key), "missing property {key}");
        }
        let required: Vec<&str> = schema["required"]
            .as_array()
            .expect("required array")
            .iter()
            .map(|v| v.as_str().expect("string"))
            .collect();
        // Stateless sessions: the epoch guard is OPTIONAL (a fresh open
        // happens per call); the query is the real input.
        assert_eq!(required, &["project_id", "query"]);
        assert!(
            properties["max_results"]["description"]
                .as_str()
                .expect("description")
                .contains("1000"),
            "the HARD_MAX_RESULTS=1000 bound is documented"
        );
    }

    #[test]
    fn session_epoch_is_optional_on_every_project_scoped_read() {
        for tool in [
            "rimloc.project.snapshot",
            "rimloc.entries.search",
            "rimloc.entry.context",
            "rimloc.validation.run",
            "rimloc.export.preview",
        ] {
            let def = REGISTRY
                .iter()
                .find(|t| t.name == tool)
                .expect("registered");
            let epoch = def
                .args
                .iter()
                .find(|a| a.name == "session_epoch")
                .expect("epoch arg present");
            assert!(!epoch.required, "{tool}: session_epoch must be optional");
        }
    }

    #[test]
    fn argless_tools_advertise_an_empty_object_schema() {
        let def = REGISTRY
            .iter()
            .find(|t| t.name == "rimloc.status")
            .expect("status is registered");
        let schema = def.input_schema();
        assert_eq!(schema["required"].as_array().expect("required").len(), 0);
        assert!(schema["properties"]
            .as_object()
            .expect("properties")
            .is_empty());
    }

    // -- read-only execution smoke (full coverage: tests/readonly_tools.rs) --

    fn test_runtime() -> Runtime {
        let dir = tempfile::tempdir().expect("tempdir");
        Runtime::build(dir.path().join("data")).expect("runtime")
    }

    #[test]
    fn status_executes_against_a_tempdir_runtime() {
        let rt = test_runtime();
        let out = call("rimloc.status", json!({}), &rt).expect("status executes");
        assert_eq!(out["structuredContent"]["app"], "self");
        assert_eq!(out["structuredContent"]["mode"], "headless");
        assert_eq!(out["structuredContent"]["ui_contract_version"], 1);
        assert!(out["content"][0]["text"]
            .as_str()
            .expect("summary is a string")
            .contains("headless writer"));
    }

    #[test]
    fn capabilities_executes_without_a_project() {
        let rt = test_runtime();
        let out = call("rimloc.capabilities", json!({}), &rt).expect("capabilities executes");
        let supported = out["structuredContent"]["supported"]
            .as_array()
            .expect("supported is an array");
        assert!(!supported.is_empty());
        assert_eq!(out["structuredContent"]["contract_version"], 1);
    }

    #[test]
    fn job_id_form_guard_refuses_traversal_before_any_fs_access() {
        let rt = test_runtime();
        for bad in ["../evil", "a/b", "C:\\x", ".hidden", "..", ""] {
            let err = call("rimloc.jobs.status", json!({ "job_id": bad }), &rt)
                .expect_err("unsafe job id must refuse");
            assert_eq!(err.code, codes::CONTRACT_ERROR, "{bad}");
            assert_eq!(err.data.expect("data")["code"], "contract_violation");
        }
    }
}
