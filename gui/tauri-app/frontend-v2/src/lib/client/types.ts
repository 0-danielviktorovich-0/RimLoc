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
  validation: string;
  lifecycle: string;
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
  project: CanonicalProjectDto;
}

export interface ProjectSummaryDto {
  project_id: string;
  name: string;
  revision: number;
  target_version?: string;
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
  | 'project_cancel_next';
