// Providers store (React lane, provider/settings parity) — LIVE contract
// state: provider_instance_list / upsert / delete / validate / test through
// RimLocClient (the Glossary/TM pattern, persist-before-ack on mutations).
// The API key NEVER lives in this store: an upsert transports it once into
// the OS keychain; everything this store holds is the REDACTED summary
// (has_key boolean, no secret field exists on the wire type).
//
// §7 F7.1 — the honest connection axis: a probe RESULT (typed, from
// contract_provider_instance_test) is the only thing that can claim
// `connected`; a config change INVALIDATES the last probe back to
// `connection_unknown`. Secrets never enter this module.

import type {
  ProviderInstanceSummaryDto,
  ProviderInstanceUpsertRequestDto,
  ProviderInstanceValidateRequestDto,
  ProviderInstanceValidateResponseDto,
  ProviderInstanceTestRequestDto,
  ProviderInstanceTestResponseDto,
  ProviderTestStatusDto,
} from '../client/types'
import { clientInstance } from '../client/instance'

/** §7 F7.1 connection vocabulary. Outcome states come ONLY from a real
 *  probe result; the lifecycle states are frontend-side:
 *  - configured: metadata present (has_key or local), never probed;
 *  - connection_unknown: config changed since the last probe (upsert);
 *  - testing: probe in flight;
 *  - local_offline: display fold of local && network_failed. */
export type ProviderConnection =
  | 'configured'
  | 'connection_unknown'
  | 'testing'
  | 'connected'
  | 'auth_failed'
  | 'network_failed'
  | 'model_not_found'
  | 'rate_limited'
  | 'server_error'
  | 'local_offline'

/** A settled probe outcome keyed by probe target (instance id, or 'new'
 *  while the form holds an unsaved provider). `detail` is the sanitized
 *  provider text — no secret can be in it (there is no wire field). */
export interface ProbeOutcome {
  status: ProviderTestStatusDto
  detail?: string | null
  endpoint?: string
}

export interface ProvidersState {
  instances: ProviderInstanceSummaryDto[]
  revision: number
  loaded: boolean
  /** Honest transport failure (no Tauri bridge, backend error) — rendered, never hidden. */
  error: string | null
  /** Settled probe results per target id ('new' = the unsaved form). */
  probes: Record<string, ProbeOutcome>
  /** Instance ids with a probe currently in flight. */
  probing: Record<string, true>
}

type Listener = () => void

/** Frontend-side template knowledge for the NEW-provider form (the backend
 *  validates the real thing; these are just the starting values). */
export const PROVIDER_TEMPLATES: {
  preset: string
  models: string[]
  baseUrl: string
  local: boolean
}[] = [
  { preset: 'zai', models: ['glm-4.6', 'glm-4.5-air'], baseUrl: 'https://api.z.ai/api/anthropic', local: false },
  { preset: 'openai', models: ['gpt-4o', 'gpt-4o-mini'], baseUrl: 'https://api.openai.com/v1', local: false },
  { preset: 'anthropic', models: ['claude-sonnet-4', 'claude-haiku-4'], baseUrl: 'https://api.anthropic.com', local: false },
  { preset: 'ollama', models: ['llama3.1:8b', 'qwen2.5:7b'], baseUrl: 'http://localhost:11434/v1', local: true },
  { preset: 'custom', models: ['custom-model'], baseUrl: 'https://your-endpoint.example/v1', local: false },
]

/** Probe target id of the detail form: the selected instance, or 'new'. */
export const FORM_PROBE_TARGET = (selectedId: string | null): string => selectedId ?? 'new'

/** §7 F7.1 display fold: for a LOCAL provider a transport failure IS the
 *  offline state (the server on this machine is simply not running). */
function foldLocal(status: ProviderTestStatusDto, local: boolean): ProviderConnection {
  return local && status === 'network_failed' ? 'local_offline' : status
}

/** The display connection of a SAVED instance: probe outcome first
 *  (connected/failures are only claimable through it), then the lifecycle
 *  states. The backend refuses keyless cloud instances, so every persisted
 *  instance is at least `configured`. */
