//! Thin Tauri adapter over the J binding seam (binding wave 2).
//!
//! Contract rules enforced by review here:
//! - NO business logic: every command is a pass-through into
//!   [`rimloc_services::session::ProjectSessionManager`]; the adapter owns
//!   state and wire-shape only.
//! - Errors serialize to the typed contract shape
//!   `{ code, message, details? }` — stable snake_case codes from
//!   [`rimloc_services::contract::ContractErrorCode`]; the adapter never
//!   invents its own codes.
//! - Registration plan: the production live entry registers the contract
//!   commands plus SAFE read-only legacy extras; the PRIVILEGED legacy
//!   surfaces (source-tree writes, arbitrary open, plugin loading,
//!   provider invocation, raw diagnostics) are registered only when the
//!   operator explicitly opts in via `RIMLOC_LEGACY_COMMANDS=1`
//!   (lead decision 033 #4). [`CONTRACT_COMMANDS`], [`registration_plan`]
//!   and the unit tests in src/tests.rs are the enforcement evidence.
//!
//! The legacy command implementations live in the binary (main.rs); the
//! combined `generate_handler!` lists therefore also live there, bound to
//! the plan constants through the unit tests.

use rimloc_services::contract::{
    capability_report, ui_contract_version, ApplyIntentsRequest, ApplyIntentsResponse,
    CapabilityReport, ChatBatchApplyRequest, ChatBatchApplyResponse, ChatBatchCreateRequest,
    ChatBatchExportRequest, ChatBatchExportResponse, ChatBatchImportRequest,
    ChatBatchImportResponse, ChatBatchListRequest, ChatBatchListResponse, ChatBatchStatusRequest,
    ChatBatchStatusResponse, CreateProjectRequest, ProjectGlossaryDeleteRequest,
    ProjectGlossaryDeleteResponse, ProjectGlossaryUpsertRequest, ProjectGlossaryUpsertResponse,
    ProjectSnapshot, ProjectSummary, ProviderInstanceDeleteRequest, ProviderInstanceDeleteResponse,
    ProviderInstanceListResponse, ProviderInstanceUpsertRequest, ProviderInstanceUpsertResponse,
    ProviderInstanceValidateRequest, ProviderInstanceValidateResponse, TmDeleteRequest,
    TmDeleteResponse, TmImportRequest, TmImportResponse, TmListRequest, TmListResponse,
    TmLookupRequest, TmLookupResponse, TmUpsertRequest, TmUpsertResponse,
};
use rimloc_services::session::ProjectSessionManager;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// Contract commands registered in EVERY entry (live and legacy opt-in).
pub const CONTRACT_COMMANDS: &[&str] = &[
    "contract_handshake",
    "project_create",
    "project_open",
    "project_list",
    "project_snapshot",
    "project_apply_intents",
    "project_refresh",
    "project_cancel_next",
    // Final night wave: validate/build/diagnostics over the contract
    // (services landed in 47758cb; this registration is the transport).
    "project_validate",
    "project_export",
    "project_output_xml_preview",
    "project_build_mod",
    "project_diagnose",
    // W2 (existing-pack flow): dry-run analysis + guarded application of an
    // existing translation pack against the open project.
    "project_import_existing",
    "project_apply_existing",
    // UI-10 (file-format parity): the project-exchange PO — export the
    // durable translations and analyze a PO pack through the SAME
    // existing-pack review/apply flow.
    "project_export_po",
    "project_import_po",
    "project_apply_po",
    "project_export_xliff",
    "project_import_xliff",
    "project_apply_xliff",
    // Wave 13: project glossary — generic project state, persist-before-ack.
    "project_glossary",
    "project_glossary_upsert",
    "project_glossary_delete",
    // TM live (owner decision A+B+C): translation memory — the glossary
    // pattern again (Project.tm, persist-before-ack).
    "project_tm_list",
    "project_tm_upsert",
    "project_tm_delete",
    "project_tm_import",
    "project_tm_lookup",
    // Durable draft autosave (Data Safety): sidecar store of UNSAVED editor
    // work — epoch-guarded, revision-stamped, never confirmed translations.
    "project_save_drafts",
    "project_restore_drafts",
    "project_clear_drafts",
    // AI translation jobs (Stage B): start/status/cancel/accept.
    "ai_job_start",
    "ai_job_status",
    "ai_job_cancel",
    "ai_job_accept",
    "ai_job_accept_selected",
    // Source Discovery (Stage A): read-only full-project source search with
    // canonical-inventory reconciliation.
    "project_source_search",
    // UI-11 Stage C: guarded per-candidate inclusion of a missed source
    // row into the managed project state (never the source mod).
    "project_include_candidate",
    // Provider instances (provider/settings parity): app-global CRUD; the
    // API key NEVER crosses into a file — it goes to the OS keychain.
    "contract_provider_instance_list",
    "contract_provider_instance_upsert",
    "contract_provider_instance_delete",
    "contract_provider_instance_validate",
    // Bounded connectivity probe (§7 F7.1/F7.2): the CLI `provider-test`
    // logic reused by the GUI — typed outcome, one tiny prompt, no batch.
    "contract_provider_instance_test",
    // Compare screen (version diff): read-only source-inventory diff of two
    // mod roots — stateless, no project session involved.
    "contract_version_diff",
    // Chat batch (external-AI workflow WITHOUT an API): select → export
    // prompt → paste response → strict parse → preview → guarded apply
    // (origin=import). Batch state persists in the managed store. The list
    // op is the batch history (mandate §27).
    "contract_chat_batch_create",
    "contract_chat_batch_export",
    "contract_chat_batch_import",
    "contract_chat_batch_status",
    "contract_chat_batch_apply",
    "contract_chat_batch_list",
];

/// Default managed-projects root: `<app-data>/managed`
/// (identifier `com.rimloc.gui` from tauri.conf).
///
/// Test isolation (owner directive §4, 2026-10-01): automation instances
/// launched with RIMLOC_DATA_DIR=<disposable root> get a fully separate
/// projects universe — owner managed projects/recents are absent from the
/// instance's discovery root entirely. Production never sets the variable;
/// the write fence in the T6 harness (§3) is the second layer.
pub fn default_managed_root() -> PathBuf {
    if let Ok(dir) = std::env::var("RIMLOC_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("managed");
        }
    }
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("com.rimloc.gui")
        .join("managed")
}

/// Managed state for the contract commands. The session manager is
/// thread-safe internally; the mutex only guards Tauri's unmanaged access.
pub struct ContractState {
    manager: Mutex<ProjectSessionManager>,
}

