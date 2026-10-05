// RimLocClient — framework-neutral contract surface (UI R1: shared seam).
// Copied from the frozen Svelte client (frontend-v2) at React-lane bootstrap;
// the wire DTOs and op surface are IDENTICAL — the Svelte copy stays frozen
// as the legacy oracle, and this file is the live one going forward
// (FRONTEND_UI_BOUNDARY.md). Later: converge both on this module.
// RimLocClient — the ONE framework-neutral client surface over the binding
// contract (UI_SDK mandate / lead decisions 033 + 029-wave handoff).
//  - handshake checks ui_contract_version; mismatch = typed error;
//  - every mutating call carries expectedRevision + sessionEpoch;
//  - stale-epoch responses are typed rejections the store layer drops;
//  - save failures NEVER clear the caller's dirty draft: the typed
//    save_failed/stale_revision error flows back verbatim so the store can
//    keep the draft and offer retry (persist-before-ack semantics).
// Store binding is a LATER step (post contract accept); nothing here
// imports ../mock/* — the mock lives behind MockTransport only.
import type {
  ApplyExistingRequestDto,
  ApplyExistingResponseDto,
  BuildIdentityDto,
  ApplyIntentsRequestDto,
  ApplyIntentsResponseDto,
  BuildModProjectResponseDto,
  ContractErrorCode,
  ContractHandshakeDto,
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
  TranslationIntentDto,
  ValidateProjectResponseDto
} from './types';
import { UI_CONTRACT_VERSION } from './types';
import {
  createTauriTransport,
  type ContractMethodMap,
  type RimLocTransport,
  type TransportMode
} from './transport';

/** Typed contract failure — thrown by every client call. */
export class ContractClientError extends Error {
  readonly code: ContractErrorCode;
  readonly details?: unknown;
  constructor(code: ContractErrorCode, message: string, details?: unknown) {
    super(message);
    this.code = code;
    this.details = details;
  }
}

export function isContractErrorDto(e: unknown): e is ContractErrorCode extends never ? never : { code: string; message: string; details?: unknown } {
  return (
    typeof e === 'object' &&
    e !== null &&
    'code' in e &&
    typeof (e as { code: unknown }).code === 'string' &&
    'message' in e
  );
}

function toClientError(rejection: unknown): ContractClientError {
  if (isContractErrorDto(rejection)) {
    return new ContractClientError(rejection.code as ContractErrorCode, rejection.message, rejection.details);
  }
  return new ContractClientError('internal', rejection instanceof Error ? rejection.message : String(rejection));
}

export interface RimLocClientOptions {
  mode: TransportMode;
  /** Production: the Tauri invoke bridge. Ignored for the mock mode. */
  invoke?: unknown;
}

export class RimLocClient {
  readonly mode: TransportMode;
  private transport: RimLocTransport;
  private handshakeOk = false;

  constructor(options: RimLocClientOptions & { transport?: RimLocTransport }) {
    this.mode = options.mode;
    if (options.transport) {
      // Injected transport (tests, future UI lanes) — the contract surface
      // is the seam, not any concrete transport implementation.
      this.transport = options.transport;
    } else if (options.mode === 'tauri') {
      if (!options.invoke) {
        // Honest production failure: no silent mock fallback (lead 033/010).
        throw new ContractClientError('internal', 'Tauri invoke bridge is required for the live transport');
      }
      this.transport = createTauriTransport(options.invoke as Parameters<typeof createTauriTransport>[0]);
    } else {
      throw new ContractClientError(
        'internal',
        'mock transport is not bundled in the React lane yet — inject a transport or use the tauri bridge',
      );
    }
  }

  /** Handshake + version gate. MUST be called before the first data call. */
  async handshake(): Promise<ContractHandshakeDto> {
    const hs = await this.transport.call('contract_handshake', {}).catch((e: unknown) => {
      throw toClientError(e);
    });
    if (hs.ui_contract_version !== UI_CONTRACT_VERSION) {
      throw new ContractClientError('contract_violation', `ui_contract_version mismatch: backend ${hs.ui_contract_version}, client ${UI_CONTRACT_VERSION}`);
    }
    this.handshakeOk = true;
    return hs;
  }

  private async call<M extends Parameters<RimLocTransport['call']>[0]>(
    method: M,
    params: Parameters<RimLocTransport['call']>[1]
  ): Promise<ContractMethodMap[M]['result']> {
    if (!this.handshakeOk) {
      // enforce the handshake ordering (cheap: caches per client instance)
      await this.handshake();
    }
    return this.transport.call(method, params).catch((e: unknown) => {
      throw toClientError(e);
    });
  }

