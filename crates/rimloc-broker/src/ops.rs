//! The broker operation surface: trait [`BrokerOps`] + the default
//! [`OpsImpl`] over `rimloc-services::ProjectSessionManager`.
//!
//! Surface = EXACTLY the MCP Phase 1 subset named by the Phase 2 design
//! (`MCP_SERVER_PHASE2.md` «Методы — ровно surface MCP Phase 1»), one
//! broker method per `rimloc.*` MCP tool:
//!
//! | broker method            | MCP tool                     | layer |
//! |--------------------------|------------------------------|-------|
//! | `projects.list`          | `rimloc.projects.list`       | read  |
//! | `project.snapshot`       | `rimloc.project.snapshot`    | read  |
//! | `entries.search`         | `rimloc.entries.search`      | read  |
//! | `entry.context`          | `rimloc.entry.context`       | read  |
//! | `validation.run`         | `rimloc.validation.run`      | read  |
//! | `export.preview`         | `rimloc.export.preview`      | read  |
//! | `jobs.status`            | `rimloc.jobs.status`         | read  |
//! | `project.open`           | `rimloc.project.open`        | write |
//! | `translation.propose`    | `rimloc.translation.propose` | write |
//! | `translation.apply`      | `rimloc.translation.apply`   | write |
//! | `ai_job.start`           | `rimloc.ai_job.start`        | write |
//! | `ai_job.status`          | `rimloc.ai_job.status`       | read  |
//! | `ai_job.cancel`          | `rimloc.ai_job.cancel`       | write |
//! | `ai_job.accept_selected` | `rimloc.ai_job.accept_selected` | write |
//! | `export.build`           | `rimloc.export.build`        | write |
//!
//! (`rimloc.status` / `rimloc.capabilities` are MCP-runtime surfaces and
//! intentionally NOT broker methods — the broker IS the status.)
//!
//! Handlers follow the rimloc-mcp `tools.rs` pattern (copied, not shared —
//! this crate's dependency budget is rimloc-services + serde + serde_json
//! only): a `ProjectSessionManager` per call, `open` the project, one
//! typed manager/service call, drop the manager; every collection on the
//! wire is bounded and truncation is flagged. WHERE the manager comes from
//! is a construction-time choice ([`OpsImpl::new`] builds a fresh one per
//! call — headless parity; [`OpsImpl::over_shared_manager`] runs every
//! call against ONE host-provided instance — the Phase 2 GUI host, whose
//! clone of the live `ContractState` manager shares the session registry).
//! The broker process is THE writer, so there is no other_writer branch —
//! but the restart-honest durable-record fallback for AI jobs stays.
//!
//! Paid-guard DOUBLE-CHECK (design: «доверяй, но проверяй»):
//! `ai_job.start` resolves the provider instance and refuses
//! `paid_providers_disabled` when its stored record has
//! `local == false` (the field lives on the provider record,
//! `crates/rimloc-services/src/providers.rs:146`) — BEFORE any launch,
//! regardless of what the connecting client already checked.
//!
//! Events: after EVERY successful write-layer call the impl emits
//! [`crate::protocol::EVENT_REVISION_CHANGED`] through the emitter the
//! server bound at startup (fan-out to all connected clients + the host
//! callback, e.g. the GUI's Tauri `emit`). Per-kind coordinate source:
//! - `project_open`: the fresh open ack (new epoch, current revision);
//! - `propose`: the drafts ack (`project_revision` — drafts do not bump
//!   the revision);
//! - `apply`: the apply ack (`revision` — the NEW durable revision);
//! - `ai_job_start`: the session snapshot taken by the same call;
//! - `ai_job_cancel`: the job's pinned `base_revision`, session_epoch
//!   sentinel `0` (a cancel carries no session — documented, not faked);
//! - `accept_selected`: a post-apply `manager.snapshot()` (no epoch
//!   bump) for the honest current coordinates;
//! - `export_build`: the session snapshot (export never bumps it).

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Deserialize;
use serde_json::{json, Value};

use rimloc_services::ai_job::AiJobRegistry;
use rimloc_services::contract::{
    ApplyIntentsRequest, ContractError, ContractErrorCode, ProjectSnapshot,
};
use rimloc_services::session::ProjectSessionManager;
use rimloc_services::source_search::{search_sources, SearchScope, SourceSearchRequest};

use crate::protocol::{BrokerError, BrokerEvent, RevisionChangedPayload};