/// Session manager construction for the shell: with the `keychain` feature
/// the manager gets the OS-keychain secret sink (provider API keys), without
/// it the manager stays sink-less and secret-bearing provider upserts are
/// refused honestly by the services layer.
fn build_session_manager(managed_root: PathBuf) -> std::io::Result<ProjectSessionManager> {
    #[cfg(feature = "keychain")]
    {
        let sink = std::sync::Arc::new(
            rimloc_services::providers::KeychainSecretSink::with_default_service(),
        );
        ProjectSessionManager::new_with_secret_sink(managed_root, sink)
    }
    #[cfg(not(feature = "keychain"))]
    {
        ProjectSessionManager::new(managed_root)
    }
}

impl ContractState {
    pub fn new(managed_root: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            manager: Mutex::new(build_session_manager(managed_root)?),
        })
    }

    /// Run one operation against the session manager (the ONE state seam;
    /// the manager serializes per project internally). Used by the shell
    /// commands outside `contract_adapter` that need session services —
    /// business logic stays in `rimloc-services`, the adapter owns state
    /// and wire-shape only.
    pub fn with_manager<R>(&self, f: impl FnOnce(&ProjectSessionManager) -> R) -> R {
        let manager = self
            .manager
            .lock()
            .expect("contract session registry poisoned");
        f(&manager)
    }

    #[cfg(test)]
    fn with_root_for_tests(root: &std::path::Path) -> std::io::Result<Self> {
        Self::new(root.to_path_buf())
    }
}

/// Agent trace (RIMLOC_TRACE=1): command name + duration + ok/error —
/// payload details deliberately stay out of the log.
fn traced_simple<T, E>(cmd: &'static str, body: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    crate::trace::traced(cmd, body, |r| {
        if r.is_ok() { "ok" } else { "error" }.to_string()
    })
}

/// ui_contract_version + honest capability report (supported vs unsupported
/// slice operations). The client compares the version on handshake and
/// raises a typed mismatch error when it differs.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ContractHandshake {
    pub ui_contract_version: u32,
    pub capabilities: CapabilityReport,
}

#[tauri::command(rename_all = "snake_case")]
pub fn contract_handshake() -> ContractHandshake {
    ContractHandshake {
        ui_contract_version: ui_contract_version(),
        capabilities: capability_report(),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_create(
    state: State<'_, ContractState>,
    request: CreateProjectRequest,
) -> Result<ProjectSnapshot, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_create", || {
        manager.create_with_source(
            std::path::Path::new(&request.mod_root.path),
            request.target_version.as_deref(),
            request.source_locale.as_deref(),
            request.source_dir.as_deref(),
        )
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_open(
    state: State<'_, ContractState>,
    project_id: String,
) -> Result<ProjectSnapshot, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_open", || manager.open(&project_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_list(state: State<'_, ContractState>) -> Vec<ProjectSummary> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    manager.list()
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_snapshot(
    state: State<'_, ContractState>,
    project_id: String,
) -> Result<ProjectSnapshot, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    manager.snapshot(&project_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_apply_intents(
    state: State<'_, ContractState>,
    request: ApplyIntentsRequest,
) -> Result<ApplyIntentsResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    manager.apply(&request)
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_refresh(
    state: State<'_, ContractState>,
    project_id: String,
) -> Result<ProjectSnapshot, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    manager.refresh(&project_id)
}

/// `project_glossary` — the project's terms (wave 13, read-only).
#[tauri::command(rename_all = "snake_case")]
pub fn project_glossary(
    state: State<'_, ContractState>,
    project_id: String,
    session_epoch: u64,
) -> Result<Vec<rimloc_domain::glossary::GlossaryTerm>, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_glossary", || {
        manager.glossary_list(&project_id, session_epoch)
    })
}

/// `project_glossary_upsert` — create/update one term (case-insensitive
/// `term` match); persist-before-ack.
#[tauri::command(rename_all = "snake_case")]
pub fn project_glossary_upsert(
    state: State<'_, ContractState>,
    request: ProjectGlossaryUpsertRequest,
) -> Result<ProjectGlossaryUpsertResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_glossary_upsert", || {
        manager.glossary_upsert(&request)
    })
}

/// `project_glossary_delete` — remove one term; unknown term is a typed
/// refusal, never silent success.
#[tauri::command(rename_all = "snake_case")]
pub fn project_glossary_delete(
    state: State<'_, ContractState>,
    request: ProjectGlossaryDeleteRequest,
) -> Result<ProjectGlossaryDeleteResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_glossary_delete", || {
        manager.glossary_delete(&request)
    })
}

// Durable draft autosave (Data Safety): pass-through into the session
// manager's sidecar-store ops — epoch-guarded, revision-stamped, typed
// errors. Store keys are FRONTEND-owned (`<locale>::<display_identity()>`);
// the adapter treats them as opaque strings.

/// `project_save_drafts` — persist the whole drafts map (the debounced
/// autosave write; replace semantics are the store's contract).
#[tauri::command(rename_all = "snake_case")]
pub fn project_save_drafts(
    state: State<'_, ContractState>,
    project_id: String,
    session_epoch: u64,
    entries: std::collections::BTreeMap<String, rimloc_services::DraftEntry>,
) -> Result<rimloc_services::DraftsSaveAck, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_save_drafts", || {
        manager.save_drafts_session(&project_id, session_epoch, entries)
    })
}

/// `project_restore_drafts` — honest restore report after open(): never a
/// silent apply; a revision mismatch quarantines the store as an orphan.
#[tauri::command(rename_all = "snake_case")]
pub fn project_restore_drafts(
    state: State<'_, ContractState>,
    project_id: String,
    session_epoch: u64,
) -> Result<rimloc_services::DraftsRestoreReport, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_restore_drafts", || {
        manager.restore_drafts_session(&project_id, session_epoch)
    })
}

/// `project_clear_drafts` — remove one key (revert / commit ack) or the
/// whole store; removing an absent key is an honest no-op.
#[tauri::command(rename_all = "snake_case")]
pub fn project_clear_drafts(
    state: State<'_, ContractState>,
    project_id: String,
    session_epoch: u64,
    key: Option<String>,
) -> Result<rimloc_services::DraftsSaveAck, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_clear_drafts", || {
        manager.clear_drafts_session(&project_id, session_epoch, key.as_deref())
    })
}

