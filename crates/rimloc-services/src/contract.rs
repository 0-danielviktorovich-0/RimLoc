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
    /// A provider configuration is semantically wrong: a malformed
    /// base_url, an empty model, or a cloud provider with no API key
    /// available. Appended (never renamed) with the provider-instances
    /// slice; the message names the exact problem, never the secret.
    InvalidConfig,
    /// A write/open target refused by the source-tree containment guard
    /// (`is_within` / `canonical_view`).
    GuardOutputDenied,
    /// A caller-specified output path is not absolute. Every contract
    /// write refuses the FORM before any filesystem access: the writer
    /// builds directories under this path with the process CWD as the
    /// implicit base, so a relative path silently lands wherever the app
    /// was launched from. (Wire rule: codes are never renamed, only
    /// appended — this is an append.)
    InvalidOutputPath,
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
            Self::InvalidConfig => "invalid_config",
            Self::GuardOutputDenied => "guard_output_denied",
            Self::InvalidOutputPath => "invalid_output_path",
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

    /// A relative (or empty) output path — refused before any write. The
    /// message tells the caller the required form instead of guessing a
    /// base directory for them.
    pub fn invalid_output_path(path: &std::path::Path) -> Self {
        Self::new(
            ContractErrorCode::InvalidOutputPath,
            format!(
                "output path `{}` is not absolute; specify an absolute output directory (e.g. `/Users/you/RimLoc-Export` or `C:/Users/you/RimLoc-Export`)",
                path.display()
            ),
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
    /// True when in-memory edits are applied but NOT durably acked
    /// (`save_failed` / external change). `refresh` DISCARDS dirty edits
    /// (disk wins) — the client must gate refresh behind an explicit
    /// discard confirmation instead of silently destroying the draft.
    /// Additive v2 field (defaults keep older payloads loadable).
    #[serde(default)]
    pub dirty: bool,
    /// The last durably ACKED revision. While dirty, `apply` compares its
    /// `expected_revision` against this — a snapshot exposing only
    /// `revision` would hand the client a base the next apply rejects as
    /// `stale_revision`. Additive v2 field.
    #[serde(default)]
    pub acked_revision: Revision,
    /// Source-drift verdict (M3), evaluated when the session started
    /// (create / open from disk / refresh): `Some(true)` the source
    /// content the inventory was built from changed under the project —
    /// a content edit, an added/removed file or a LoadFolders version
    /// rollback; `Some(false)` in sync; `None` unknown — legacy envelopes
    /// carry no recorded fingerprint, and an unreadable source at check
    /// time is NEVER reported as "in sync". Additive v2 field.
    #[serde(default)]
    pub source_changed: Option<bool>,
    /// Read-only source mod root the project was built from, as the session
    /// holds it (persisted envelope H5). `None` on legacy envelopes — the
    /// UI must show an honest unknown instead of a template placeholder
    /// (finding M-7: the Project tab rendered mock location constants on
    /// live contract projects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<PathBufDto>,
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

/// A managed project file that could NOT be loaded (M2). Surfaced
/// explicitly instead of silently disappearing from the list — corruption
/// (disk fault, bad backup rollback) must never look like "the project is
/// gone".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UnloadableProject {
    pub project_id: ProjectId,
    /// Stable lowercase reason code: `corrupt_project` (unreadable/broken
    /// JSON) or `schema_version` (unsupported container version).
    pub reason: String,
}

/// The full list view: loadable projects plus explicit diagnostics for
/// unloadable managed files (M2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectListReport {
    pub projects: Vec<ProjectSummary>,
    pub unloadable: Vec<UnloadableProject>,
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
    /// Full drop-in mod package from the session state (`project_build_mod`):
    /// `About/About.xml` in the game-loadable `<ModMetaData>` shape (what the
    /// CLI build-mod writer emits) plus the `Languages/<locale>` tree — the
    /// folder a player can move straight into the game's Mods directory.
    /// Wire name appends (never renames) per the contract rule.
    ProjectBuildMod,
    ProjectDiagnosticsBundle,
    /// Self-localization entry (mandate D): the shell exposes the app-bundled
    /// UI catalog as an ORDINARY project source. Transport lives at the shell
    /// level (`selfloc_catalog_dir`, next to pick_directory) — this entry
    /// makes the supported slice visible on the handshake instead of hiding
    /// a shipped capability. Wire name appends (never renames) per the
    /// contract rule.
    SelflocCatalog,
    /// Dry-run analysis of an existing translation pack against the open
    /// project (`project_import_existing`): read-only classification into
    /// reusable / conflicts / obsolete / ambiguous / invalid + the count of
    /// inventory strings the pack does not cover. Wire name appends.
    ProjectImportExisting,
    /// Apply the reusable set of an analyzed existing pack into the open
    /// project (`project_apply_existing`): the same resolution as the
    /// analysis, persist-before-ack, existing translations never
    /// overwritten, ambiguous lines never auto-applied. Wire name appends.
    ProjectApplyExisting,
    /// Project glossary (wave 13): read the terms (`project_glossary`),
    /// create/update by case-insensitive term (`project_glossary_upsert`),
    /// delete (`project_glossary_delete`) — generic project state,
    /// persist-before-ack. Wire name appends (never renames) per the
    /// contract rule.
    ProjectGlossary,
    /// Translation memory (TM live, owner decision A+B+C): list/filter
    /// (`project_tm_list`), manual CRUD (`project_tm_upsert`,
    /// `project_tm_delete`), bulk import JSON/CSV (`project_tm_import`),
    /// ranked lookup exact→normalized→fuzzy (`project_tm_lookup`). Records
    /// live in the project envelope (`Project.tm`), persist-before-ack;
    /// auto-accumulation rides the apply ack (A). Wire name appends (never
    /// renames) per the contract rule.
    TranslationMemory,
    /// Provider instances (provider/settings parity slice): app-global CRUD
    /// over AI provider configurations (`provider_instance_list`,
    /// `provider_instance_upsert`, `provider_instance_delete`,
    /// `provider_instance_validate`). Instance metadata persists in the
    /// settings file next to the managed projects; the API key NEVER does —
    /// it lives in the OS keychain and the contract surface only ever
    /// reports the `has_key` boolean. Wire name appends (never renames)
    /// per the contract rule.
    ProviderInstances,
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
            Capability::ProjectBuildMod,
            Capability::ProjectDiagnosticsBundle,
            Capability::SelflocCatalog,
            Capability::ProjectImportExisting,
            Capability::ProjectApplyExisting,
            Capability::ProjectGlossary,
            Capability::TranslationMemory,
            Capability::ProviderInstances,
        ],
        unsupported: vec![
            UnsupportedCapability {
                capability: "source_inspector_actions".into(),
                reason: "next slice: identity-based source actions with size-limited reads".into(),
            },
            UnsupportedCapability {
                capability: "entry_create_delete".into(),
                reason: "intents cover translation edits only; identities come from rescan".into(),
            },
            // W2: `import_pack` moved from unsupported to the two live
            // entries above (import = dry-run analysis, apply = separate
            // guarded command). The unsupported entry is REMOVED — the
            // report must never claim a shipped capability is missing.
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

/// Result of `project_build_mod`: the FULL drop-in mod package written from
/// the trusted session state, reparse-verified BEFORE the ack. Same DTO
/// pattern as [`ExportProjectResponse`] — the differences live in the
/// output, not the report: `About/About.xml` lands in the game-loadable
/// `<ModMetaData>` shape (CLI build-mod writer) instead of the export's
/// internal `<RimWorldManifest>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BuildModProjectResponse {
    pub job_id: JobId,
    /// The out dir AS THE WRITER used it (the mod package root: `About/` +
    /// `Languages/` live directly inside).
    pub out_dir: PathBufDto,
    pub files_written: usize,
    /// Keys the EXISTING scanner re-parsed from the written output.
    pub reparsed_keys: usize,
    /// Unknown-def-type entries skipped by the writer (surfaced for
    /// review/rescan — same accounting as `project_export`).
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

// ---------------------------------------------------------------------------
// Existing translation pack (W2): dry-run analysis + guarded application
// ---------------------------------------------------------------------------

/// Size cap for the per-category sample lists in
/// [`ImportExistingResponse`]. Counts are always exact; the lists are
/// capped samples so a huge pack can never flood the wire or the UI.
pub const EXISTING_LIST_LIMIT: usize = 50;

/// Request `project_import_existing`: dry-run analysis of an existing
/// translation pack directory against the open project. READ-ONLY: nothing
/// on disk or in the project is written, no revision bump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImportExistingRequest {
    pub project_id: ProjectId,
    /// Stale-session guard (the read-only ops carry it the same way
    /// `project_validate` does).
    pub session_epoch: SessionEpoch,
    /// Absolute directory of the existing pack to scan (typically a
    /// `Languages/<locale>` folder). Form guard: relative paths are
    /// refused before any filesystem access.
    pub existing_dir: PathBufDto,
    /// Target locale the pack would feed (strict language-folder form).
    pub locale: String,
}

