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

export interface DiagnoseResponseDto {
  job_id: string;
  /** Sanitized bundle directory (outside the read-only source tree). */
  bundle_dir: { path: string };
  operation_id: string;
  files: string[];
  redacted_count: number;
  excluded_count: number;
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
  | 'project_diagnose';