  async createProject(modRoot: string, targetVersion?: string): Promise<ProjectSnapshotDto> {
    const snap = await this.call('project_create', { request: { mod_root: { path: modRoot }, target_version: targetVersion } });
    return snap;
  }

  async openProject(projectId: string): Promise<ProjectSnapshotDto> {
    return this.call('project_open', { project_id: projectId });
  }

  async listProjects(): Promise<ProjectSummaryDto[]> {
    return this.call('project_list', {});
  }

  async snapshot(projectId: string): Promise<ProjectSnapshotDto> {
    return this.call('project_snapshot', { project_id: projectId });
  }

  /**
   * Apply typed intents with lost-update protection. Throws typed
   * ContractClientError on stale_epoch / stale_revision / save_failed —
   * the store keeps its dirty draft and retries; nothing is lost.
   */
  async applyIntents(args: {
    projectId: string;
    expectedRevision: number;
    sessionEpoch: number;
    intents: TranslationIntentDto[];
  }): Promise<ApplyIntentsResponseDto> {
    const req: ApplyIntentsRequestDto = {
      project_id: args.projectId,
      expected_revision: args.expectedRevision,
      session_epoch: args.sessionEpoch,
      intents: args.intents
    };
    return this.call('project_apply_intents', { request: req });
  }

  async refresh(projectId: string): Promise<ProjectSnapshotDto> {
    return this.call('project_refresh', { project_id: projectId });
  }

  async cancelNext(projectId: string): Promise<boolean> {
    return this.call('project_cancel_next', { project_id: projectId });
  }

  /** Read-only validation over the trusted session state (epoch-guarded).
   *  `status: 'failed'` = error-severity findings present. */
  async validateProject(
    projectId: string,
    sessionEpoch: number,
    locale?: string
  ): Promise<ValidateProjectResponseDto> {
    return this.call('project_validate', {
      project_id: projectId,
      session_epoch: sessionEpoch,
      ...(locale ? { locale } : {})
    });
  }

  /** Isolated native export into a CALLER-SPECIFIED out directory; the
   *  services guard refuses source-tree/managed-root targets and the
   *  result is reparse-verified before the ack. `locale` is the strict
   *  folder form ("Russian"). */
  async exportProject(
    projectId: string,
    sessionEpoch: number,
    outDir: string,
    locale: string
  ): Promise<ExportProjectResponseDto> {
    return this.call('project_export', {
      project_id: projectId,
      session_epoch: sessionEpoch,
      out_dir: outDir,
      locale
    });
  }

  /** Sanitized support bundle over the project's LAST FAILED operation. */
  async diagnoseProject(projectId: string, outDir: string): Promise<DiagnoseResponseDto> {
    return this.call('project_diagnose', { project_id: projectId, out_dir: outDir });
  }

  /** Dry-run analysis of an existing translation pack against the open
   *  project (READ-ONLY: nothing is written, no revision bump). The
   *  reusable set it reports is exactly what applyExisting applies. */
  async importExisting(request: ImportExistingRequestDto): Promise<ImportExistingResponseDto> {
    return this.call('project_import_existing', { request });
  }

  /** Apply the REUSABLE set of an analyzed pack into the open project
   *  (persist-before-ack). Existing translations are never overwritten;
   *  ambiguous lines are never auto-applied. */
  async applyExisting(request: ApplyExistingRequestDto): Promise<ApplyExistingResponseDto> {
    return this.call('project_apply_existing', { request });
  }

  /** Project glossary: the project's terms (wave 13, read-only). */
  async glossaryList(projectId: string, sessionEpoch: number): Promise<GlossaryTermDto[]> {
    return this.call('project_glossary', { project_id: projectId, session_epoch: sessionEpoch });
  }

  /** Create/update one glossary term (case-insensitive `term` match;
   *  persist-before-ack — the returned revision is durable). */
  async glossaryUpsert(
    request: ProjectGlossaryUpsertRequestDto
  ): Promise<ProjectGlossaryUpsertResponseDto> {
    return this.call('project_glossary_upsert', { request });
  }

  /** Remove one glossary term; an unknown term is a typed refusal. */
  async glossaryDelete(
    request: ProjectGlossaryDeleteRequestDto
  ): Promise<ProjectGlossaryDeleteResponseDto> {
    return this.call('project_glossary_delete', { request });
  }