/// One analyzed pack line in a capped sample list. `entry` carries the
/// FULL structural identity the line addresses (reusable / conflicts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExistingMatchItem {
    /// The pack's serialization key.
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<SourceEntryId>,
}

/// One ambiguous pack line: several candidate source identities — reported
/// for review, NEVER auto-applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExistingAmbiguousItem {
    pub key: String,
    /// Candidate source keys (from the shared matcher).
    pub candidates: Vec<String>,
}

/// Result of the dry-run `project_import_existing`. Categories mirror the
/// services analyzer ([`crate::project::ExistingPackAnalysis`]):
/// - `reusable` — matched an entry with an empty `<locale>` slot: exactly
///   what `project_apply_existing` would apply;
/// - `conflicts` — matched an entry that ALREADY has a `<locale>`
///   translation: existing work wins, never overwritten;
/// - `obsolete` — pack lines addressing nothing in the inventory;
/// - `ambiguous` — several candidate identities, human decides;
/// - `invalid` — empty/TODO pack lines;
/// - `new_count` — inventory entries that stay untranslated after the
///   merge (the pack does not cover them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImportExistingResponse {
    pub job_id: JobId,
    pub scanned_files: usize,
    pub scanned_keys: usize,
    pub reusable_count: usize,
    pub conflict_count: usize,
    pub obsolete_count: usize,
    pub ambiguous_count: usize,
    pub invalid_count: usize,
    pub new_count: usize,
    /// Capped sample lists (see [`EXISTING_LIST_LIMIT`]).
    pub reusable: Vec<ExistingMatchItem>,
    pub conflicts: Vec<ExistingMatchItem>,
    pub obsolete: Vec<ExistingMatchItem>,
    pub ambiguous: Vec<ExistingAmbiguousItem>,
    pub invalid: Vec<ExistingMatchItem>,
}

