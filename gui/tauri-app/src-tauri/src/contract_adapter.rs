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
    CapabilityReport, CreateProjectRequest, ProjectGlossaryDeleteRequest,
    ProjectGlossaryDeleteResponse, ProjectGlossaryUpsertRequest, ProjectGlossaryUpsertResponse,
    ProjectSnapshot, ProjectSummary, TmDeleteRequest, TmDeleteResponse, TmImportRequest,
    TmImportResponse, TmListRequest, TmListResponse, TmLookupRequest, TmLookupResponse,
    TmUpsertRequest, TmUpsertResponse,
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
    "project_build_mod",
    "project_diagnose",
    // W2 (existing-pack flow): dry-run analysis + guarded application of an
    // existing translation pack against the open project.
    "project_import_existing",
    "project_apply_existing",
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

impl ContractState {
    pub fn new(managed_root: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            manager: Mutex::new(ProjectSessionManager::new(managed_root)?),
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
        manager.create(
            std::path::Path::new(&request.mod_root.path),
            request.target_version.as_deref(),
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
) -> Result<
    Vec<rimloc_domain::glossary::GlossaryTerm>,
    rimloc_services::contract::ContractError,
> {
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
    traced_simple("project_glossary_upsert", || manager.glossary_upsert(&request))
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
    traced_simple("project_glossary_delete", || manager.glossary_delete(&request))
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
    traced_simple("project_export", || {
        manager.export_project(
            &project_id,
            session_epoch,
            std::path::Path::new(&out_dir),
            &locale,
        )
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
    traced_simple("project_build_mod", || {
        manager.build_mod_project(
            &project_id,
            session_epoch,
            std::path::Path::new(&out_dir),
            &locale,
        )
    })
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

/// Manage the contract state on a builder. Returns the same builder type;
/// runtime-generic so tests can use the mock runtime.
pub fn attach_contract<C: tauri::Runtime>(
    builder: tauri::Builder<C>,
    managed_root: PathBuf,
) -> Result<tauri::Builder<C>, std::io::Error> {
    let state = ContractState::new(managed_root)?;
    Ok(builder.manage(state))
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
                name.starts_with("contract_") || name.starts_with("project_"),
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