  // --- translation memory (TM live, owner decision A+B+C) — the glossary
  // chain repeated: typed requests, persist-before-ack on every mutation. ---

  /** TM records with optional locale/status/query filters (read-only).
   *  `total` is the UNFILTERED count. */
  async tmList(request: TmListRequestDto): Promise<TmListResponseDto> {
    return this.call('project_tm_list', { request });
  }

  /** Manual CRUD write (C): create/update by (source_text, target_locale).
   *  Provenance is set by the service (MANUAL); omitted status → ACCEPTED.
   *  Persist-before-ack — the returned revision is durable. */
  async tmUpsert(request: TmUpsertRequestDto): Promise<TmUpsertResponseDto> {
    return this.call('project_tm_upsert', { request });
  }

  /** Remove one TM record by stable id; unknown id is a typed refusal. */
  async tmDelete(request: TmDeleteRequestDto): Promise<TmDeleteResponseDto> {
    return this.call('project_tm_delete', { request });
  }

  /** Bulk import (B): JSON array or CSV lines; DRAFT by default, an
   *  explicit row status IS honored. Never weakens a stronger record.
   *  Per-row form refusals are counted in `rejected`, never fatal. */
  async tmImport(request: TmImportRequestDto): Promise<TmImportResponseDto> {
    return this.call('project_tm_import', { request });
  }

  /** Ranked candidates within ONE target locale (isolation is mandatory):
   *  exact → normalized → bounded fuzzy, best-first. */
  async tmLookup(request: TmLookupRequestDto): Promise<TmLookupResponseDto> {
    return this.call('project_tm_lookup', { request });
  }

  /** Identity of the RUNNING binary (soak-hardening §1): acceptance
   * preflight compares this against the expected source commit BEFORE any
   * cycles — a wrong artifact aborts, never soaks silently. */
  async buildIdentity(): Promise<BuildIdentityDto> {
    return this.call('build_identity', {});
  }

  /** FULL drop-in mod package (`About/About.xml` in the game-loadable
   *  `<ModMetaData>` shape + `Languages/<locale>`) into a CALLER-SPECIFIED
   *  out directory — the folder can move straight into the game's Mods
   *  directory. The services guard partition is identical to exportProject
   *  and the result is reparse-verified before the ack. */
  async buildModProject(
    projectId: string,
    sessionEpoch: number,
    outDir: string,
    locale: string
  ): Promise<BuildModProjectResponseDto> {
    return this.call('project_build_mod', {
      project_id: projectId,
      session_epoch: sessionEpoch,
      out_dir: outDir,
      locale
    });
  }

  /** Native OS folder dialog (main.rs pick_directory → blocking_pick_folder).
   *  Resolves the picked ABSOLUTE path, or null when the user cancelled —
   *  null is a normal outcome, never an error. In mock mode this rejects
   *  with the honest `unsupported_capability` refusal: there is no OS dialog
   *  without the desktop bridge, and the mock never fakes one. */
  async pickDirectory(initial?: string): Promise<string | null> {
    return this.call('pick_directory', initial ? { initial } : {});
  }

  /** Self-localization entry (mandate D): absolute path of the app-bundled
   * RimLoc UI catalog, prepared as an ORDINARY project source directory —
   * feed it straight into createProject() and the existing ui_catalog
   * adapter routes it (no special-cased client flow). Failure is a typed
   * error, never a fabricated path; in mock mode this rejects with the
   * honest `unsupported_capability` refusal. */
  async selflocCatalogDir(): Promise<string> {
    return this.call('selfloc_catalog_dir', {});
  }

  /** Self-localization contribution (beta): build the offline translation
   * bundle from the OPEN session (the RimLoc UI catalog project) into the
   * CALLER-SPECIFIED out directory. The services guard refuses relative
   * paths and source-tree/managed-root targets; NEEDS-FIXES writes nothing
   * and carries the enumerated refusals. */
  async selflocBuildContribution(
    projectId: string,
    sessionEpoch: number,
    outDir: string,
    locale: string
  ): Promise<SelflocBuildContributionResponseDto> {
    return this.call('selfloc_build_contribution', {
      project_id: projectId,
      session_epoch: sessionEpoch,
      out_dir: outDir,
      locale
    });
  }
}

/** Explicit client factory — the ONLY place a mode is chosen. Production
 *  code must construct mode:'tauri' with the real invoke; dev/demo entries
 *  may construct mode:'mock'. There is no implicit default. */
export function createRimLocClient(options: RimLocClientOptions): RimLocClient {
  return new RimLocClient(options);
}