/// Request `project_apply_existing`: apply the REUSABLE set of the pack
/// into the open project, persist-before-ack. The full apply-intents guard
/// set applies: stale epoch/revision refuse the whole operation, existing
/// translations are never overwritten (conflicts stay conflicts), and
/// ambiguous/obsolete/invalid lines are never applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyExistingRequest {
    pub project_id: ProjectId,
    /// Lost-update guard: the revision the caller based the decision on
    /// (same discipline as `project_apply_intents`).
    pub expected_revision: Revision,
    /// Stale-session guard.
    pub session_epoch: SessionEpoch,
    /// Absolute directory of the existing pack (same form guards).
    pub existing_dir: PathBufDto,
    /// Target locale (strict language-folder form).
    pub locale: String,
}

/// Result of an acked `project_apply_existing`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyExistingResponse {
    pub job_id: JobId,
    pub revision: Revision,
    /// Pack lines applied into previously empty slots (origin=Imported).
    pub applied: usize,
    /// Existing translations that were NOT overwritten.
    pub conflicts: usize,
    /// Pack lines addressing nothing in the inventory (not applied).
    pub unmatched: usize,
    /// Ambiguous pack lines (not applied — a human decides).
    pub ambiguous: usize,
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
        assert_eq!(ContractErrorCode::InvalidConfig.as_str(), "invalid_config");
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
        assert!(report.supported.contains(&Capability::ProjectBuildMod));
        assert!(report
            .supported
            .contains(&Capability::ProjectDiagnosticsBundle));
        // W2: existing-pack import/apply are live slice operations now.
        assert!(report
            .supported
            .contains(&Capability::ProjectImportExisting));
        assert!(report.supported.contains(&Capability::ProjectApplyExisting));
        assert!(report.supported.contains(&Capability::ProjectGlossary));
        // TM live (A+B+C): the capability is reported, never hidden.
        assert!(report.supported.contains(&Capability::TranslationMemory));
        // Provider-instances slice: shipped — the capability is reported and
        // the stale `providers_settings` unsupported entry is GONE (the
        // report must never claim a shipped capability is missing).
        assert!(report.supported.contains(&Capability::ProviderInstances));
        assert!(!report
            .unsupported
            .iter()
            .any(|u| u.capability == "providers_settings"));
        assert!(report.unsupported.iter().all(|u| !u.reason.is_empty()));
        assert!(!report.unsupported.iter().any(|u| {
            u.capability == "validate_via_contract"
                || u.capability == "build_export"
                || u.capability == "diagnostics_bundle"
                // W2: import_pack IS supported (via the two entries above)
                // — the report must not claim it is missing.
                || u.capability == "import_pack"
        }));
    }

    /// The existing-pack DTOs round-trip with capped list samples and
    /// optional identity fields omitted when absent.
    #[test]
    fn existing_pack_dtos_round_trip() {
        let resp = ImportExistingResponse {
            job_id: "op-1".into(),
            scanned_files: 2,
            scanned_keys: 5,
            reusable_count: 2,
            conflict_count: 1,
            obsolete_count: 1,
            ambiguous_count: 1,
            invalid_count: 0,
            new_count: 0,
            reusable: vec![ExistingMatchItem {
                key: "Greeting".into(),
                entry: Some(SourceEntryId {
                    kind: rimloc_domain::canonical::EntryKind::Keyed,
                    key: "Greeting".into(),
                    def_type: None,
                }),
            }],
            conflicts: vec![],
            obsolete: vec![ExistingMatchItem {
                key: "OldKey".into(),
                entry: None,
            }],
            ambiguous: vec![ExistingAmbiguousItem {
                key: "A.C".into(),
                candidates: vec!["A.C".into(), "A.B".into()],
            }],
            invalid: vec![],
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["reusable"][0]["entry"]["kind"], "keyed");
        // Absent optional identity is omitted from the wire, not null.
        assert!(v["obsolete"][0].get("entry").is_none());
        let back: ImportExistingResponse = serde_json::from_value(v).unwrap();
        assert_eq!(back, resp);

        let req = ApplyExistingRequest {
            project_id: "proj-x".into(),
            expected_revision: 3,
            session_epoch: 2,
            existing_dir: PathBufDto::new("/mods/MyMod/Languages/Russian"),
            locale: "Russian".into(),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["expected_revision"], 3);
        assert_eq!(v["session_epoch"], 2);
        assert_eq!(v["existing_dir"]["path"], "/mods/MyMod/Languages/Russian");
        let back: ApplyExistingRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
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

    /// TM live (A+B+C): request/response DTOs round-trip with the domain
    /// entry embedded; the status/provenance wire values are snake_case.
    #[test]
    fn tm_dtos_round_trip_wire_shapes() {
        let entry = rimloc_domain::tm::TranslationMemoryEntry {
            id: "op-1".into(),
            source_text: "Save game".into(),
            target_text: "Сохранить игру".into(),
            target_locale: "Russian".into(),
            status: rimloc_domain::tm::TmStatus::Draft,
            provenance: rimloc_domain::tm::TmProvenance::Import,
        };
        let v = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["status"], "draft");
        assert_eq!(v["provenance"], "import");
        let back: rimloc_domain::tm::TranslationMemoryEntry = serde_json::from_value(v).unwrap();
        assert_eq!(back, entry);

        let req = TmLookupRequest {
            project_id: "proj-x".into(),
            session_epoch: 2,
            source_text: "Save game".into(),
            target_locale: "Russian".into(),
            limit: Some(5),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["target_locale"], "Russian");
        assert_eq!(v["limit"], 5);
        let back: TmLookupRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);

        // Optional filters/limit deserialize from a bare wire object.
        let bare: TmListRequest = serde_json::from_value(serde_json::json!({
            "project_id": "proj-x", "session_epoch": 1
        }))
        .unwrap();
        assert_eq!(bare.locale, None);
        assert_eq!(bare.status, None);
        assert_eq!(bare.query, None);

        let resp = TmImportResponse {
            job_id: "op-2".into(),
            revision: 7,
            imported: 3,
            updated: 1,
            skipped: 2,
            rejected: 1,
        };
        let v = serde_json::to_value(&resp).unwrap();
        let back: TmImportResponse = serde_json::from_value(v).unwrap();
        assert_eq!(back, resp);
    }

    /// Provider instances (redaction slice): the summary DTO has NO secret
    /// field on the wire, the upsert request Debug output masks the secret,
    /// and the summary round-trips with optional fields omitted when absent.
    #[test]
    fn provider_dtos_redact_secret_and_round_trip() {
        const SECRET: &str = "sk-provider-secret-must-never-echo";

        let summary = ProviderInstanceSummary {
            id: "prov-abc123".into(),
            preset: "openai".into(),
            label: "OpenAI".into(),
            model: "gpt-5".into(),
            base_url: Some("https://api.openai.com/v1".into()),
            local: false,
            has_key: true,
            created_at_ms: 1_759_680_000_000,
            updated_at_ms: 1_759_680_000_000,
        };
        let wire = serde_json::to_string(&summary).unwrap();
        assert!(
            !wire.contains(SECRET),
            "summary wire form must never contain the secret"
        );
        // The secret field simply does not exist on the wire type.
        let v: serde_json::Value = serde_json::from_str(&wire).unwrap();
        assert!(v.get("secret").is_none());
        assert_eq!(v["has_key"], true);
        let back: ProviderInstanceSummary = serde_json::from_value(v).unwrap();
        assert_eq!(back, summary);

        // Absent optional base_url is omitted, not null.
        let mut no_url = summary.clone();
        no_url.base_url = None;
        let v = serde_json::to_value(&no_url).unwrap();
        assert!(v.get("base_url").is_none());

        // The upsert request transports the secret once but NEVER through
        // Debug — logs showing `{:?}` of a request stay clean.
        let req = ProviderInstanceUpsertRequest {
            instance_id: None,
            preset: "openai".into(),
            label: "OpenAI".into(),
            model: "gpt-5".into(),
            base_url: Some("https://api.openai.com/v1".into()),
            secret: Some(SECRET.into()),
            local: None,
            expected_revision: None,
        };
        let debug = format!("{req:?}");
        assert!(
            !debug.contains(SECRET),
            "Debug of an upsert request must mask the secret: {debug}"
        );
        assert!(debug.contains("<redacted>"));
        // Serde still carries it on the wire (the one legitimate transport).
        let wire = serde_json::to_string(&req).unwrap();
        assert!(wire.contains(SECRET));
    }
}

