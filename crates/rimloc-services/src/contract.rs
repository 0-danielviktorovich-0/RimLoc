//! UI binding contract (binding first slice, lead decision 033) — the
//! framework-neutral seam between the GUI client and the services layer.
//!
//! This module owns TYPES ONLY: the contract version, request/response DTOs,
//! the typed error surface with stable machine codes, and the honest
//! capability report for the current slice. No IO lives here; the session
//! layer ([`crate::session`]) executes, the (next-wave) Tauri adapters
//! expose.
//!
//! Contract invariants (binding-first-slice v2, lead decisions):
//! - the UI NEVER sends a whole canonical [`Project`]; edits arrive as
//!   typed [`TranslationIntent`]s against full structural
//!   [`SourceEntryId`]s — the service resolves identity/eligibility and
//!   sets provenance/status itself, so roots/contexts/provenance can never
//!   be forged by the client;
//! - source-side fields are READ-ONLY DTO mirrors;
//! - every mutating call carries `expected_revision` + `session_epoch`
//!   (lost-update and stale-session protection);
//! - unsupported operations are reported honestly via
//!   [`ContractErrorCode::UnsupportedCapability`], never approximated.

use rimloc_domain::canonical::{Project, SourceEntryId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Version of THIS UI binding contract. The client checks it on handshake;
/// a mismatch is a typed [`ContractErrorCode::SchemaVersion`] error, never a
/// best-effort guess. Bump on ANY breaking DTO/error-code change.
pub const UI_CONTRACT_VERSION: u32 = 1;

/// The contract version (function form for the app-info surface).
pub fn ui_contract_version() -> u32 {
    UI_CONTRACT_VERSION
}

/// Stable machine-readable error codes. Values are the wire contract —
/// never rename, only append. (Proposal-doc `E_*` names map 1:1 to these
/// snake_case values.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractErrorCode {
    /// The caller's `session_epoch` is stale (a newer open/refresh won).
    StaleEpoch,
    /// The caller's `expected_revision` does not match the current state.
    StaleRevision,
    /// The durable write failed; the applied state stays dirty in memory
    /// and the client keeps its draft (ack was never given).
    SaveFailed,
    /// The project file on disk changed outside this session between load
    /// and save (content-hash mismatch).
    ProjectChangedOnDisk,
    /// The request violates the contract: unknown identity, non-permitted
    /// action (e.g. editing a deterministic NoTranslate entry), malformed
    /// intent.
    ContractViolation,
    /// A write/open target refused by the source-tree containment guard
    /// (`is_within` / `canonical_view`).
    GuardOutputDenied,
    /// The operation exists in the mandate but is not part of this slice —
    /// reported honestly, never approximated.
    UnsupportedCapability,
    /// The requested project id has no managed project file.
    ProjectNotFound,
    /// The persisted container version is not supported by this build.
    SchemaVersion,
    /// Applied intents produced validation findings classified as errors.
    ValidationFailed,
    /// Anything else; message carries the detail.
    Internal,
}

impl ContractErrorCode {
    /// Stable wire string (the snake_case code).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StaleEpoch => "stale_epoch",
            Self::StaleRevision => "stale_revision",
            Self::SaveFailed => "save_failed",
            Self::ProjectChangedOnDisk => "project_changed_on_disk",
            Self::ContractViolation => "contract_violation",
            Self::GuardOutputDenied => "guard_output_denied",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::ProjectNotFound => "project_not_found",
            Self::SchemaVersion => "schema_version",
            Self::ValidationFailed => "validation_failed",
            Self::Internal => "internal",
        }
    }
}

/// The typed contract error: stable `code` + human message + optional
/// structured details (sanitized upstream; never raw paths to secrets).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ContractError {
    pub code: ContractErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ContractError {
    pub fn new(code: ContractErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn stale_epoch(expected: u64, actual: u64) -> Self {
        Self::new(
            ContractErrorCode::StaleEpoch,
            format!("session epoch {expected} is stale; current is {actual}"),
        )
        .with_details(serde_json::json!({ "expected": expected, "current": actual }))
    }

    pub fn stale_revision(expected: u64, actual: u64) -> Self {
        Self::new(
            ContractErrorCode::StaleRevision,
            format!("expected revision {expected} does not match current {actual}"),
        )
        .with_details(serde_json::json!({ "expected": expected, "current": actual }))
    }

    pub fn project_not_found(project_id: &str) -> Self {
        Self::new(
            ContractErrorCode::ProjectNotFound,
            format!("no managed project with id `{project_id}`"),
        )
    }
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ContractError {}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Opaque durable project id minted by the session layer (never derived
/// from a filesystem path).
pub type ProjectId = String;

/// A single permitted user edit: full structural identity + target locale +
/// the action. `text` carries the new text for [`IntentAction::SetTranslation`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TranslationIntent {
    /// FULL structural identity (kind + key + def_type discriminator).
    pub entry: SourceEntryId,
    /// Target locale of the translation ("Russian" — the folder contract).
    pub locale: String,
    /// The permitted user action.
    pub action: IntentAction,
    /// New text for [`IntentAction::SetTranslation`] (required there,
    /// ignored otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Permitted user actions on a translation. Anything outside this list
/// (entry creation/deletion, identity edits, provenance edits) is not an
/// intent — it is either a read-only view or unsupported in this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    /// Set the translation text (non-empty; completeness → translated).
    SetTranslation,
    /// Mark the literal TODO placeholder (completeness → todo).
    MarkTodo,
    /// Clear the translation (completeness → untranslated).
    ClearTranslation,
}

/// Apply typed intents with lost-update protection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyIntentsRequest {
    pub project_id: ProjectId,
    /// Lost-update guard: the revision the caller based its edits on.
    pub expected_revision: u64,
    /// Stale-session guard: the epoch of the caller's open session.
    pub session_epoch: u64,
    pub intents: Vec<TranslationIntent>,
}

/// One intent the service refused. The rest of the batch still applies
/// (per-intent errors are data, not a whole-batch failure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkippedIntent {
    /// Index in the request `intents` array.
    pub index: usize,
    pub code: ContractErrorCode,
    pub message: String,
}

