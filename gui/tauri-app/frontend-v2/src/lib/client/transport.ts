// Transport seam of the binding contract (UI_SDK mandate: ONE transport).
// The production entry uses TauriTransport (the single `invoke` bridge);
// the dev/demo entry uses MockTransport — deterministic, no backend. The
// mode choice is EXPLICIT at client construction; the mock is never the
// production default (MOCK_LIVE_ONBOARDING §2, lead decision 010).
import type {
  ApplyIntentsRequestDto,
  ApplyIntentsResponseDto,
  ContractHandshakeDto,
  ContractMethod,
  CreateProjectRequestDto,
  ProjectSnapshotDto,
  ProjectSummaryDto
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