// AI translation jobs (Stage B) — thin typed adapter over the shared
// services::ai_job registry (one engine, one canonical writer). State lives
// per app process; the durable record is the canonical apply + checkpoints.

use rimloc_services::ai_job::{AiJobParams, AiJobRegistry};

/// Managed root shared with the ai job service for checkpoint placement.
pub struct ManagedRoot(pub PathBuf);

pub struct AiJobs(pub AiJobRegistry);

/// `project_include_candidate` (UI-11 Stage C) — guarded per-candidate
/// human approval: include ONE missed source row into the MANAGED
/// project state. Every UI input is re-verified server-side (view
/// membership, XML validity, the row's own element/text, NoTranslate,
/// duplicate identity, source fingerprint); persist-before-ack with a
/// revision bump; the source mod is never written.
#[tauri::command(rename_all = "snake_case")]
pub fn project_include_candidate(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::IncludeCandidateRequest,
) -> Result<
    rimloc_services::contract::IncludeCandidateResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_include_candidate", || {
        manager.include_candidate(&request)
    })
}

/// `project_source_search` (Source Discovery, Stage A) — READ-ONLY
/// full-project source search reconciled against the canonical inventory.
/// The search context (mod root, versions, source language) comes from the
/// LIVE session snapshot; the epoch guard protects against superseded
/// sessions. v1 runs synchronously; UI-level cancellation arrives with the
/// streaming wiring (documented limitation).
#[tauri::command(rename_all = "snake_case")]
pub fn project_source_search(
    state: State<'_, ContractState>,
    request: SourceSearchArgs,
) -> Result<
    rimloc_services::source_search::SourceSearchReport,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_source_search", || {
        let snap = manager.snapshot(&request.project_id)?;
        if snap.session_epoch != request.session_epoch {
            return Err(rimloc_services::contract::ContractError::stale_epoch(
                request.session_epoch,
                snap.session_epoch,
            ));
        }
        let root = snap
            .source_root
            .as_ref()
            .map(|r| std::path::PathBuf::from(&r.path))
            .ok_or_else(|| {
                rimloc_services::contract::ContractError::new(
                    rimloc_services::contract::ContractErrorCode::InvalidConfig,
                    "project has no source root — source search needs the source mod",
                )
            })?;
        let scope = match request.scope.as_deref() {
            Some("all_text") => rimloc_services::source_search::SearchScope::AllText,
            _ => rimloc_services::source_search::SearchScope::Standard,
        };
        let req = rimloc_services::source_search::SourceSearchRequest {
            mod_root: root,
            target_version: snap.project.context.target_version.clone(),
            source_lang_dir: snap.project.context.source_dir.clone(),
            query: request.query,
            scope,
            offset: request.offset,
            max_results: request.max_results,
            // Stage C reconciliation: rows the human recovered into the
            // managed state classify as indexed on the next search.
            manual_ids: snap.project.entries.iter().map(|e| e.id.clone()).collect(),
        };
        rimloc_services::source_search::search_sources(&req, &|| false)
    })
}