/// Per-project session counter: bumps on every open/refresh so stale UI
/// sessions can be dropped client-side without touching the store.
pub type SessionEpoch = u64;

/// Durable monotonic content revision: bumps on every acked persisted
/// change; durable in the same atomic managed-project record as the
/// content itself.
pub type Revision = u64;

/// The job id of a mutating service call (observability op-id semantics:
/// stable, log-correlated). Cancellation of the NEXT slice's long jobs
/// builds on this handle.
pub type JobId = String;

/// Result of an acked `project_apply`: the new durable revision, the job id
/// (correlates with the observability operation log), per-intent accounting
/// and whether a cancellation cut the batch short.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyIntentsResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub applied: usize,
    pub skipped: Vec<SkippedIntent>,
    /// True when a cancel was requested mid-batch: the applied prefix is
    /// persisted (persist-before-ack), the rest is reported as skipped.
    pub cancelled: bool,
}

/// Read-only snapshot of a managed project. The embedded [`Project`] is the
/// canonical domain state; source-side fields are read-only by contract —
/// edits go back exclusively as [`TranslationIntent`]s.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSnapshot {
    pub project_id: ProjectId,
    pub revision: Revision,
    pub session_epoch: SessionEpoch,
    pub project: Project,
}

/// One managed project in the list view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSummary {
    pub project_id: ProjectId,
    /// Human display name captured at create (mod folder name).
    pub name: String,
    pub revision: Revision,
    /// Target RimWorld version the project was built for, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_version: Option<String>,
}

/// Parameters of `project_create`: build a canonical project from a
/// read-only source mod and persist it under a fresh opaque id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateProjectRequest {
    /// Read-only source mod root (never written; guard-checked).
    pub mod_root: PathBufDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_version: Option<String>,
}

/// Path DTO — keep the wire framework-neutral (no camino/tarexpr leakage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PathBufDto {
    pub path: String,
}

impl PathBufDto {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

// ---------------------------------------------------------------------------
// Capability report — honest slice boundary
// ---------------------------------------------------------------------------

/// Operations this contract slice SUPPORTS end-to-end (services layer;
/// Tauri registration is the next wave).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ProjectCreate,
    ProjectOpen,
    ProjectList,
    ProjectSnapshot,
    ProjectApplyIntents,
    ProjectRefresh,
    JobCancel,
    ProjectValidate,
    ProjectBuildExport,
    ProjectDiagnosticsBundle,
}

/// Mandated operations that are honestly NOT in this slice. Each carries
/// the reason so the client can render a truthful "unsupported" state
/// instead of a silent no-op.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UnsupportedCapability {
    pub capability: String,
    pub reason: String,
}

/// The capability report served on contract handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityReport {
    pub contract_version: u32,
    pub supported: Vec<Capability>,
    pub unsupported: Vec<UnsupportedCapability>,
}

/// The honest capability report for this slice.
pub fn capability_report() -> CapabilityReport {
    CapabilityReport {
        contract_version: UI_CONTRACT_VERSION,
        supported: vec![
            Capability::ProjectCreate,
            Capability::ProjectOpen,
            Capability::ProjectList,
            Capability::ProjectSnapshot,
            Capability::ProjectApplyIntents,
            Capability::ProjectRefresh,
            Capability::JobCancel,
            Capability::ProjectValidate,
            Capability::ProjectBuildExport,
            Capability::ProjectDiagnosticsBundle,
        ],
        unsupported: vec![
            UnsupportedCapability {
                capability: "source_inspector_actions".into(),
                reason: "next slice: identity-based source actions with size-limited reads".into(),
            },
            UnsupportedCapability {
                capability: "providers_settings".into(),
                reason: "later slice: provider/settings parity".into(),
            },
            UnsupportedCapability {
                capability: "entry_create_delete".into(),
                reason: "intents cover translation edits only; identities come from rescan".into(),
            },
            UnsupportedCapability {
                capability: "import_pack".into(),
                reason: "existing-pack import is not exposed as a contract intent yet".into(),
            },
        ],
    }
}