/// `project_glossary_upsert` request (wave 13): create/update one term by
/// case-insensitive `term` match. Nothing whole-project rides the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectGlossaryUpsertRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    pub term: String,
    pub translation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Upsert ack: the durable revision plus the stored entry (id minted once).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectGlossaryUpsertResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub entry: rimloc_domain::glossary::GlossaryTerm,
}

/// `project_glossary_delete` request: remove by case-insensitive `term`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectGlossaryDeleteRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    pub term: String,
}

/// Delete ack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectGlossaryDeleteResponse {
    pub job_id: JobId,
    pub revision: Revision,
    /// Stable id of the removed entry.
    pub removed_id: String,
}

// ---------------------------------------------------------------------------
// Translation memory (TM live, owner decision A+B+C). Records live in
// `Project.tm` (the glossary pattern); every mutating op is
// persist-before-ack. Domain types (`rimloc_domain::tm`) ride the wire as-is
// like the glossary entries do.
// ---------------------------------------------------------------------------

/// `project_tm_list` request: the project's TM records with OPTIONAL
/// in-memory filters (locale exact, status exact, query substring
/// case-insensitive over source/target). Filters never touch the
/// filesystem, so they need no form guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmListRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<rimloc_domain::tm::TmStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// `project_tm_list` ack: the filtered records plus the UNFILTERED total
/// (the UI can render an honest "N of M" without a second call).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmListResponse {
    pub job_id: JobId,
    pub entries: Vec<rimloc_domain::tm::TranslationMemoryEntry>,
    pub total: usize,
}