/// `project_source_xml_preview` (rel27) — read-only bounded window of a
/// project source file for the Source Inspector XML preview. The path is
/// resolved against the session's source root with containment (no `..`
/// traversal, absolute paths refused); lines are returned 1-based.
#[derive(Debug, serde::Deserialize)]
pub struct SourceXmlPreviewArgs {
    pub project_id: String,
    pub session_epoch: u64,
    pub file: String,
    #[serde(default)]
    pub line: Option<usize>,
    #[serde(default)]
    pub before: usize,
    #[serde(default)]
    pub after: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct SourceXmlPreviewLine {
    pub n: usize,
    pub text: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SourceXmlPreview {
    pub file: String,
    pub target_line: Option<usize>,
    pub lines: Vec<SourceXmlPreviewLine>,
    pub truncated_head: bool,
    pub truncated_tail: bool,
    pub total_lines: usize,
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_source_xml_preview(
    state: State<'_, ContractState>,
    request: SourceXmlPreviewArgs,
) -> Result<SourceXmlPreview, rimloc_services::contract::ContractError> {
    use rimloc_services::contract::{ContractError, ContractErrorCode};
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_source_xml_preview", || {
        let snap = manager.snapshot(&request.project_id)?;
        if snap.session_epoch != request.session_epoch {
            return Err(ContractError::stale_epoch(
                request.session_epoch,
                snap.session_epoch,
            ));
        }
        let root = snap
            .source_root
            .as_ref()
            .map(|r| std::path::PathBuf::from(&r.path))
            .ok_or_else(|| {
                ContractError::new(
                    ContractErrorCode::InvalidConfig,
                    "project has no source root",
                )
            })?;
        let rel = std::path::Path::new(&request.file);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root".to_string(),
            ));
        }
        let full = root.join(rel);
        if !full.starts_with(&root) {
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "source preview path escapes the project source root".to_string(),
            ));
        }
        let bytes = std::fs::read(&full).map_err(|source| {
            ContractError::new(
                ContractErrorCode::InvalidConfig,
                format!("source preview read `{}`: {source}", request.file),
            )
        })?;
        if bytes.iter().take(8192).any(|b| *b == 0) {
            return Err(ContractError::new(
                ContractErrorCode::InvalidConfig,
                "source preview: file looks binary".to_string(),
            ));
        }
        let content = String::from_utf8_lossy(&bytes);
        let total = content.lines().count();
        let target = request.line.unwrap_or(1).min(total.max(1));
        let before = request.before.min(200);
        let after = request.after.min(200);
        let start = target.saturating_sub(before).max(1);
        let end = (target + after).min(total);
        let lines = content
            .lines()
            .skip(start - 1)
            .take(end - start + 1)
            .enumerate()
            .map(|(i, text)| SourceXmlPreviewLine {
                n: start + i,
                text: text.to_string(),
            })
            .collect();
        Ok(SourceXmlPreview {
            file: request.file,
            target_line: request.line,
            lines,
            truncated_head: start > 1,
            truncated_tail: end < total,
            total_lines: total,
        })
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn ai_job_start(
    state: State<'_, ContractState>,
    root: State<'_, ManagedRoot>,
    ai: State<'_, AiJobs>,
    request: AiJobParamsWire,
) -> Result<rimloc_services::ai_job::AiJobStatus, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("ai_job_start", || {
        let mut params = request.into_params();
        // The project's REAL source locale rides the params (never a
        // hardcoded "en"): the adapter resolves it from the live snapshot.
        let snap = manager.snapshot(&params.project_id)?;
        params.source_locale = snap
            .project
            .context
            .source_locale
            .clone()
            .unwrap_or_else(|| "en".to_string());
        rimloc_services::ai_job::start_job(&ai.0, &manager, &managed_root_of(&root), params)
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn ai_job_status(
    ai: State<'_, AiJobs>,
    root: State<'_, ManagedRoot>,
    job_id: String,
) -> Result<rimloc_services::ai_job::AiJobStatus, rimloc_services::contract::ContractError> {
    // Registry first, durable record second: a job asked about after an
    // app restart answers with its last persisted truth (interrupted /
    // terminal), never a fake "job not found".
    rimloc_services::ai_job::job_status_with_record(&ai.0, &root.0, &job_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn ai_job_cancel(
    ai: State<'_, AiJobs>,
    job_id: String,
) -> Result<rimloc_services::ai_job::AiJobStatus, rimloc_services::contract::ContractError> {
    rimloc_services::ai_job::cancel_job(&ai.0, &job_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn ai_job_accept(
    state: State<'_, ContractState>,
    ai: State<'_, AiJobs>,
    request: AiJobAcceptWire,
) -> Result<rimloc_services::ai_job::AiJobStatus, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("ai_job_accept", || {
        rimloc_services::ai_job::accept_job(
            &ai.0,
            &manager,
            &request.job_id,
            request.session_epoch,
            request.acknowledge_full_scope,
        )
    })
}

/// `ai_job_accept_selected` — per-row review: apply ONLY the selected
/// preview rows; rejected rows are marked in the preview ledger and the
/// rest stays reviewable. Same pinned-revision / stale-epoch protections
/// as the full accept.
#[tauri::command(rename_all = "snake_case")]
pub fn ai_job_accept_selected(
    state: State<'_, ContractState>,
    ai: State<'_, AiJobs>,
    request: AiJobAcceptSelectedWire,
) -> Result<rimloc_services::ai_job::AiJobStatus, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    rimloc_services::ai_job::accept_selected(
        &ai.0,
        &manager,
        &request.job_id,
        request.session_epoch,
        &request.selected,
        &request.rejected,
    )
}

#[derive(Debug, serde::Deserialize)]
pub struct AiJobParamsWire {
    pub project_id: String,
    pub session_epoch: u64,
    pub locale: String,
    pub provider_id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    /// ProvidersScreen instance (`prov-*`): when set, the instance's
    /// preset/model/base_url/key define the launch target and the direct
    /// provider fields are ignored. The KEY never rides this wire — it is
    /// resolved in-process from the OS keychain sink.
    #[serde(default)]
    pub provider_instance_id: Option<String>,
    #[serde(default)]
    pub batch_char_budget: usize,
}

impl AiJobParamsWire {
    fn into_params(self) -> AiJobParams {
        AiJobParams {
            project_id: self.project_id,
            source_locale: String::new(),
            session_epoch: self.session_epoch,
            locale: self.locale,
            provider_id: self.provider_id,
            model: self.model,
            base_url: self.base_url,
            provider_instance_id: self.provider_instance_id,
            batch_char_budget: self.batch_char_budget,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct AiJobAcceptWire {
    pub job_id: String,
    pub session_epoch: u64,
    /// Full-scope acknowledgement for truncated previews (> 400 rows):
    /// the caller CONFIRMS applying rows the bounded preview did not
    /// show. Defaults to false — a silent full apply is impossible.
    #[serde(default)]
    pub acknowledge_full_scope: bool,
}

/// Wire args of `ai_job_accept_selected` (per-row review).
#[derive(Debug, serde::Deserialize)]
pub struct AiJobAcceptSelectedWire {
    pub job_id: String,
    pub session_epoch: u64,
    /// Identities to apply (row `identity` renderings).
    #[serde(default)]
    pub selected: Vec<String>,
    /// Identities to mark rejected (kept in the preview ledger).
    #[serde(default)]
    pub rejected: Vec<String>,
}

/// Wire args of `project_source_search`.
#[derive(Debug, serde::Deserialize)]
pub struct SourceSearchArgs {
    pub project_id: String,
    pub session_epoch: u64,
    pub query: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub max_results: usize,
}

// TM live (owner decision A+B+C): the glossary command chain, repeated
// literally — pass-through into the session manager, typed errors, trace.

/// `project_tm_list` — TM records with optional locale/status/query
/// filters (read-only).
#[tauri::command(rename_all = "snake_case")]
pub fn project_tm_list(
    state: State<'_, ContractState>,
    request: TmListRequest,
) -> Result<TmListResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_tm_list", || manager.tm_list(&request))
}

/// `project_tm_upsert` — manual CRUD write (C); persist-before-ack.
#[tauri::command(rename_all = "snake_case")]
pub fn project_tm_upsert(
    state: State<'_, ContractState>,
    request: TmUpsertRequest,
) -> Result<TmUpsertResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_tm_upsert", || manager.tm_upsert(&request))
}

/// `project_tm_delete` — remove by stable id; unknown id is a typed
/// refusal, never silent success.
#[tauri::command(rename_all = "snake_case")]
pub fn project_tm_delete(
    state: State<'_, ContractState>,
    request: TmDeleteRequest,
) -> Result<TmDeleteResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_tm_delete", || manager.tm_delete(&request))
}

/// `project_tm_import` — bulk import (B), JSON or CSV; persist-before-ack.
#[tauri::command(rename_all = "snake_case")]
pub fn project_tm_import(
    state: State<'_, ContractState>,
    request: TmImportRequest,
) -> Result<TmImportResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_tm_import", || manager.tm_import(&request))
}

/// `project_tm_lookup` — ranked candidates within one target locale
/// (read-only).
#[tauri::command(rename_all = "snake_case")]
pub fn project_tm_lookup(
    state: State<'_, ContractState>,
    request: TmLookupRequest,
) -> Result<TmLookupResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_tm_lookup", || manager.tm_lookup(&request))
}

// Provider instances (provider/settings parity): the TM chain again —
// pass-through into the session manager, typed errors, trace. The adapter
// never sees the secret value (it rides inside the request DTO once and is
// manual-Debug-masked); `traced_simple` logs only the command + ok/error.

/// `provider_instance_list` — redacted summaries (`has_key`, no secret).
#[tauri::command(rename_all = "snake_case")]
pub fn contract_provider_instance_list(
    state: State<'_, ContractState>,
) -> Result<ProviderInstanceListResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_provider_instance_list", || {
        manager.provider_instance_list()
    })
}

