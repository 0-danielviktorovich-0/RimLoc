// Wire-DTO mirrors of the binding contract (rimloc-services/src/contract.rs,
// UI_CONTRACT_VERSION = 1). Kept in sync by the handshake check: the client
// refuses to operate against a backend reporting a different version.
// Field names mirror the serde serialization (snake_case) — do not rename.

export const UI_CONTRACT_VERSION = 1;

// --- stable error codes (append-only; never rename) ---
export type ContractErrorCode =
  | 'stale_epoch'
  | 'stale_revision'
  | 'save_failed'
  | 'project_changed_on_disk'
  | 'contract_violation'
  | 'guard_output_denied'
  | 'invalid_output_path'
  | 'unsupported_capability'
  | 'project_not_found'
  | 'schema_version'
  | 'validation_failed'
  | 'internal';

export interface ContractErrorDto {
  code: ContractErrorCode;
  message: string;
  details?: unknown;
}

// --- identities ---
/** Full structural identity (kind + key [+ defType discriminator]). */
export interface SourceEntryIdDto {
  kind: string;
  key: string;
  def_type?: string;
}

export interface TranslationIntentDto {
  entry: SourceEntryIdDto;
  /** Target locale, folder contract ("Russian"). */
  locale: string;
  action: 'set_translation' | 'mark_todo' | 'clear_translation';
  /** New text for set_translation (required there, ignored otherwise). */
  text?: string;
}

export interface SkippedIntentDto {
  index: number;
  code: ContractErrorCode;
  message: string;
}

// --- requests / responses ---
export interface CreateProjectRequestDto {
  mod_root: { path: string };
  target_version?: string;
}

export interface ApplyIntentsRequestDto {
  project_id: string;
  /** Lost-update guard: the revision the edits were based on. */
  expected_revision: number;
  /** Stale-session guard: the epoch of the caller's open session. */
  session_epoch: number;
  intents: TranslationIntentDto[];
}

export interface ApplyIntentsResponseDto {
  job_id: string;
  revision: number;
  applied: number;
  skipped: SkippedIntentDto[];
  cancelled: boolean;
}

// --- project state (read-only DTO of the canonical domain Project) ---
/** Canonical source entry: source fields are read-only by contract. */
export interface SourceEntryDto {
  id: SourceEntryIdDto;
  text: string;
  source_locale: string;
  tkey?: unknown;
  /** Additive (contract, wave 12): live Source Inspector projection — the
   *  EFFECTIVE context's file RELATIVE to the project root, the
   *  parser-guaranteed line (absent = unknown, never faked) and the winner
   *  reason (rimloc winner_reason vocabulary). Absent on legacy snapshots
   *  and whenever the backend cannot honestly project it. */
  source_ref?: EntrySourceRefDto;
}

export interface EntrySourceRefDto {
  file: string;
  line?: number;
  selected_by: string;
}

export interface TranslationDto {
  source_id: SourceEntryIdDto;
  locale: string;
  text: string | null;
  completeness: string;
  review: string;
  /** ValidationState on the wire (serde, externally tagged):
   *  'unknown' | 'ok' | { issues: string[] }. */
  validation: 'unknown' | 'ok' | { issues: string[] };
  lifecycle: string;
  /** Origin on the wire (serde snake_case): 'unknown'|'human'|'tm'|'llm'|'imported'. */
  origin: string;
  notes?: string;
}

export interface CanonicalProjectDto {
  context: {
    target_version?: string | null;
    active_dlc: string[];
    active_mods: string[];
    load_order: string[];
    view: string;
  };
  entries: SourceEntryDto[];
  translations: TranslationDto[];
}

export interface ProjectSnapshotDto {
  project_id: string;
  revision: number;
  session_epoch: number;
  /** Additive v2 (contract.rs): in-memory edits applied but NOT durably
   *  acked (save_failed / external change). `refresh` DISCARDS backend
   *  dirty edits (disk wins) — the client must gate refresh behind an
   *  explicit confirmation. */
  dirty?: boolean;
  /** Additive v2 (contract.rs): the last durably ACKED revision — the
   *  correct `expected_revision` base for apply while dirty. */
  acked_revision?: number;
  /** Additive (M-7): read-only source mod root the session holds; absent
   *  on legacy envelopes — UI must show an honest unknown, never a
   *  template placeholder. */
  source_root?: { path: string };
  project: CanonicalProjectDto;
}

export interface ProjectSummaryDto {
  project_id: string;
  name: string;
  revision: number;
  target_version?: string;
}

// --- validate / export / diagnose (final night wave; mirrors contract.rs
// ValidationFinding / ValidateProjectResponse / ExportProjectResponse /
// DiagnoseResponse) ---
/** One typed validation finding; `id` carries the FULL structural identity
 *  when the finding resolves to an inventory entry. */
export interface ValidationFindingDto {
  id?: SourceEntryIdDto;
  severity: 'error' | 'warning' | 'info';
  kind: string;
  key: string;
  path: string;
  line?: number;
  message: string;
}

export interface ValidateProjectResponseDto {
  job_id: string;
  /** 'failed' only when error-severity findings exist (026 semantics). */
  status: 'succeeded' | 'failed';
  findings: ValidationFindingDto[];
  error_count: number;
  warning_count: number;
  info_count: number;
  locale?: string;
}

export interface ExportProjectResponseDto {
  job_id: string;
  out_dir: { path: string };
  files_written: number;
  /** Keys the EXISTING scanner re-parsed from the written output. */
  reparsed_keys: number;
  skipped_unknown_type: string[];
}