export function connectionOf(p: ProviderInstanceSummaryDto, probes: Record<string, ProbeOutcome>, probing: Record<string, true>): ProviderConnection {
  if (probing[p.id]) return 'testing'
  const probe = probes[p.id]
  if (probe) return foldLocal(probe.status, p.local)
  return 'configured'
}

class ProviderInstancesStore {
  private state: ProvidersState = {
    instances: [],
    revision: 0,
    loaded: false,
    error: null,
    probes: {},
    probing: {},
  }
  private listeners = new Set<Listener>()

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  getSnapshot(): ProvidersState {
    return this.state
  }

  private set(patch: Partial<ProvidersState>): void {
    this.state = { ...this.state, ...patch }
    this.listeners.forEach((l) => l())
  }

  /** Redacted list from the live contract (has_key only, never the key).
   *  Probe outcomes are session-local knowledge — a reload never clobbers
   *  them, but outcomes of DELETED instances are dropped. */
  async reload(): Promise<void> {
    try {
      const res = await clientInstance.getClient().providerInstanceList()
      const live = new Set(res.instances.map((p) => p.id))
      const probes: Record<string, ProbeOutcome> = {}
      for (const [id, outcome] of Object.entries(this.state.probes)) {
        if (id === 'new' || live.has(id)) probes[id] = outcome
      }
      this.set({
        instances: res.instances,
        revision: res.revision,
        loaded: true,
        error: null,
        probes,
      })
    } catch (e) {
      this.set({ loaded: true, error: errorText(e) })
    }
  }

  /** Create/edit; `secret` (when present) goes once into the OS keychain.
   *  §7 F7.1 invalidation: ANY upsert of model/base_url/key drops the
   *  target's last probe outcome back to `connection_unknown` — a stored
   *  verdict about the OLD config must never display as current truth. */
  async upsert(req: ProviderInstanceUpsertRequestDto): Promise<void> {
    const res = await clientInstance.getClient().providerInstanceUpsert(req)
    await this.reload()
    const target = res.instance.id
    if (this.state.probes[target]) {
      const probes = { ...this.state.probes }
      delete probes[target]
      this.set({ probes })
    }
  }

  /** Remove the instance AND its keychain key (backend deletes key first). */
  async remove(instanceId: string): Promise<void> {
    await clientInstance
      .getClient()
      .providerInstanceDelete({ instance_id: instanceId })
    const probes = { ...this.state.probes }
    delete probes[instanceId]
    const probing = { ...this.state.probing }
    delete probing[instanceId]
    await this.reload()
    this.set({ probes, probing })
  }

  /** Typed form validation — no network, no keychain access. */
  validate(
    req: ProviderInstanceValidateRequestDto,
  ): Promise<ProviderInstanceValidateResponseDto> {
    return clientInstance.getClient().providerInstanceValidate(req)
  }

  /** The REAL bounded probe (§7 F7.1/F7.2): one tiny prompt to the
   *  provider, typed outcome. The probe-only `secret` crosses ONCE here
   *  and is never stored in this module. The outcome is keyed by the probe
   *  target so both the card badge and the form row display it. */
  async testConnection(
    target: string,
    req: ProviderInstanceTestRequestDto,
  ): Promise<ProviderInstanceTestResponseDto> {
    this.set({ probing: { ...this.state.probing, [target]: true } })
    try {
      const res = await clientInstance.getClient().providerInstanceTest(req)
      const probes = { ...this.state.probes }
      probes[target] = { status: res.status, detail: res.detail ?? null, endpoint: res.endpoint }
      this.set({ probes })
      return res
    } finally {
      const probing = { ...this.state.probing }
      delete probing[target]
      this.set({ probing })
    }
  }
}

export function errorText(e: unknown): string {
  const dto = e as { code?: string; message?: string }
  if (dto && typeof dto.message === 'string') {
    return dto.code ? `${dto.code}: ${dto.message}` : dto.message
  }
  return e instanceof Error ? e.message : String(e)
}

export const providers = new ProviderInstancesStore()