/// `provider_instance_upsert` — create/edit; the key goes to the OS
/// keychain, the metadata to the settings file (persist-before-ack).
#[tauri::command(rename_all = "snake_case")]
pub fn contract_provider_instance_upsert(
    state: State<'_, ContractState>,
    request: ProviderInstanceUpsertRequest,
) -> Result<ProviderInstanceUpsertResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_provider_instance_upsert", || {
        manager.provider_instance_upsert(&request)
    })
}

/// `provider_instance_delete` — keychain key first, then metadata.
#[tauri::command(rename_all = "snake_case")]
pub fn contract_provider_instance_delete(
    state: State<'_, ContractState>,
    request: ProviderInstanceDeleteRequest,
) -> Result<ProviderInstanceDeleteResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_provider_instance_delete", || {
        manager.provider_instance_delete(&request)
    })
}

/// `provider_instance_validate` — typed form validation, no network.
#[tauri::command(rename_all = "snake_case")]
pub fn contract_provider_instance_validate(
    state: State<'_, ContractState>,
    request: ProviderInstanceValidateRequest,
) -> Result<ProviderInstanceValidateResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_provider_instance_validate", || {
        manager.provider_instance_validate(&request)
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn project_cancel_next(
    state: State<'_, ContractState>,
    project_id: String,
) -> Result<bool, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    manager.cancel_next(&project_id)
}

/// `project_validate` — the EXISTING validator over the trusted session
/// state; read-only, never mutates the project. `session_epoch` guards
/// against stale callers; `locale` is an in-memory filter.
#[tauri::command(rename_all = "snake_case")]
pub fn project_validate(
    state: State<'_, ContractState>,
    project_id: String,
    session_epoch: u64,
    locale: Option<String>,
) -> Result<
    rimloc_services::contract::ValidateProjectResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_validate", || {
        manager.validate_project(&project_id, session_epoch, locale.as_deref())
    })
}

/// `project_export` — isolated native output into a CALLER-SPECIFIED out
/// directory; the services guard refuses source-tree/managed-root targets
/// (guard_output_denied) and the result is reparse-verified before ack.
#[tauri::command(rename_all = "snake_case")]
pub fn project_export(
    state: State<'_, ContractState>,
    reveal: State<'_, crate::reveal::RevealState>,
    project_id: String,
    session_epoch: u64,
    out_dir: String,
    locale: String,
) -> Result<
    rimloc_services::contract::ExportProjectResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    let ack = traced_simple("project_export", || {
        manager.export_project(
            &project_id,
            session_epoch,
            std::path::Path::new(&out_dir),
            &locale,
        )
    })?;
    // §8 F8.4: ONLY a successful ack blesses its out_dir for this session's
    // `reveal_path` shell command — the services guard + reparse check have
    // already validated the real location by the time this line runs.
    reveal.allow(std::path::Path::new(&ack.out_dir.path));
    Ok(ack)
}

/// `project_output_xml_preview` — UI-05 (owner audit Oct-14): the
/// CANONICAL serializer rendered in-memory (scratch dir in the OS temp,
/// removed best-effort). Read-only toward user-visible locations; the
/// response carries the produced XML files for the requested locale.
#[tauri::command(rename_all = "snake_case")]
pub fn project_output_xml_preview(
    state: State<'_, ContractState>,
    project_id: String,
    session_epoch: u64,
    locale: String,
) -> Result<
    rimloc_services::contract::OutputXmlPreviewResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_output_xml_preview", || {
        manager.project_output_xml_preview(&project_id, session_epoch, &locale)
    })
}

/// `project_build_mod` — the FULL drop-in mod package (`About/About.xml` in
/// the game-loadable `<ModMetaData>` shape + `Languages/<locale>`) into a
/// CALLER-SPECIFIED out directory; the guard partition is identical to
/// `project_export` (source-tree/managed-root denies, absolute form) and
/// the result is reparse-verified before the ack.
#[tauri::command(rename_all = "snake_case")]
pub fn project_build_mod(
    state: State<'_, ContractState>,
    reveal: State<'_, crate::reveal::RevealState>,
    project_id: String,
    session_epoch: u64,
    out_dir: String,
    locale: String,
) -> Result<
    rimloc_services::contract::BuildModProjectResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    let ack = traced_simple("project_build_mod", || {
        manager.build_mod_project(
            &project_id,
            session_epoch,
            std::path::Path::new(&out_dir),
            &locale,
        )
    })?;
    // §8 F8.4: same session reveal blessing as project_export.
    reveal.allow(std::path::Path::new(&ack.out_dir.path));
    Ok(ack)
}

/// `project_diagnose` — sanitized support bundle over the project's last
/// failed operation; the collector enforces the out-of-source-tree guard.
#[tauri::command(rename_all = "snake_case")]
pub fn project_diagnose(
    state: State<'_, ContractState>,
    project_id: String,
    out_dir: String,
) -> Result<rimloc_services::contract::DiagnoseResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    manager.diagnose(&project_id, std::path::Path::new(&out_dir))
}

/// `project_import_existing` — DRY-RUN analysis of an existing translation
/// pack against the open project; read-only (the services guard refuses
/// relative paths, non-directories and managed-root targets).
#[tauri::command(rename_all = "snake_case")]
pub fn project_import_existing(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ImportExistingRequest,
) -> Result<
    rimloc_services::contract::ImportExistingResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_import_existing", || {
        manager.import_existing(&request)
    })
}

/// `project_apply_existing` — apply the REUSABLE set of an analyzed pack
/// into the open project; persist-before-ack, existing translations never
/// overwritten, ambiguous lines never auto-applied.
#[tauri::command(rename_all = "snake_case")]
pub fn project_apply_existing(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ApplyExistingRequest,
) -> Result<
    rimloc_services::contract::ApplyExistingResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_apply_existing", || {
        manager.apply_existing(&request)
    })
}

/// `project_export_po` (UI-10) — the project-exchange PO: msgid is the
/// canonical source text, msgstr the CURRENT durable translation of the
/// requested locale. The write is guarded like every export (absolute
/// path, never inside the read-only source tree or the managed root).
#[tauri::command(rename_all = "snake_case")]
pub fn project_export_po(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ExportProjectPoRequest,
) -> Result<
    rimloc_services::contract::ExportProjectPoResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_export_po", || {
        let out_po = std::path::PathBuf::from(&request.out_po.path);
        manager.export_project_po(
            &request.project_id,
            request.session_epoch,
            &out_po,
            &request.locale,
        )
    })
}