// ---------------------------------------------------------------------------
// Typed params (wire → struct; serde refusals become typed invalid_params)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ProjectRefParams {
    pub project_id: String,
    #[serde(default)]
    pub session_epoch: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub project_id: String,
    #[serde(default)]
    pub session_epoch: Option<u64>,
    pub query: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ContextParams {
    pub project_id: String,
    #[serde(default)]
    pub session_epoch: Option<u64>,
    pub file: String,
    #[serde(default)]
    pub line: Option<usize>,
    #[serde(default)]
    pub before: Option<usize>,
    #[serde(default)]
    pub after: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateParams {
    pub project_id: String,
    #[serde(default)]
    pub session_epoch: Option<u64>,
    #[serde(default)]
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewParams {
    pub project_id: String,
    #[serde(default)]
    pub session_epoch: Option<u64>,
    pub locale: String,
}

#[derive(Debug, Deserialize)]
pub struct JobIdParams {
    pub job_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectOpenParams {
    pub project_id: String,
}

/// One draft row of `translation.propose`. The identity is the FULL
/// structural identity shape; it is validated HERE against the kind
/// whitelist and rendered with the GUI draft-key convention
/// (`<locale>::<display_identity()>`). `SourceEntryId` itself is not
/// nameable through this crate's dependency budget (rimloc-services +
/// serde + serde_json only), so the shape is mirrored and the rendered
/// key is pinned byte-for-byte by a test
/// (`propose_writes_gui_compatible_draft_keys`) against the same
/// `"Russian::keyed·Fixture_Greet"` string the rimloc-mcp suite pins
/// (`crates/rimloc-mcp/tests/writable_tools.rs:357`); at apply time the
/// service re-resolves every identity against the trusted inventory, so
/// a forged key can never become a translation.
#[derive(Debug, Deserialize)]
pub struct ProposeIdentityParams {
    pub kind: String,
    pub key: String,
    #[serde(default)]
    pub def_type: Option<String>,
}

impl ProposeIdentityParams {
    /// Byte-compatible mirror of
    /// `rimloc_domain::canonical::SourceEntryId::display_identity` —
    /// kind strings and the `·` join are pinned by that crate's source
    /// and by our draft-key test.
    fn display_identity(&self) -> Result<String, BrokerError> {
        let kind = match self.kind.as_str() {
            "keyed" => "keyed",
            "def_injected" => "def_injected",
            "t_key" => "t_key",
            "strings" => "strings",
            "backstories" => "backstories",
            "patch_derived" => "patch_derived",
            other => {
                return Err(BrokerError::invalid_params(format!(
                    "unknown identity kind `{other}` (known: keyed, def_injected, t_key, \
                     strings, backstories, patch_derived)"
                )))
            }
        };
        let key = self.key.trim();
        if key.is_empty() {
            return Err(BrokerError::invalid_params(
                "identity key must not be empty",
            ));
        }
        Ok(match &self.def_type {
            Some(dt) if !dt.trim().is_empty() => format!("{kind}·{}·{key}", dt.trim()),
            _ => format!("{kind}·{key}"),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ProposeParams {
    pub project_id: String,
    pub session_epoch: u64,
    pub locale: String,
    pub entries: Vec<ProposeEntryParams>,
}

#[derive(Debug, Deserialize)]
pub struct ProposeEntryParams {
    pub identity: ProposeIdentityParams,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct ApplyParams {
    pub project_id: String,
    pub session_epoch: u64,
    pub expected_revision: u64,
    /// Typed intents, deserialized straight into the services contract
    /// type (`ApplyIntentsRequest.intents`) — serde validates every
    /// identity/action; this crate never re-parses them.
    pub intents: Vec<rimloc_services::contract::TranslationIntent>,
    #[serde(default)]
    pub origin: Option<rimloc_services::contract::ApplyOrigin>,
}

#[derive(Debug, Deserialize)]
pub struct AiJobStartParams {
    pub project_id: String,
    pub session_epoch: u64,
    pub locale: String,
    pub provider_instance_id: String,
    #[serde(default)]
    pub batch_char_budget: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptSelectedParams {
    pub job_id: String,
    pub session_epoch: u64,
    pub selected: Vec<String>,
    #[serde(default)]
    pub rejected: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportBuildParams {
    pub project_id: String,
    pub session_epoch: u64,
    pub out_dir: String,
    pub locale: String,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// The broker operation seam. The server dispatches through
/// [`BrokerOps::dispatch`]; the default provided `dispatch` maps method
/// names onto the typed methods and turns serde failures into typed
/// `invalid_params`. A GUI host may implement the trait over its own
/// live `ContractState` instead of using [`OpsImpl`].
pub trait BrokerOps: Send + Sync + 'static {
    /// The server binds the fan-out emitter at startup (broadcast to all
    /// connected clients + the host notify callback). Default: no-op, so
    /// a custom impl or a bare test driver needs no wiring.
    fn bind_emitter(&self, _emit: EventEmitter) {}

    // -- read layer ---------------------------------------------------------
    fn projects_list(&self) -> Result<Value, BrokerError>;
    fn project_snapshot(&self, params: ProjectRefParams) -> Result<Value, BrokerError>;
    fn entries_search(&self, params: SearchParams) -> Result<Value, BrokerError>;
    fn entry_context(&self, params: ContextParams) -> Result<Value, BrokerError>;
    fn validation_run(&self, params: ValidateParams) -> Result<Value, BrokerError>;
    fn export_preview(&self, params: PreviewParams) -> Result<Value, BrokerError>;
    fn jobs_status(&self, params: JobIdParams) -> Result<Value, BrokerError>;

    // -- controlled-write layer (each success emits revision_changed) --------
    fn project_open(&self, params: ProjectOpenParams) -> Result<Value, BrokerError>;
    fn translation_propose(&self, params: ProposeParams) -> Result<Value, BrokerError>;
    fn translation_apply(&self, params: ApplyParams) -> Result<Value, BrokerError>;
    fn ai_job_start(&self, params: AiJobStartParams) -> Result<Value, BrokerError>;
    fn ai_job_status(&self, params: JobIdParams) -> Result<Value, BrokerError>;
    fn ai_job_cancel(&self, params: JobIdParams) -> Result<Value, BrokerError>;
    fn ai_job_accept_selected(&self, params: AcceptSelectedParams) -> Result<Value, BrokerError>;
    fn export_build(&self, params: ExportBuildParams) -> Result<Value, BrokerError>;

    /// Method-name dispatch: exact match only — no prefix matching, no
    /// dynamic execution, the same fixed-table discipline as the MCP
    /// registry.
    fn dispatch(&self, method: &str, params: Value) -> Result<Value, BrokerError> {
        match method {
            "projects.list" => self.projects_list(),
            "project.snapshot" => self.project_snapshot(parse_params(method, &params)?),
            "entries.search" => self.entries_search(parse_params(method, &params)?),
            "entry.context" => self.entry_context(parse_params(method, &params)?),
            "validation.run" => self.validation_run(parse_params(method, &params)?),
            "export.preview" => self.export_preview(parse_params(method, &params)?),
            "jobs.status" => self.jobs_status(parse_params(method, &params)?),
            "project.open" => self.project_open(parse_params(method, &params)?),
            "translation.propose" => self.translation_propose(parse_params(method, &params)?),
            "translation.apply" => self.translation_apply(parse_params(method, &params)?),
            "ai_job.start" => self.ai_job_start(parse_params(method, &params)?),
            "ai_job.status" => self.ai_job_status(parse_params(method, &params)?),
            "ai_job.cancel" => self.ai_job_cancel(parse_params(method, &params)?),
            "ai_job.accept_selected" => self.ai_job_accept_selected(parse_params(method, &params)?),
            "export.build" => self.export_build(parse_params(method, &params)?),
            other => Err(BrokerError::method_not_found(other)),
        }
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(
    method: &str,
    params: &Value,
) -> Result<T, BrokerError> {
    serde_json::from_value(params.clone()).map_err(|err| {
        BrokerError::invalid_params(format!("invalid params for `{method}`: {err}"))
            .with_data(json!({ "code": "invalid_arguments", "method": method }))
    })
}

// ---------------------------------------------------------------------------
// OpsImpl — default implementation over ProjectSessionManager
// ---------------------------------------------------------------------------

/// The event fan-out seam: the server binds one at startup; calling it
/// broadcasts the event to every connected client AND the host callback.
pub type EventEmitter = Arc<dyn Fn(BrokerEvent) + Send + Sync>;

type EmitterSlot = Mutex<Option<EventEmitter>>;

/// Where the per-call [`ProjectSessionManager`] comes from.
///
/// - [`ManagerSource::Root`] — a FRESH manager per call over this root,
///   exactly the `crates/rimloc-mcp/src/tools.rs` stateless discipline
///   (headless mode and the crate's own tests);
/// - [`ManagerSource::Shared`] — ONE manager instance handed over by the
///   host (Phase 2: the GUI passes a clone of its live `ContractState`
///   manager; `ProjectSessionManager` is an `Arc`-based `Clone`,
///   `rimloc-services/src/session.rs:101`, so the clone shares the session
///   registry, provider settings and the keychain sink). Broker
///   operations then run against the SAME live sessions/epochs the GUI
///   commands see — the single-writer invariant's in-process half.
enum ManagerSource {
    Root(PathBuf),
    Shared(ProjectSessionManager),
}

/// The default [`BrokerOps`]: stateless sessions (fresh manager per call)
/// exactly like `crates/rimloc-mcp/src/tools.rs`, plus the process-owned
/// AI-job registry (jobs live in the app process per the services
/// contract; the durable `<job_id>.job.json` records are the
/// restart-honest fallback).
pub struct OpsImpl {
    source: ManagerSource,
    ai_jobs: AiJobRegistry,
    emit: EmitterSlot,
}

impl OpsImpl {
    /// Headless/test mode: a fresh manager per call over `managed_root`
    /// (tools.rs discipline).
    pub fn new(managed_root: impl Into<PathBuf>) -> Self {
        Self {
            source: ManagerSource::Root(managed_root.into()),
            ai_jobs: AiJobRegistry::new(),
            emit: Mutex::new(None),
        }
    }

    /// Host mode (Phase 2 GUI): run every operation against the given
    /// manager INSTANCE — the host passes a clone of its live manager, so
    /// the broker drives the same sessions the host's own commands do
    /// instead of constructing rival managers over the same root. The
    /// durable-record root for AI jobs is the manager's own
    /// [`ProjectSessionManager::managed_root`]. Epoch discipline: method
    /// calls keep the caller's epoch guard honest against the LIVE
    /// session without bumping it — `project.open` is the one
    /// epoch-minting verb (see [`Self::open_session`]).
    pub fn over_shared_manager(manager: ProjectSessionManager) -> Self {
        Self {
            source: ManagerSource::Shared(manager),
            ai_jobs: AiJobRegistry::new(),
            emit: Mutex::new(None),
        }
    }

    /// The authoritative managed root (AI-job durable records, socket
    /// placement): the configured root, or the shared manager's own.
    pub fn managed_root(&self) -> &Path {
        match &self.source {
            ManagerSource::Root(root) => root.as_path(),
            ManagerSource::Shared(manager) => manager.managed_root(),
        }
    }

    /// Poison-tolerant lock (a panicking emitter callback must not turn
    /// the next successful mutation into a panic).
    fn emitter(&self) -> MutexGuard<'_, Option<EventEmitter>> {
        self.emit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Fan one event out: all connected clients (server-side registry)
    /// AND the host callback (the GUI's Tauri emit). No emitter bound →
    /// silently nothing (a bare test driver).
    fn notify(&self, payload: RevisionChangedPayload) {
        let event = BrokerEvent::revision_changed(payload);
        if let Some(emit) = self.emitter().as_ref() {
            emit(event);
        }
    }
}

impl BrokerOps for OpsImpl {
    fn bind_emitter(&self, emit: EventEmitter) {
        *self.emitter() = Some(emit);
    }

    // -- read layer ---------------------------------------------------------

    fn projects_list(&self) -> Result<Value, BrokerError> {
        let manager = new_manager(self, "projects.list")?;
        let report = manager.list_report();
        to_value(&report, "projects.list")
    }

    fn project_snapshot(&self, params: ProjectRefParams) -> Result<Value, BrokerError> {
        let (_manager, snap) =
            self.open_session("project.snapshot", &params.project_id, params.session_epoch)?;
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
        Ok(structured)
    }

    fn entries_search(&self, params: SearchParams) -> Result<Value, BrokerError> {
        let (_manager, snap) =
            self.open_session("entries.search", &params.project_id, params.session_epoch)?;
        let root = source_root_of(&snap)?;
        let scope = match params.scope.as_deref() {
            None | Some("standard") => SearchScope::Standard,
            Some("all_text") => SearchScope::AllText,
            Some(other) => {
                return Err(BrokerError::invalid_params(format!(
                    "invalid scope `{other}`: expected \"standard\" or \"all_text\""
                ))
                .with_data(json!({ "code": "invalid_arguments", "method": "entries.search" })))
            }
        };
        let request = SourceSearchRequest {
            mod_root: root,
            target_version: snap.project.context.target_version.clone(),
            source_lang_dir: snap.project.context.source_dir.clone(),
            query: params.query.clone(),
            scope,
            offset: params.offset.unwrap_or(0),
            // 0 = "service default" (200), the service clamps at HARD_MAX 1000.
            max_results: params.max_results.unwrap_or(0),
            // Reconciliation against the trusted inventory, same as the
            // GUI adapter and the MCP handler.
            manual_ids: snap.project.entries.iter().map(|e| e.id.clone()).collect(),
        };
        let report = search_sources(&request, &|| false).map_err(BrokerError::contract)?;
        to_value(&report, "entries.search")
    }

    fn entry_context(&self, params: ContextParams) -> Result<Value, BrokerError> {
        const CONTEXT_MAX_BEFORE_AFTER: usize = 200;
        const CONTEXT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
        let (_manager, snap) =
            self.open_session("entry.context", &params.project_id, params.session_epoch)?;
        let root = source_root_of(&snap)?;
        let rel = Path::new(&params.file);
        let escapes = rel.is_absolute()
            || rel.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            });
        if escapes {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root",
            )));
        }
        let full = root.join(rel);
        // The read-target must EXIST before containment can be proven on
        // the canonical view; an unresolvable/symlinked-out path refuses
        // instead of guessing (tools.rs semantics, copied).
        let meta = std::fs::metadata(&full).map_err(|err| {
            BrokerError::contract(ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {err}", params.file),
            ))
        })?;
        if !is_within_allow(&full, &root) {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root",
            )));
        }
        if meta.len() > CONTEXT_MAX_FILE_BYTES {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "source file is {} bytes; the preview read cap is {CONTEXT_MAX_FILE_BYTES} — \
                     use entries.search to locate the region first",
                    meta.len()
                ),
            )));
        }
        let canonical = full.canonicalize().map_err(|err| {
            BrokerError::contract(ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {err}", params.file),
            ))
        })?;
        if !is_within_allow(&canonical, &root) {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root",
            )));
        }
        let bytes = std::fs::read(&canonical).map_err(|err| {
            BrokerError::contract(ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {err}", params.file),
            ))
        })?;
        // The metadata check can race a growing file; the wire answer is
        // bounded by what was actually read.
        if bytes.len() as u64 > CONTEXT_MAX_FILE_BYTES {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "source file grew past the read cap ({CONTEXT_MAX_FILE_BYTES} bytes) \
                     while opening — retry"
                ),
            )));
        }
        if bytes.iter().take(8192).any(|b| *b == 0) {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::InvalidConfig,
                "source preview: file looks binary",
            )));
        }
        let content = String::from_utf8_lossy(&bytes);
        let total_lines = content.lines().count();
        let target = params.line.unwrap_or(1).max(1).min(total_lines.max(1));
        let before = params.before.unwrap_or(0).min(CONTEXT_MAX_BEFORE_AFTER);
        let after = params.after.unwrap_or(0).min(CONTEXT_MAX_BEFORE_AFTER);
        let start = target.saturating_sub(before).max(1);
        let end = (target + after).min(total_lines.max(1));
        let lines: Vec<Value> = content
            .lines()
            .skip(start - 1)
            .take(end - start + 1)
            .enumerate()
            .map(|(i, text)| json!({ "n": start + i, "text": text }))
            .collect();
        Ok(json!({
            "file": params.file,
            "target_line": target,
            "total_lines": total_lines,
            "truncated_head": start > 1,
            "truncated_tail": end < total_lines,
            "before_cap": CONTEXT_MAX_BEFORE_AFTER,
            "after_cap": CONTEXT_MAX_BEFORE_AFTER,
            "lines": lines,
        }))
    }

    fn validation_run(&self, params: ValidateParams) -> Result<Value, BrokerError> {
        let (manager, snap) =
            self.open_session("validation.run", &params.project_id, params.session_epoch)?;
        let resp = manager
            .validate_project(
                &params.project_id,
                snap.session_epoch,
                params.locale.as_deref(),
            )
            .map_err(BrokerError::contract)?;
        // Bounded surface: findings grow with the project, not with the
        // request — capped window plus the true totals (tools.rs:1328).
        const MAX_FINDINGS_ON_WIRE: usize = 200;
        let findings_total = resp.findings.len();
        let findings_truncated = findings_total > MAX_FINDINGS_ON_WIRE;
        let mut structured = to_value(&resp, "validation.run")?;
        if let Some(obj) = structured.as_object_mut() {
            if findings_truncated {
                if let Some(findings) = obj.get_mut("findings").and_then(|f| f.as_array_mut()) {
                    findings.truncate(MAX_FINDINGS_ON_WIRE);
                }
            }
            obj.insert("findings_total".to_string(), json!(findings_total));
            obj.insert("findings_truncated".to_string(), json!(findings_truncated));
        }
        Ok(structured)
    }

    fn export_preview(&self, params: PreviewParams) -> Result<Value, BrokerError> {
        let (manager, snap) =
            self.open_session("export.preview", &params.project_id, params.session_epoch)?;
        let resp = manager
            .project_output_xml_preview(&params.project_id, snap.session_epoch, &params.locale)
            .map_err(BrokerError::contract)?;
        to_value(&resp, "export.preview")
    }

    fn jobs_status(&self, params: JobIdParams) -> Result<Value, BrokerError> {
        let status = self.job_status_typed(&params.job_id)?;
        to_value(&status, "jobs.status")
    }

    // -- controlled-write layer ---------------------------------------------

    fn project_open(&self, params: ProjectOpenParams) -> Result<Value, BrokerError> {
        let manager = new_manager(self, "project.open")?;
        // project.open is THE epoch-minting verb, in BOTH modes: `open()`
        // bumps a live session to its next generation (the stale-session
        // contract) or recovers one from disk. This is the ONE broker
        // method allowed to bump implicitly — callers re-open exactly when
        // they mean to rebase.
        let snap = manager
            .open(&params.project_id)
            .map_err(BrokerError::contract)?;
        let structured = json!({
            "project_id": snap.project_id,
            "revision": snap.revision,
            "session_epoch": snap.session_epoch,
            "dirty": snap.dirty,
            "acked_revision": snap.acked_revision,
            "source_changed": snap.source_changed,
            "counts": {
                "entries": snap.project.entries.len(),
                "translations": snap.project.translations.len(),
            },
        });
        self.notify(RevisionChangedPayload {
            project_id: snap.project_id.clone(),
            revision: snap.revision,
            session_epoch: snap.session_epoch,
            kind: "project_open".to_string(),
        });
        // `manager` kept alive through here on purpose: dropping it closes
        // the session (there is no close API — tools.rs discipline).
        drop(manager);
        Ok(structured)
    }

    fn translation_propose(&self, params: ProposeParams) -> Result<Value, BrokerError> {
        if params.entries.is_empty() {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::ContractViolation,
                "translation.propose received zero entries — a proposal must name at \
                 least one identity",
            )));
        }
        let (manager, _snap) = self.open_session(
            "translation.propose",
            &params.project_id,
            Some(params.session_epoch),
        )?;
        // Draft keys are `<locale>::<display_identity()>` — the UI
        // convention, derived HERE (the client never supplies the key
        // string). The store merges: entries not mentioned survive.
        let mut entries: BTreeMap<String, rimloc_services::DraftEntry> = BTreeMap::new();
        for row in &params.entries {
            let display = row.identity.display_identity()?;
            let key = format!("{}::{}", params.locale, display);
            entries.insert(
                key,
                rimloc_services::DraftEntry::new(row.text.clone(), now_ms()),
            );
        }
        let proposed = params.entries.len();
        let ack = manager
            .save_drafts_session(&params.project_id, params.session_epoch, entries)
            .map_err(BrokerError::contract)?;
        let structured = json!({
            "project_id": params.project_id,
            "proposed": proposed,
            "saved_total": ack.saved,
            "project_revision": ack.project_revision,
            "updated_at": ack.updated_at,
            "committed": false,
            "note": "drafts only — nothing became a committed translation; apply explicitly",
        });
        self.notify(RevisionChangedPayload {
            project_id: params.project_id.clone(),
            revision: ack.project_revision,
            session_epoch: params.session_epoch,
            kind: "propose".to_string(),
        });
        Ok(structured)
    }

    fn translation_apply(&self, params: ApplyParams) -> Result<Value, BrokerError> {
        let (manager, _snap) = self.open_session(
            "translation.apply",
            &params.project_id,
            Some(params.session_epoch),
        )?;
        let req = ApplyIntentsRequest {
            project_id: params.project_id.clone(),
            expected_revision: params.expected_revision,
            session_epoch: params.session_epoch,
            intents: params.intents.clone(),
            origin: params.origin,
        };
        let resp = manager.apply(&req).map_err(BrokerError::contract)?;
        let structured = to_value(&resp, "translation.apply")?;
        self.notify(RevisionChangedPayload {
            project_id: params.project_id.clone(),
            revision: resp.revision,
            session_epoch: params.session_epoch,
            kind: "apply".to_string(),
        });
        Ok(structured)
    }

    fn ai_job_start(&self, params: AiJobStartParams) -> Result<Value, BrokerError> {
        let (manager, snap) = self.open_session(
            "ai_job.start",
            &params.project_id,
            Some(params.session_epoch),
        )?;
        // PAID-GUARD DOUBLE-CHECK (broker side of the MCP client guard):
        // resolve the instance record and refuse anything non-local
        // BEFORE any launch. An UNKNOWN instance falls through to
        // start_job, whose own resolution refuses it typed
        // (`project_not_found` wording "unknown provider instance") —
        // an unknown instance is not a paid instance.
        let list = manager
            .provider_instance_list()
            .map_err(BrokerError::contract)?;
        if let Some(instance) = list
            .instances
            .iter()
            .find(|i| i.id == params.provider_instance_id)
        {
            if !instance.local {
                return Err(BrokerError::paid_providers_disabled(
                    &params.provider_instance_id,
                ));
            }
        }
        // The instance FULLY defines the launch target (preset → provider
        // id, model, base_url, keychain key); `provider_id` is a
        // placeholder the service overwrites during resolution. Secrets
        // are resolved in-process and never cross the wire.
        let job_params = rimloc_services::ai_job::AiJobParams {
            project_id: params.project_id.clone(),
            session_epoch: params.session_epoch,
            locale: params.locale.clone(),
            source_locale: snap
                .project
                .context
                .source_locale
                .clone()
                .unwrap_or_default(),
            provider_id: String::new(),
            model: None,
            base_url: None,
            provider_instance_id: Some(params.provider_instance_id.clone()),
            batch_char_budget: params.batch_char_budget.unwrap_or(0),
        };
        let status = rimloc_services::ai_job::start_job(
            &self.ai_jobs,
            &manager,
            self.managed_root(),
            job_params,
        )
        .map_err(BrokerError::contract)?;
        let structured = to_value(&status, "ai_job.start")?;
        // Job creation writes durable records into the managed root; the
        // project coordinates themselves are unchanged — report the
        // session truth so clients can correlate.
        self.notify(RevisionChangedPayload {
            project_id: params.project_id.clone(),
            revision: snap.revision,
            session_epoch: params.session_epoch,
            kind: "ai_job_start".to_string(),
        });
        Ok(structured)
    }

    fn ai_job_status(&self, params: JobIdParams) -> Result<Value, BrokerError> {
        let status = self.job_status_typed(&params.job_id)?;
        to_value(&status, "ai_job.status")
    }

    fn ai_job_cancel(&self, params: JobIdParams) -> Result<Value, BrokerError> {
        let status = rimloc_services::ai_job::cancel_job(&self.ai_jobs, &params.job_id)
            .map_err(BrokerError::contract)?;
        let structured = to_value(&status, "ai_job.cancel")?;
        // Event coordinates: the job's pinned base revision; session
        // epoch sentinel 0 — a cancel carries no session (documented in
        // the module docs, not faked).
        if let Some(project_id) = self.job_project_id(&params.job_id) {
            self.notify(RevisionChangedPayload {
                project_id,
                revision: status.base_revision,
                session_epoch: 0,
                kind: "ai_job_cancel".to_string(),
            });
        }
        Ok(structured)
    }

    fn ai_job_accept_selected(&self, params: AcceptSelectedParams) -> Result<Value, BrokerError> {
        // The job owns its project id — resolve it from the process
        // registry first, then from the durable record, so the session
        // below opens the RIGHT project with the caller's epoch guard.
        let project_id = self.job_project_id(&params.job_id).ok_or_else(|| {
            BrokerError::contract(ContractError::new(
                ContractErrorCode::ProjectNotFound,
                format!(
                    "ai job `{}` not found (jobs live in the app process)",
                    params.job_id
                ),
            ))
        })?;
        let (manager, _snap) = self.open_session(
            "ai_job.accept_selected",
            &project_id,
            Some(params.session_epoch),
        )?;
        let status = rimloc_services::ai_job::accept_selected(
            &self.ai_jobs,
            &manager,
            &params.job_id,
            params.session_epoch,
            &params.selected,
            &params.rejected,
        )
        .map_err(BrokerError::contract)?;
        let structured = to_value(&status, "ai_job.accept_selected")?;
        // The accept applies rows through the manager — the project MAY
        // have moved. The session is open in THIS manager, so a
        // `snapshot()` (no epoch bump) reports the honest current
        // coordinates; if it somehow fails, the job's pinned base is the
        // fallback truth.
        let (revision, epoch) = match manager.snapshot(&project_id) {
            Ok(snap) => (snap.revision, snap.session_epoch),
            Err(_) => (status.base_revision, params.session_epoch),
        };
        self.notify(RevisionChangedPayload {
            project_id,
            revision,
            session_epoch: epoch,
            kind: "accept_selected".to_string(),
        });
        Ok(structured)
    }

    fn export_build(&self, params: ExportBuildParams) -> Result<Value, BrokerError> {
        let (manager, snap) = self.open_session(
            "export.build",
            &params.project_id,
            Some(params.session_epoch),
        )?;
        let out_dir = PathBuf::from(&params.out_dir);
        let resp = manager
            .export_project(
                &params.project_id,
                params.session_epoch,
                &out_dir,
                &params.locale,
            )
            .map_err(BrokerError::contract)?;
        let structured = to_value(&resp, "export.build")?;
        // Export writes ONLY the explicit guarded out dir; the managed
        // revision is unchanged — report the session coordinates.
        self.notify(RevisionChangedPayload {
            project_id: params.project_id.clone(),
            revision: snap.revision,
            session_epoch: params.session_epoch,
            kind: "export_build".to_string(),
        });
        Ok(structured)
    }
}