/// `project_tm_upsert` request (C = manual CRUD): create or update one
/// record by the (source_text, target_locale) key. Provenance is set by
/// the SERVICE (Manual — the client never forges it); the status is the
/// user's choice, `None` → ACCEPTED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmUpsertRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    pub source_text: String,
    pub target_text: String,
    pub target_locale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<rimloc_domain::tm::TmStatus>,
}

/// Upsert ack: the durable revision plus the stored entry (id minted once,
/// survives updates).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmUpsertResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub entry: rimloc_domain::tm::TranslationMemoryEntry,
}

/// `project_tm_delete` request: remove by stable id; an unknown id is a
/// typed refusal, never silent success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmDeleteRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    pub id: String,
}

/// Delete ack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmDeleteResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub removed_id: String,
}

/// `project_tm_import` request (B): a bulk payload, JSON array of
/// `{source, target[, locale][, status]}` objects OR CSV lines
/// `source,target[,locale][,status]` (optional header, RFC-4180-lite
/// quoting). The format is auto-detected; `format` forces it. Records land
/// with provenance=IMPORT and status=DRAFT unless the row carries an
/// EXPLICIT status — an import is never trusted blindly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmImportRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    /// Raw payload (file contents pasted or read client-side; the contract
    /// carries text, never a server-side path).
    pub payload: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<TmImportFormat>,
}