/// `project_import_po` (UI-10) — analyze an existing-translation PO FILE
/// through the same temp-pack rebuild + analyzer as the folder pack, so
/// the review UI and the guarded apply stay identical.
#[tauri::command(rename_all = "snake_case")]
pub fn project_import_po(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ImportExistingPoRequest,
) -> Result<
    rimloc_services::contract::ImportExistingResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_import_po", || manager.import_existing_po(&request))
}

/// `project_apply_po` (UI-10) — apply the REUSABLE set of an analyzed PO
/// pack; persist-before-ack, identical safety semantics to the folder
/// apply (conflicts never overwritten, ambiguous never auto-applied).
#[tauri::command(rename_all = "snake_case")]
pub fn project_apply_po(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ApplyExistingPoRequest,
) -> Result<
    rimloc_services::contract::ApplyExistingResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_apply_po", || manager.apply_existing_po(&request))
}

/// `project_export_xliff` (UI-10) — the XLIFF mirror of the exchange PO.
#[tauri::command(rename_all = "snake_case")]
pub fn project_export_xliff(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ExportProjectXliffRequest,
) -> Result<
    rimloc_services::contract::ExportProjectXliffResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_export_xliff", || {
        let out = std::path::PathBuf::from(&request.out_xliff.path);
        manager.export_project_xliff(
            &request.project_id,
            request.session_epoch,
            &out,
            &request.locale,
        )
    })
}

/// `project_import_xliff` (UI-10) — analyze an XLIFF pack through the
/// same canonical-tree rebuild + analyzer as the folder and PO packs.
#[tauri::command(rename_all = "snake_case")]
pub fn project_import_xliff(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ImportExistingXliffRequest,
) -> Result<
    rimloc_services::contract::ImportExistingResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_import_xliff", || {
        manager.import_existing_xliff(&request)
    })
}

/// `project_apply_xliff` (UI-10) — apply the REUSABLE set of an analyzed
/// XLIFF pack; identical safety semantics to the folder and PO applies.
#[tauri::command(rename_all = "snake_case")]
pub fn project_apply_xliff(
    state: State<'_, ContractState>,
    request: rimloc_services::contract::ApplyExistingXliffRequest,
) -> Result<
    rimloc_services::contract::ApplyExistingResponse,
    rimloc_services::contract::ContractError,
> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("project_apply_xliff", || {
        manager.apply_existing_xliff(&request)
    })
}

// Chat batch (external-AI workflow WITHOUT an API): the TM chain again —
// pass-through into the session manager, typed errors, trace. The parser,
// gates and persistence live in `rimloc-services::chat_batch`; the apply
// goes through `session.apply` (origin=import, persist-before-ack).

/// `contract_chat_batch_create` — pin a selection into a fresh batch.
#[tauri::command(rename_all = "snake_case")]
pub fn contract_chat_batch_create(
    state: State<'_, ContractState>,
    request: ChatBatchCreateRequest,
) -> Result<rimloc_services::contract::ChatBatch, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_chat_batch_create", || {
        manager.chat_batch_create(&request)
    })
}

/// `contract_chat_batch_export` — build the structured prompt; records the
/// revision + source hash gates and persists the batch `exported`.
#[tauri::command(rename_all = "snake_case")]
pub fn contract_chat_batch_export(
    state: State<'_, ContractState>,
    request: ChatBatchExportRequest,
) -> Result<ChatBatchExportResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_chat_batch_export", || {
        manager.chat_batch_export(&request)
    })
}

/// `contract_chat_batch_import` — strict-parse the pasted response into the
/// pending preview (identity/revision/source-hash gates; whole-import
/// refusals, never partial).
#[tauri::command(rename_all = "snake_case")]
pub fn contract_chat_batch_import(
    state: State<'_, ContractState>,
    request: ChatBatchImportRequest,
) -> Result<ChatBatchImportResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_chat_batch_import", || {
        manager.chat_batch_import(&request)
    })
}

/// `contract_chat_batch_status` — the durable batch + the live drift
/// verdict (pure read).
#[tauri::command(rename_all = "snake_case")]
pub fn contract_chat_batch_status(
    state: State<'_, ContractState>,
    request: ChatBatchStatusRequest,
) -> Result<ChatBatchStatusResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_chat_batch_status", || {
        manager.chat_batch_status(&request)
    })
}

/// `contract_chat_batch_apply` — the human-confirmed apply: the stored
/// preview becomes `ApplyIntentsRequest` with `origin=import` through
/// `session.apply` (persist-before-ack).
#[tauri::command(rename_all = "snake_case")]
pub fn contract_chat_batch_apply(
    state: State<'_, ContractState>,
    request: ChatBatchApplyRequest,
) -> Result<ChatBatchApplyResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_chat_batch_apply", || {
        manager.chat_batch_apply(&request)
    })
}

/// `contract_chat_batch_list` — the batch history (mandate §27): every
/// durable batch of ONE project, oldest first (read-only).
#[tauri::command(rename_all = "snake_case")]
pub fn contract_chat_batch_list(
    state: State<'_, ContractState>,
    request: ChatBatchListRequest,
) -> Result<ChatBatchListResponse, rimloc_services::contract::ContractError> {
    let manager = state
        .manager
        .lock()
        .expect("contract session registry poisoned");
    traced_simple("contract_chat_batch_list", || {
        manager.chat_batch_list_op(&request)
    })
}