// ---------------------------------------------------------------------------
// OpsImpl internals (tools.rs seams, copied verbatim where noted)
// ---------------------------------------------------------------------------

impl OpsImpl {
    /// Manager per call + `open` + the optional caller-epoch guard
    /// (the `open_session` seam of tools.rs:869; the broker has no
    /// other_writer branch — it IS the writer).
    ///
    /// Epoch discipline DIFFERS by mode, and that is the point:
    /// - [`ManagerSource::Root`] — the fresh per-call manager has no live
    ///   session, so `open()` IS the session start (epoch 1): the
    ///   stateless headless pattern;
    /// - [`ManagerSource::Shared`] — the host's registry holds LIVE
    ///   sessions, where `open()` BUMPS the epoch. An implicit bump on
    ///   every broker read/write would invalidate the host's own open
    ///   sessions — the exact damage Phase 2 forbids. So the live session
    ///   is read via `snapshot()` (no bump) and `open()` runs only when
    ///   nothing is live yet (first touch / disk recovery). The caller's
    ///   epoch guard is then compared against the CURRENT live epoch
    ///   without disturbing it.
    fn open_session(
        &self,
        method: &str,
        project_id: &str,
        session_epoch: Option<u64>,
    ) -> Result<(ProjectSessionManager, ProjectSnapshot), BrokerError> {
        let manager = new_manager(self, method)?;
        let snap = match &self.source {
            ManagerSource::Root(_) => manager.open(project_id).map_err(BrokerError::contract)?,
            ManagerSource::Shared(_) => match manager.snapshot(project_id) {
                Ok(snap) => snap,
                Err(_) => manager.open(project_id).map_err(BrokerError::contract)?,
            },
        };
        if let Some(want) = session_epoch {
            if want != snap.session_epoch {
                return Err(BrokerError::contract(ContractError::stale_epoch(
                    want,
                    snap.session_epoch,
                )));
            }
        }
        Ok((manager, snap))
    }