/// One typed validation finding over the canonical inventory. `id` carries
/// the FULL structural identity when the finding resolves to an inventory
/// entry; `key`/`path` stay raw (validator wording) for diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ValidationFinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<SourceEntryId>,
    /// "error" | "warning" | "info" (ValidationSeverity wording).
    pub severity: String,
    pub kind: String,
    pub key: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub message: String,
}

/// Result of `project_validate`: typed findings over the trusted session
/// state. The operation NEVER mutates the project; `status` is "failed"
/// only when error-severity findings exist (026 semantics — warnings/info
/// are successful and reported).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ValidateProjectResponse {
    pub job_id: JobId,
    /// "succeeded" | "failed" (failed = error-severity findings present).
    pub status: String,
    pub findings: Vec<ValidationFinding>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    /// Locale the findings were produced for (all locales when unfiltered).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// Result of `project_export`: the isolated native output written from the
/// trusted session state, reparse-verified BEFORE the ack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExportProjectResponse {
    pub job_id: JobId,
    pub out_dir: PathBufDto,
    pub files_written: usize,
    /// Keys the EXISTING scanner re-parsed from the written output.
    pub reparsed_keys: usize,
    /// Unknown-def-type entries skipped by the writer (surfaced for
    /// review/rescan — delta risk #5).
    pub skipped_unknown_type: Vec<String>,
}

/// Result of `project_diagnose`: a sanitized support bundle describing the
/// project's last FAILED operation (id, cause, affected identities).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagnoseResponse {
    pub job_id: JobId,
    /// Sanitized bundle (path is OUTSIDE the read-only source tree).
    pub bundle_dir: PathBufDto,
    pub operation_id: String,
    pub files: Vec<String>,
    pub redacted_count: usize,
    pub excluded_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Error codes are a closed wire contract: the snake_case strings must
    /// stay stable.
    #[test]
    fn error_code_wire_values_are_stable() {
        assert_eq!(ContractErrorCode::StaleEpoch.as_str(), "stale_epoch");
        assert_eq!(ContractErrorCode::StaleRevision.as_str(), "stale_revision");
        assert_eq!(ContractErrorCode::SaveFailed.as_str(), "save_failed");
        assert_eq!(
            ContractErrorCode::ProjectChangedOnDisk.as_str(),
            "project_changed_on_disk"
        );
        assert_eq!(
            ContractErrorCode::ContractViolation.as_str(),
            "contract_violation"
        );
        assert_eq!(
            ContractErrorCode::GuardOutputDenied.as_str(),
            "guard_output_denied"
        );
        assert_eq!(
            ContractErrorCode::UnsupportedCapability.as_str(),
            "unsupported_capability"
        );
        // serde round-trips by value, not by position.
        let code: ContractErrorCode =
            serde_json::from_value(serde_json::json!("stale_revision")).unwrap();
        assert_eq!(code, ContractErrorCode::StaleRevision);
    }

    /// The handshake version is a constant — client equality check.
    #[test]
    fn ui_contract_version_is_stable() {
        assert_eq!(ui_contract_version(), UI_CONTRACT_VERSION);
        assert_eq!(UI_CONTRACT_VERSION, 1);
    }

    /// The capability report is honest: this slice's supported list, and
    /// every unsupported entry carries a reason. The validate / build /
    /// diagnostics wave moved from unsupported to a live services surface
    /// in this slice — it must not linger in the unsupported report.
    #[test]
    fn capability_report_lists_slice_boundary() {
        let report = capability_report();
        assert_eq!(report.contract_version, 1);
        assert!(report.supported.contains(&Capability::ProjectApplyIntents));
        assert!(report.supported.contains(&Capability::ProjectValidate));
        assert!(report.supported.contains(&Capability::ProjectBuildExport));
        assert!(report
            .supported
            .contains(&Capability::ProjectDiagnosticsBundle));
        assert!(report.unsupported.iter().all(|u| !u.reason.is_empty()));
        assert!(!report.unsupported.iter().any(|u| {
            u.capability == "validate_via_contract"
                || u.capability == "build_export"
                || u.capability == "diagnostics_bundle"
        }));
    }

    /// Intents round-trip with the full structural identity.
    #[test]
    fn intent_round_trips_structural_identity() {
        let intent = TranslationIntent {
            entry: SourceEntryId {
                kind: rimloc_domain::canonical::EntryKind::DefInjected,
                key: "Widget.label".into(),
                def_type: Some("ThingDef".into()),
            },
            locale: "Russian".into(),
            action: IntentAction::SetTranslation,
            text: Some("метка".into()),
        };
        let v = serde_json::to_value(&intent).unwrap();
        assert_eq!(v["entry"]["def_type"], "ThingDef");
        assert_eq!(v["action"], "set_translation");
        let back: TranslationIntent = serde_json::from_value(v).unwrap();
        assert_eq!(back, intent);
    }
}