/// Import payload format. Auto-detect: a payload whose first byte is `[`
/// or `{` parses as JSON (so broken JSON is a typed refusal, never a
/// garbage CSV row), else as CSV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TmImportFormat {
    Json,
    Csv,
}

/// Import ack. `imported` = fresh keys, `updated` = existing weaker
/// (DRAFT) records refreshed by the import, `skipped` = existing
/// stronger-or-equal records kept untouched, `rejected` = per-row form
/// refusals (reason per row). A payload that parses as NEITHER JSON nor
/// CSV is a whole-operation `contract_violation`, never a partial apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmImportResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub rejected: usize,
}

/// `project_tm_lookup` request: candidates for one source text within ONE
/// target locale (isolation is mandatory — the locale is required).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmLookupRequest {
    pub project_id: ProjectId,
    pub session_epoch: SessionEpoch,
    pub source_text: String,
    pub target_locale: String,
    /// Max matches (default 5, clamped 1..=50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// One ranked match. `match_kind` names the tier that found it (exact →
/// normalized → fuzzy); `distance` is the Levenshtein distance of the
/// NORMALIZED forms (0 for exact, `None` for exact per contract
/// simplicity — present only for the fuzzy tier).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmMatch {
    pub entry: rimloc_domain::tm::TranslationMemoryEntry,
    /// "exact" | "normalized" | "fuzzy".
    pub match_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<usize>,
}