    /// Job-status seam (tools.rs:1408): the process registry first, the
    /// durable record as the restart-honest fallback; malformed ids
    /// refuse before any filesystem access.
    fn job_status_typed(
        &self,
        job_id: &str,
    ) -> Result<rimloc_services::ai_job::AiJobStatus, BrokerError> {
        if !job_id_is_safe(job_id) {
            return Err(BrokerError::contract(ContractError::new(
                ContractErrorCode::ContractViolation,
                "malformed job id: expected a single path-safe component",
            )));
        }
        if let Ok(status) = rimloc_services::ai_job::job_status(&self.ai_jobs, job_id) {
            return Ok(status);
        }
        rimloc_services::ai_job::job_status_from_record(self.managed_root(), job_id).ok_or_else(
            || {
                BrokerError::contract(ContractError::new(
                    ContractErrorCode::ProjectNotFound,
                    format!("ai job `{job_id}` has no live state and no durable record"),
                ))
            },
        )
    }

    /// The project a job belongs to: registry first, durable record
    /// second (tools.rs:1664). Poisoned status mutexes recover their
    /// plain-data payload instead of panicking.
    fn job_project_id(&self, job_id: &str) -> Option<String> {
        self.ai_jobs
            .get(job_id)
            .map(|handle| {
                handle
                    .status
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .project_id
                    .clone()
            })
            .or_else(|| {
                rimloc_services::ai_job::job_status_from_record(self.managed_root(), job_id)
                    .map(|st| st.project_id)
            })
    }
}