/// `contract_version_diff` — read-only cross-version diff of two mod roots
/// (compare screen). Stateless: no project session, no epoch/revision —
/// the services scan pipeline classifies new/missing/changed/unchanged per
/// key. The adapter owns the path-FORM guards BEFORE any filesystem access
/// (the `existing_pack_dir` partition): absolute form, directory, and
/// never inside the managed-projects root.
#[tauri::command(rename_all = "snake_case")]
pub fn contract_version_diff(
    request: rimloc_services::contract::VersionDiffRequest,
) -> Result<rimloc_services::contract::VersionDiffResponse, rimloc_services::contract::ContractError>
{
    use rimloc_services::contract::{ContractError, ContractErrorCode};
    traced_simple("contract_version_diff", || {
        let managed_root = default_managed_root();
        let roots = [
            ("old", std::path::PathBuf::from(&request.old_root.path)),
            ("new", std::path::PathBuf::from(&request.new_root.path)),
        ];
        for (role, dir) in &roots {
            if !dir.is_absolute() {
                return Err(ContractError::new(
                    ContractErrorCode::InvalidOutputPath,
                    format!(
                        "version diff {role} root `{}` is not absolute; specify an absolute directory",
                        dir.display()
                    ),
                ));
            }
            if rimloc_services::is_within(dir, &managed_root) {
                return Err(ContractError::new(
                    ContractErrorCode::GuardOutputDenied,
                    format!(
                        "version diff {role} root `{}` is inside the managed projects root `{}`",
                        dir.display(),
                        managed_root.display()
                    ),
                ));
            }
            if !dir.is_dir() {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!(
                        "version diff {role} root `{}` is not a directory",
                        dir.display()
                    ),
                ));
            }
        }
        let report = rimloc_services::extras::version_diff::version_diff_scan(
            &roots[0].1,
            &roots[1].1,
            &rimloc_services::extras::version_diff::VersionDiffScanOptions {
                source_lang: request.source_lang.clone(),
                from_label: None,
                to_label: None,
            },
        )
        .map_err(|e| {
            // The scan pipeline reports color_eyre errors; the contract
            // surface carries them as the typed `internal` code with the
            // (already sanitized) display text.
            ContractError::new(ContractErrorCode::Internal, format!("{e}"))
        })?;
        Ok(rimloc_services::contract::VersionDiffResponse::from_report(
            report,
        ))
    })
}

/// Manage the contract state on a builder. Returns the same builder type;
/// runtime-generic so tests can use the mock runtime.
pub fn attach_contract<C: tauri::Runtime>(
    builder: tauri::Builder<C>,
    managed_root: PathBuf,
) -> Result<tauri::Builder<C>, std::io::Error> {
    let state = ContractState::new(managed_root.clone())?;
    Ok(builder
        .manage(state)
        .manage(ManagedRoot(managed_root))
        .manage(AiJobs(AiJobRegistry::new())))
}

