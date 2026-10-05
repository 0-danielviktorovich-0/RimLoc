// Transport seam of the binding contract (UI_SDK mandate: ONE transport).
// The production entry uses TauriTransport (the single `invoke` bridge);
// the dev/demo entry uses MockTransport — deterministic, no backend. The
// mode choice is EXPLICIT at client construction; the mock is never the
// production default (MOCK_LIVE_ONBOARDING §2, lead decision 010).
import type {
  ApplyExistingRequestDto,
  ApplyExistingResponseDto,
  ApplyIntentsRequestDto,
  BuildIdentityDto,
  ApplyIntentsResponseDto,
  BuildModProjectResponseDto,
  ContractHandshakeDto,
  ContractMethod,
  CreateProjectRequestDto,
  DiagnoseResponseDto,
  ExportProjectResponseDto,
  GlossaryTermDto,
  ImportExistingRequestDto,
  ImportExistingResponseDto,
  ProjectGlossaryDeleteRequestDto,
  ProjectGlossaryDeleteResponseDto,
  ProjectGlossaryUpsertRequestDto,
  ProjectGlossaryUpsertResponseDto,
  ProjectSnapshotDto,
  ProjectSummaryDto,
  SelflocBuildContributionResponseDto,
  TmDeleteRequestDto,
  TmDeleteResponseDto,
  TmImportRequestDto,
  TmImportResponseDto,
  TmListRequestDto,
  TmListResponseDto,
  TmLookupRequestDto,
  TmLookupResponseDto,
  TmUpsertRequestDto,
  TmUpsertResponseDto,
  ValidateProjectResponseDto
} from './types';

/** Method params/result map — the whole wire surface, statically. */
export interface ContractMethodMap {
  contract_handshake: { params: Record<string, never>; result: ContractHandshakeDto };
  project_create: { params: { request: CreateProjectRequestDto }; result: ProjectSnapshotDto };
  project_open: { params: { project_id: string }; result: ProjectSnapshotDto };
  project_list: { params: Record<string, never>; result: ProjectSummaryDto[] };
  project_snapshot: { params: { project_id: string }; result: ProjectSnapshotDto };
  project_apply_intents: { params: { request: ApplyIntentsRequestDto }; result: ApplyIntentsResponseDto };
  project_refresh: { params: { project_id: string }; result: ProjectSnapshotDto };
  project_cancel_next: { params: { project_id: string }; result: boolean };
  // final night wave: validate/build/diagnostics over the contract
  project_validate: {
    params: { project_id: string; session_epoch: number; locale?: string };
    result: ValidateProjectResponseDto;
  };
  project_export: {
    params: { project_id: string; session_epoch: number; out_dir: string; locale: string };
    result: ExportProjectResponseDto;
  };
  project_build_mod: {
    params: { project_id: string; session_epoch: number; out_dir: string; locale: string };
    result: BuildModProjectResponseDto;
  };
  project_diagnose: {
    params: { project_id: string; out_dir: string };
    result: DiagnoseResponseDto;
  };
  // Native folder dialog (main.rs pick_directory). `initial` seeds the
  // dialog's start directory; null result = the user cancelled. No OS
  // dialog exists outside the desktop bridge — the mock refuses honestly.
  pick_directory: { params: { initial?: string }; result: string | null };
  // Self-localization entry (mandate D): absolute path of the app-managed
  // RimLoc UI catalog project dir (mod_root for project_create). Refused
  // honestly in mock — there is no bundled catalog outside the desktop app.
  selfloc_catalog_dir: { params: Record<string, never>; result: string };
  // Self-localization contribution (beta, wave 7): build the offline
  // translation bundle from the OPEN session into a caller-chosen
  // directory. The services layer owns the §6 gate and the readiness
  // statuses; refused honestly in mock (no backend session there).
  selfloc_build_contribution: {
    params: { project_id: string; session_epoch: number; out_dir: string; locale: string };
    result: SelflocBuildContributionResponseDto;
  };
  // Existing translation pack (W2): dry-run analysis + separate guarded
  // application against the open project.
  project_import_existing: {
    params: { request: ImportExistingRequestDto };
    result: ImportExistingResponseDto;
  };
  project_apply_existing: {
    params: { request: ApplyExistingRequestDto };
    result: ApplyExistingResponseDto;
  };
  // Project glossary (wave 13): generic project state, persist-before-ack.
  project_glossary: {
    params: { project_id: string; session_epoch: number };
    result: GlossaryTermDto[];
  };
  project_glossary_upsert: {
    params: { request: ProjectGlossaryUpsertRequestDto };
    result: ProjectGlossaryUpsertResponseDto;
  };
  project_glossary_delete: {
    params: { request: ProjectGlossaryDeleteRequestDto };
    result: ProjectGlossaryDeleteResponseDto;
  };
  // Translation memory (TM live, owner decision A+B+C): the glossary
  // pattern again — generic project state, persist-before-ack.
  project_tm_list: {
    params: { request: TmListRequestDto };
    result: TmListResponseDto;
  };
  project_tm_upsert: {
    params: { request: TmUpsertRequestDto };
    result: TmUpsertResponseDto;
  };
  project_tm_delete: {
    params: { request: TmDeleteRequestDto };
    result: TmDeleteResponseDto;
  };
  project_tm_import: {
    params: { request: TmImportRequestDto };
    result: TmImportResponseDto;
  };
  project_tm_lookup: {
    params: { request: TmLookupRequestDto };
    result: TmLookupResponseDto;
  };
  build_identity: { params: Record<string, never>; result: BuildIdentityDto };
}

export type TransportMode = 'tauri' | 'mock';

export interface RimLocTransport {
  readonly mode: TransportMode;
  call<M extends ContractMethod>(
    method: M,
    params: ContractMethodMap[M]['params']
  ): Promise<ContractMethodMap[M]['result']>;
}

/** Production transport: the ONLY invoke bridge of the app. */
export interface TauriInvokeLike {
  <T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

export function createTauriTransport(invoke: TauriInvokeLike): RimLocTransport {
  return {
    mode: 'tauri',
    async call<M extends ContractMethod>(
      method: M,
      params: ContractMethodMap[M]['params']
    ): Promise<ContractMethodMap[M]['result']> {
      // Tauri v2 camelCases arg keys; the contract DTOs are snake_case, so
      // the request object is passed under the documented arg names as-is.
      return invoke<ContractMethodMap[M]['result']>(method, params);
    }
  };
}