/// Stateless manager seam (tools.rs `runtime::new_manager`): constructor
/// I/O failures are typed internal errors, never panics. In host mode
/// (`ManagerSource::Shared`) there is nothing to construct — the shared
/// instance is cloned out (an `Arc`-cheap clone) and the root's
/// constructor errors simply cannot happen.
fn new_manager(ops: &OpsImpl, method: &str) -> Result<ProjectSessionManager, BrokerError> {
    match &ops.source {
        ManagerSource::Root(root) => ProjectSessionManager::new(root).map_err(|err| {
            BrokerError::internal(format!("session manager init failed for `{method}`: {err}"))
                .with_data(json!({ "code": "session_init_failed", "method": method }))
        }),
        ManagerSource::Shared(manager) => Ok(manager.clone()),
    }
}

fn to_value<T: serde::Serialize>(value: &T, method: &str) -> Result<Value, BrokerError> {
    serde_json::to_value(value)
        .map_err(|err| BrokerError::internal(format!("`{method}` serialization failed: {err}")))
}

/// Wall-clock milliseconds for draft stamps (tools.rs:1456; the services
/// helper is crate-private).
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn source_root_of(snap: &ProjectSnapshot) -> Result<PathBuf, BrokerError> {
    snap.source_root
        .as_ref()
        .map(|r| PathBuf::from(&r.path))
        .ok_or_else(|| {
            BrokerError::contract(ContractError::new(
                ContractErrorCode::InvalidConfig,
                "project has no source root — the source mod is unavailable",
            ))
        })
}

/// Allow-direction containment (tools.rs:915): containment must be PROVEN
/// on the canonical (symlink-resolved) view — an unresolvable path refuses
/// instead of guessing.
fn is_within_allow(candidate: &Path, root: &Path) -> bool {
    match (candidate.canonicalize(), root.canonicalize()) {
        (Ok(c), Ok(r)) => c.starts_with(&r),
        _ => false,
    }
}

/// Job ids become file names under the managed root
/// (`<job_id>.job.json`): refuse anything that is not a single safe
/// component BEFORE any filesystem access (tools.rs:1401).
fn job_id_is_safe(job_id: &str) -> bool {
    !job_id.is_empty()
        && !job_id.starts_with('.')
        && !job_id.contains(['/', '\\'])
        && !job_id.contains("..")
}