/// Lookup ack: ranked best-first (tier, then distance, then trust rank,
/// then stable insertion order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TmLookupResponse {
    pub job_id: JobId,
    pub matches: Vec<TmMatch>,
}

// ---------------------------------------------------------------------------
// Provider instances (provider/settings parity). App-GLOBAL settings (no
// project_id, no session epoch — there is no open project session behind
// them); the settings file lives next to the managed projects and the API
// key NEVER does: the secret goes straight into the OS keychain and this
// wire surface carries only the `has_key` boolean.
//
// Redaction is BY CONSTRUCTION: [`ProviderInstanceSummary`] has no field
// that could hold a secret, and the upsert request's `secret` field has a
// manual `Debug` impl that prints `<redacted>` — a request can be logged
// without leaking.
// ---------------------------------------------------------------------------

/// One provider instance in list/ack responses — REDACTED by construction:
/// no secret field exists on this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceSummary {
    /// Minted opaque id (`prov-<hex>`); also the keychain account.
    pub id: String,
    /// Builtin preset id (`anthropic` | `openai` | `zai` | `ollama`) or
    /// `custom`.
    pub preset: String,
    pub label: String,
    pub model: String,
    /// Resolved base URL (explicit override or the preset default);
    /// `None` = native Anthropic endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Local providers (ollama-class) need no API key.
    pub local: bool,
    /// Presence marker only — the key itself lives in the OS keychain.
    pub has_key: bool,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// `provider_instance_upsert` request: create (`instance_id = None`) or
/// edit (`Some`). `secret = Some` stores a NEW key into the OS keychain
/// (empty/whitespace treated as absent — "keep the existing key");
/// `secret = None` never touches the stored key. `local` is derived from
/// the preset (ollama) and only OVERRIDABLE for `custom`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceUpsertRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub preset: String,
    #[serde(default)]
    pub label: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// The API key for the OS keychain — transported once on this call and
    /// NEVER persisted to a file, echoed back, or logged (`Debug` masks it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<bool>,
    /// Lost-update guard over the settings revision (durable, bumps on every
    /// acked settings change). `None` = last write wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<Revision>,
}

impl std::fmt::Debug for ProviderInstanceUpsertRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderInstanceUpsertRequest")
            .field("instance_id", &self.instance_id)
            .field("preset", &self.preset)
            .field("label", &self.label)
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .field("local", &self.local)
            .field("expected_revision", &self.expected_revision)
            .finish()
    }
}

/// Upsert ack: the durable settings revision plus the REDACTED summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceUpsertResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub instance: ProviderInstanceSummary,
}

/// The redacted list ack — the summary type carries no secret field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceListResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub total: usize,
    pub instances: Vec<ProviderInstanceSummary>,
}

/// `provider_instance_delete` request: remove the instance AND its keychain
/// key (the key is deleted FIRST — a failed keychain delete refuses the
/// whole operation and the metadata stays, so no orphaned secret remains).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceDeleteRequest {
    pub instance_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<Revision>,
}

/// Delete ack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceDeleteResponse {
    pub job_id: JobId,
    pub revision: Revision,
    pub removed_id: String,
    /// True when a keychain key existed and was removed.
    pub key_removed: bool,
}

/// `provider_instance_validate` request: FORM validation of a provider
/// configuration WITHOUT a network call and WITHOUT touching the keychain.
/// `has_key` is the caller's claim (the UI knows it from the list ack);
/// the cloud-without-key problem is reported against this claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceValidateRequest {
    pub preset: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<bool>,
    #[serde(default)]
    pub has_key: bool,
}

/// Validate ack: `ok` mirrors `problems.is_empty()`; every problem names the
/// exact form defect. This is a CONFIGURATION check — reachability/auth
/// probes would need network and are NOT part of this slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderInstanceValidateResponse {
    pub job_id: JobId,
    pub ok: bool,
    pub problems: Vec<String>,
}