/** `project_build_mod`: the FULL drop-in mod package (`About/About.xml` in
 *  the game-loadable `<ModMetaData>` shape + `Languages/<locale>`) — same
 *  DTO pattern as the export; the differences live in the output, not the
 *  report. */
export interface BuildModProjectResponseDto {
  job_id: string;
  out_dir: { path: string };
  files_written: number;
  reparsed_keys: number;
  skipped_unknown_type: string[];
}

export interface DiagnoseResponseDto {
  job_id: string;
  /** Sanitized bundle directory (outside the read-only source tree). */
  bundle_dir: { path: string };
  operation_id: string;
  files: string[];
  redacted_count: number;
  excluded_count: number;
}

// --- existing translation pack (W2; mirrors contract.rs Import/Apply
// Existing DTOs). Analysis is DRY-RUN ONLY; application is a SEPARATE,
// fully guarded command. ---
export interface ExistingMatchItemDto {
  /** The pack's serialization key. */
  key: string;
  /** FULL structural identity the line addresses (reusable / conflicts). */
  entry?: SourceEntryIdDto;
}

export interface ExistingAmbiguousItemDto {
  key: string;
  /** Candidate source keys — reported for review, never auto-applied. */
  candidates: string[];
}

export interface ImportExistingRequestDto {
  project_id: string;
  session_epoch: number;
  /** Absolute pack directory (typically Languages/<locale>). */
  existing_dir: { path: string };
  /** Target locale, strict language-folder form ("Russian"). */
  locale: string;
}

export interface ImportExistingResponseDto {
  job_id: string;
  scanned_files: number;
  scanned_keys: number;
  reusable_count: number;
  conflict_count: number;
  obsolete_count: number;
  ambiguous_count: number;
  invalid_count: number;
  /** Inventory strings that stay untranslated after the merge. */
  new_count: number;
  /** Capped sample lists (backend EXISTING_LIST_LIMIT). */
  reusable: ExistingMatchItemDto[];
  conflicts: ExistingMatchItemDto[];
  obsolete: ExistingMatchItemDto[];
  ambiguous: ExistingAmbiguousItemDto[];
  invalid: ExistingMatchItemDto[];
}

export interface ApplyExistingRequestDto {
  project_id: string;
  expected_revision: number;
  session_epoch: number;
  existing_dir: { path: string };
  locale: string;
}

export interface ApplyExistingResponseDto {
  job_id: string;
  revision: number;
  applied: number;
  /** Existing translations NOT overwritten. */
  conflicts: number;
  /** Pack lines addressing nothing in the inventory (not applied). */
  unmatched: number;
  /** Ambiguous lines (not applied). */
  ambiguous: number;
}

// --- handshake / capabilities ---
export interface UnsupportedCapabilityDto {
  capability: string;
  reason: string;
}

export interface CapabilityReportDto {
  contract_version: number;
  supported: string[];
  unsupported: UnsupportedCapabilityDto[];
}

export interface ContractHandshakeDto {
  ui_contract_version: number;
  capabilities: CapabilityReportDto;
}

/** Transport method names — mirror the Tauri command names 1:1. */
export type ContractMethod =
  | 'contract_handshake'
  | 'project_create'
  | 'project_open'
  | 'project_list'
  | 'project_snapshot'
  | 'project_apply_intents'
  | 'project_refresh'
  | 'project_cancel_next'
  | 'project_validate'
  | 'project_export'
  | 'project_build_mod'
  | 'project_diagnose'
  // Window-level native folder dialog (main.rs pick_directory, NOT a
  // contract_adapter op): exposed through the same typed surface so the UI
  // fills absolute paths from the OS dialog instead of hand-typing them.
  // The mock transport refuses it honestly — no OS dialog exists there.
  | 'pick_directory'
  // Self-localization entry (mandate D, shell-level like pick_directory):
  // resolves the app-bundled RimLoc UI catalog as an ordinary project
  // source dir (mod_root for the EXISTING contract create flow). The mock
  // transport refuses it honestly — no bundled catalog exists there.
  | 'selfloc_catalog_dir'
  // Self-localization contribution (beta, wave 7): build the offline
  // translation bundle from the OPEN RimLoc UI catalog session into a
  // caller-chosen directory. The mock transport refuses it honestly —
  // no backend session exists there.
  | 'selfloc_build_contribution'
  // Existing translation pack (W2): dry-run analysis + separate guarded
  // application against the open project.
  | 'project_import_existing'
  | 'project_apply_existing';

/** Readiness status of the contribution bundle build (services
 * `rimloc_services::contribution::BundleStatus`): READY (everything valid) /
 * PARTIAL-BUT-VALID (valid subset bundled, refusals enumerated) /
 * NEEDS-FIXES (nothing written, exact blocker list). */
export type ContributionBuildStatus = 'READY' | 'PARTIAL-BUT-VALID' | 'NEEDS-FIXES';

/** One §6-gate refusal of the contribution build — a change id (or
 * `<root>` for structural blockers) plus a translator-actionable reason.
 * Secret hits name the PATTERN, never the matched text. */
export interface ContributionRejectionDto {
  id: string;
  reason: string;
}

/** Response of selfloc_build_contribution. `bundle_path` is null exactly
 * when the status is NEEDS-FIXES (no file is written). */
export interface SelflocBuildContributionResponseDto {
  status: ContributionBuildStatus;
  bundle_path: string | null;
  accepted_count: number;
  rejected: ContributionRejectionDto[];
}
