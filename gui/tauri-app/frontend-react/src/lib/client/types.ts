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
  | 'invalid_config'
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
  /** Provenance of the batch; absent = human (older clients, the GUI). */
  origin?: 'human' | 'llm' | 'tm' | 'import';
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
  | 'project_apply_existing'
  // Project glossary (wave 13): generic project state, persist-before-ack.
  | 'project_glossary'
  | 'project_glossary_upsert'
  | 'project_glossary_delete'
  // Translation memory (TM live, owner decision A+B+C): the glossary
  // pattern again — generic project state, persist-before-ack.
  | 'project_tm_list'
  | 'project_tm_upsert'
  | 'project_tm_delete'
  | 'project_tm_import'
  | 'project_tm_lookup'
  // Provider instances (provider/settings parity): app-global CRUD; the API
  // key never persists outside the OS keychain.
  | 'contract_provider_instance_list'
  | 'contract_provider_instance_upsert'
  | 'contract_provider_instance_delete'
  | 'contract_provider_instance_validate'
  // Build identity of the RUNNING binary (soak-hardening §1): long-running
  // acceptance runs verify the artifact they drive independently of any
  // wrapper path. Refused honestly in mock — no running binary there.
  | 'build_identity';

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

// --- project glossary (wave 13; mirrors contract.rs ProjectGlossary* DTOs).
// The glossary is GENERIC project state (adapter-independent): term is
// unique per project CASE-INSENSITIVE — an upsert with different casing
// updates the existing entry in place (stable id survives). ---
export interface GlossaryTermDto {
  id: string;
  term: string;
  translation: string;
  note?: string;
}

export interface ProjectGlossaryUpsertRequestDto {
  project_id: string;
  session_epoch: number;
  term: string;
  translation: string;
  note?: string;
}

export interface ProjectGlossaryUpsertResponseDto {
  job_id: string;
  revision: number;
  entry: GlossaryTermDto;
}

export interface ProjectGlossaryDeleteRequestDto {
  project_id: string;
  session_epoch: number;
  term: string;
}

export interface ProjectGlossaryDeleteResponseDto {
  job_id: string;
  revision: number;
  removed_id: string;
}

// --- translation memory (TM live, owner decision A+B+C; mirrors contract.rs
// Tm* DTOs). Records live in `Project.tm` (the glossary pattern): key =
// (source_text, target_locale), target locales are ISOLATED, trust status
// DRAFT < ACCEPTED < REVIEWED, provenance AUTO | IMPORT | MANUAL. A =
// auto-accumulation on the acked accept; B = import (DRAFT by default);
// C = manual CRUD. ---
export type TmStatusDto = 'draft' | 'accepted' | 'reviewed';

export type TmProvenanceDto = 'auto' | 'import' | 'manual';

export interface TranslationMemoryEntryDto {
  id: string;
  source_text: string;
  target_text: string;
  /** Target locale (folder contract, e.g. "Russian") — the ISOLATION key. */
  target_locale: string;
  status: TmStatusDto;
  provenance: TmProvenanceDto;
}

export interface TmListRequestDto {
  project_id: string;
  session_epoch: number;
  locale?: string;
  status?: TmStatusDto;
  /** Case-insensitive substring over source/target. */
  query?: string;
}

export interface TmListResponseDto {
  job_id: string;
  entries: TranslationMemoryEntryDto[];
  /** UNFILTERED total — the UI renders an honest "N of M". */
  total: number;
}

export interface TmUpsertRequestDto {
  project_id: string;
  session_epoch: number;
  source_text: string;
  target_text: string;
  target_locale: string;
  /** User's choice; omitted → ACCEPTED. Provenance is set by the service. */
  status?: TmStatusDto;
}

export interface TmUpsertResponseDto {
  job_id: string;
  revision: number;
  entry: TranslationMemoryEntryDto;
}

export interface TmDeleteRequestDto {
  project_id: string;
  session_epoch: number;
  id: string;
}

export interface TmDeleteResponseDto {
  job_id: string;
  revision: number;
  removed_id: string;
}

export type TmImportFormatDto = 'json' | 'csv';

export interface TmImportRequestDto {
  project_id: string;
  session_epoch: number;
  /** Raw payload (JSON array or CSV text); the format auto-detects unless
   *  forced. A payload that parses as NEITHER is a typed contract_violation. */
  payload: string;
  format?: TmImportFormatDto;
}

export interface TmImportResponseDto {
  job_id: string;
  revision: number;
  /** Fresh keys. */
  imported: number;
  /** Existing weaker (DRAFT) records refreshed. */
  updated: number;
  /** Existing stronger records kept untouched (never weakened). */
  skipped: number;
  /** Per-row form refusals (counted, never fatal). */
  rejected: number;
}

export interface TmLookupRequestDto {
  project_id: string;
  session_epoch: number;
  source_text: string;
  target_locale: string;
  /** Max matches (default 5, clamped 1..=50). */
  limit?: number;
}

/** One ranked match: `match_kind` names the tier that found it. */
export interface TmMatchDto {
  entry: TranslationMemoryEntryDto;
  match_kind: 'exact' | 'normalized' | 'fuzzy';
  /** Levenshtein distance of the NORMALIZED forms; only on the fuzzy tier. */
  distance?: number;
}

export interface TmLookupResponseDto {
  job_id: string;
  matches: TmMatchDto[];
}

/// Identity of the running binary, reported by the app itself
/// (build.rs RIMLOC_* env, never derived from wrapper paths). Wire shape
/// follows the shell-info precedent (AppInfo): camelCase.
export interface BuildIdentityDto {
  sourceCommit: string;
  buildProfile: string;
  buildFeatures: string;
  appVersion: string;
}

// --- provider instances (provider/settings parity) — the API key NEVER
// rides this surface: the secret goes straight to the OS keychain and the
// summary carries only the has_key boolean (redaction by construction). ---
export type ProviderPresetId = 'anthropic' | 'openai' | 'zai' | 'ollama' | 'custom';

/** REDACTED summary: no field can carry a secret. */
export interface ProviderInstanceSummaryDto {
  id: string;
  preset: string;
  label: string;
  model: string;
  base_url?: string;
  local: boolean;
  has_key: boolean;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface ProviderInstanceUpsertRequestDto {
  instance_id?: string;
  preset: string;
  label?: string;
  model: string;
  base_url?: string;
  /** Transported ONCE on this call into the OS keychain; never echoed. */
  secret?: string;
  local?: boolean;
  expected_revision?: number;
}

export interface ProviderInstanceUpsertResponseDto {
  job_id: string;
  revision: number;
  instance: ProviderInstanceSummaryDto;
}

export interface ProviderInstanceListResponseDto {
  job_id: string;
  revision: number;
  total: number;
  instances: ProviderInstanceSummaryDto[];
}

export interface ProviderInstanceDeleteRequestDto {
  instance_id: string;
  expected_revision?: number;
}

export interface ProviderInstanceDeleteResponseDto {
  job_id: string;
  revision: number;
  removed_id: string;
  key_removed: boolean;
}

/** Form validation WITHOUT network and WITHOUT keychain access. */
export interface ProviderInstanceValidateRequestDto {
  preset: string;
  model: string;
  base_url?: string;
  local?: boolean;
  has_key: boolean;
}

export interface ProviderInstanceValidateResponseDto {
  job_id: string;
  ok: boolean;
  problems: string[];
}