/// Project-relative path of the managed root (for the ai job service).
pub fn managed_root_of(state: &State<'_, ManagedRoot>) -> PathBuf {
    state.0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    #[test]
    fn handshake_reports_version_one_and_capabilities() {
        let hs = contract_handshake();
        assert_eq!(hs.ui_contract_version, ui_contract_version());
        // Final night wave: validate/build/diagnostics ARE supported now —
        // asserted in the WIRE form (serde snake_case), not Debug.
        let supported: Vec<String> = hs
            .capabilities
            .supported
            .iter()
            .map(|c| {
                serde_json::to_value(c)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        for cap in [
            "project_validate",
            "project_build_export",
            "project_build_mod",
            "project_diagnostics_bundle",
        ] {
            assert!(
                supported.iter().any(|s| s == cap),
                "capability {cap} must be supported"
            );
        }
        // …and the slice boundary stays honest about what is NOT here.
        assert!(!hs.capabilities.unsupported.is_empty());
    }

    #[test]
    fn contract_commands_all_start_with_contract_or_project_prefix() {
        for name in CONTRACT_COMMANDS {
            assert!(
                name.starts_with("contract_")
                    || name.starts_with("project_")
                    // AI translation jobs (Stage B): the ai_job_* family is
                    // part of the typed command surface by design.
                    || name.starts_with("ai_job_"),
                "unexpected contract command name: {name}"
            );
        }
    }

    /// Built-app evidence (stronger than the plan regex): the real adapter
    /// builds against the mock runtime and the managed state resolves.
    #[test]
    fn adapter_builds_on_mock_runtime_and_manages_state() {
        let dir = tempfile::tempdir().unwrap();
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let state = ContractState::with_root_for_tests(dir.path()).unwrap();
        app.manage(state);
        let resolved = app.state::<ContractState>();
        let list = {
            let m = resolved.manager.lock().unwrap();
            m.list()
        };
        assert!(list.is_empty());
    }
}

// IPC integration tests (review iteration 9, Codecov gap): the three draft
// commands are exercised through the REAL tauri invoke routing — mock
// runtime, JSON body deserialization, handler dispatch, response/error
// serialization — not by calling private helpers. Services semantics
// (merge save, revision guard, quarantine) keep their own suites.
#[cfg(test)]
mod ipc_draft_tests {
    use super::*;
    use serde_json::json;
    use tauri::Manager;

    fn mod_fixture(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("About")).unwrap();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::write(
            root.join("About/About.xml"),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><ModMetaData><name>Ipc Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.ipc</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>",
        )
        .unwrap();
        std::fs::write(
            root.join("Defs/A_Thing.xml"),
            "<Defs><ThingDef><defName>Dup</defName><label>Thing Dup</label></ThingDef></Defs>",
        )
        .unwrap();
    }

    fn invoke<W: AsRef<tauri::Webview<tauri::test::MockRuntime>>>(
        webview: &W,
        cmd: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        tauri::test::get_ipc_response(
            webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.to_string(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<serde_json::Value>().expect("json body"))
    }

    #[test]
    fn drafts_roundtrip_through_tauri_ipc() {
        let dir = tempfile::tempdir().unwrap();
        mod_fixture(&dir.path().join("mod"));
        let app = tauri::test::mock_builder()
            .invoke_handler(tauri::generate_handler![
                project_create,
                project_save_drafts,
                project_restore_drafts,
                project_clear_drafts,
                project_source_xml_preview,
                ai_job_start,
                ai_job_status,
                ai_job_cancel,
                ai_job_accept,
                ai_job_accept_selected,
                project_source_search,
                project_include_candidate,
            ])
            .build(tauri::generate_context!())
            .expect("mock app");
        app.manage(ContractState::new(dir.path().join("managed")).expect("contract state"));
        app.manage(ManagedRoot(dir.path().join("managed")));
        app.manage(AiJobs(AiJobRegistry::new()));
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("webview");

        // create → live session coordinates.
        let created = invoke(
            &webview,
            "project_create",
            json!({"request": {"mod_root": {"path": dir.path().join("mod").to_string_lossy()}, "target_version": "1.6"}}),
        )
        .expect("project_create ok");
        let pid = created["project_id"].as_str().unwrap().to_string();
        let epoch = created["session_epoch"].as_u64().unwrap();
        assert!(!pid.is_empty());

        // save → ack carries the merged count.
        let saved = invoke(
            &webview,
            "project_save_drafts",
            json!({
                "project_id": pid,
                "session_epoch": epoch,
                "entries": {"ru::def_injected·ThingDef·Dup.label": {"text": "черновик", "updated_at": 1}}
            }),
        )
        .expect("save ok");
        assert_eq!(saved["saved"], 1);
        assert_eq!(saved["project_revision"], created["revision"]);

        // restore → the sidecar raises the draft verbatim.
        let report = invoke(
            &webview,
            "project_restore_drafts",
            json!({"project_id": pid, "session_epoch": epoch}),
        )
        .expect("restore ok");
        assert_eq!(report["status"], "restored");
        assert_eq!(
            report["drafts"]["ru::def_injected·ThingDef·Dup.label"]["text"],
            "черновик"
        );

        // clear → acknowledged removal; restore honestly reports none.
        let cleared = invoke(
            &webview,
            "project_clear_drafts",
            json!({"project_id": pid, "session_epoch": epoch, "key": "ru::def_injected·ThingDef·Dup.label"}),
        )
        .expect("clear ok");
        assert_eq!(cleared["saved"], 0);
        let report = invoke(
            &webview,
            "project_restore_drafts",
            json!({"project_id": pid, "session_epoch": epoch}),
        )
        .expect("restore ok");
        assert_eq!(report["status"], "none");

        // Whole-store clear (key omitted) + an absent-key clear that is an
        // honest no-op — both branches of the clear command.
        let store_cleared = invoke(
            &webview,
            "project_save_drafts",
            json!({
                "project_id": pid,
                "session_epoch": epoch,
                "entries": {"ru::def_injected·ThingDef·Dup.label": {"text": "снова", "updated_at": 2}}
            }),
        )
        .expect("save ok");
        assert_eq!(store_cleared["saved"], 1);
        let cleared_all = invoke(
            &webview,
            "project_clear_drafts",
            json!({"project_id": pid, "session_epoch": epoch}),
        )
        .expect("clear-all ok");
        assert_eq!(cleared_all["saved"], 0);
        let absent = invoke(
            &webview,
            "project_clear_drafts",
            json!({"project_id": pid, "session_epoch": epoch, "key": "ru::keyed·Absent"}),
        )
        .expect("absent clear ok");
        assert_eq!(absent["saved"], 0);

        // AI job (Stage B) through the SAME ipc: mock provider translates
        // the fixture labels; the preview stays DRAFT until accepted. The
        // demo provider needs the explicit dev opt-in (production refuses).
        std::env::set_var("RIMLOC_ALLOW_MOCK_JOBS", "1");
        let started = invoke(
            &webview,
            "ai_job_start",
            json!({"request": {
                "project_id": pid,
                "session_epoch": epoch,
                "locale": "Russian",
                "provider_id": "mock",
                "batch_char_budget": 4000
            }}),
        )
        .expect("ai start ok");
        let job_id = started["job_id"].as_str().unwrap().to_string();
        assert_eq!(started["state"], "running");

        let mut final_state = String::new();
        for _ in 0..200 {
            let st =
                invoke(&webview, "ai_job_status", json!({"job_id": job_id})).expect("status ok");
            final_state = st["state"].as_str().unwrap().to_string();
            if final_state != "running" {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(final_state, "preview_ready", "job must reach preview");

        // Accept through the ipc: canonical apply, origin Llm.
        let accepted = invoke(
            &webview,
            "ai_job_accept",
            json!({"request": {"job_id": job_id, "session_epoch": epoch}}),
        )
        .expect("accept ok");
        assert_eq!(accepted["state"], "applied");
        std::env::remove_var("RIMLOC_ALLOW_MOCK_JOBS");
        // Source Discovery (Stage A) through the SAME ipc: query a source
        // text and get reconciled classifications back.
        let search = invoke(
            &webview,
            "project_source_search",
            json!({"request": {
                "project_id": pid,
                "session_epoch": epoch,
                "query": "Thing Dup",
                "scope": "standard",
                "offset": 0,
                "max_results": 50
            }}),
        )
        .expect("source search ok");
        // The fixture Defs file is single-line: one match, reconciled
        // against the inventory with the FULL canonical identity.
        assert_eq!(search["total_matches"], 1, "{search}");
        assert_eq!(
            search["matches"][0]["status"], "already_indexed",
            "{search}"
        );
        assert_eq!(
            search["matches"][0]["identity"]["def_type"], "ThingDef",
            "{search}"
        );
        let statuses: Vec<&str> = search["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["status"].as_str().unwrap_or("?"))
            .collect();
        assert!(
            statuses.contains(&"already_indexed"),
            "DefInjected-origin source region must reconcile as indexed: {statuses:?}"
        );

        // Stale-epoch guard covers the search command too.
        let err = invoke(
            &webview,
            "project_source_search",
            json!({"request": {
                "project_id": pid,
                "session_epoch": epoch + 5,
                "query": "Thing Dup",
                "offset": 0,
                "max_results": 10
            }}),
        )
        .expect_err("stale epoch must refuse search");
        assert_eq!(err["code"], "stale_epoch");

        // Error path: a stale epoch is a typed refusal through the SAME ipc.
        let err = invoke(
            &webview,
            "project_save_drafts",
            json!({
                "project_id": pid,
                "session_epoch": epoch + 999,
                "entries": {}
            }),
        )
        .expect_err("stale epoch must refuse");
        assert_eq!(err["code"], "stale_epoch");
    }

    #[test]
    fn drafts_commands_refuse_unknown_project_via_ipc() {
        let dir = tempfile::tempdir().unwrap();
        mod_fixture(&dir.path().join("mod"));
        let app = tauri::test::mock_builder()
            .invoke_handler(tauri::generate_handler![
                project_create,
                project_save_drafts,
                project_restore_drafts,
                project_clear_drafts,
                project_source_xml_preview,
                ai_job_start,
                ai_job_status,
                ai_job_cancel,
                ai_job_accept,
                ai_job_accept_selected,
                project_source_search,
                project_include_candidate,
            ])
            .build(tauri::generate_context!())
            .expect("mock app");
        app.manage(ContractState::new(dir.path().join("managed")).expect("contract state"));
        app.manage(ManagedRoot(dir.path().join("managed")));
        app.manage(AiJobs(AiJobRegistry::new()));
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("webview");

        for cmd in ["project_save_drafts", "project_restore_drafts"] {
            let body = if cmd == "project_save_drafts" {
                json!({"project_id": "proj-unknown", "session_epoch": 1, "entries": {}})
            } else {
                json!({"project_id": "proj-unknown", "session_epoch": 1})
            };
            let err = invoke(&webview, cmd, body).expect_err("unknown project refuses");
            assert_eq!(err["code"], "project_not_found", "{cmd}");
        }
    }
}
