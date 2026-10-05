// Providers store (React lane, provider/settings parity) — LIVE contract
// state: provider_instance_list / upsert / delete / validate through
// RimLocClient (the Glossary/TM pattern, persist-before-ack on mutations).
// The API key NEVER lives in this store: an upsert transports it once into
// the OS keychain; everything this store holds is the REDACTED summary
// (has_key boolean, no secret field exists on the wire type).

import type {
  ProviderInstanceSummaryDto,
  ProviderInstanceUpsertRequestDto,
  ProviderInstanceValidateRequestDto,
  ProviderInstanceValidateResponseDto,
} from '../client/types'
import { clientInstance } from '../client/instance'

export type ProviderConnection = 'ready' | 'not_configured'

export interface ProvidersState {
  instances: ProviderInstanceSummaryDto[]
  revision: number
  loaded: boolean
  /** Honest transport failure (no Tauri bridge, backend error) — rendered, never hidden. */
  error: string | null
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

class ProviderInstancesStore {
  private state: ProvidersState = {
    instances: [],
    revision: 0,
    loaded: false,
    error: null,
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

  /** Redacted list from the live contract (has_key only, never the key). */
  async reload(): Promise<void> {
    try {
      const res = await clientInstance.getClient().providerInstanceList()
      this.set({
        instances: res.instances,
        revision: res.revision,
        loaded: true,
        error: null,
      })
    } catch (e) {
      this.set({ loaded: true, error: errorText(e) })
    }
  }

  /** Create/edit; `secret` (when present) goes once into the OS keychain. */
  async upsert(req: ProviderInstanceUpsertRequestDto): Promise<void> {
    await clientInstance.getClient().providerInstanceUpsert(req)
    await this.reload()
  }

  /** Remove the instance AND its keychain key (backend deletes key first). */
  async remove(instanceId: string): Promise<void> {
    await clientInstance
      .getClient()
      .providerInstanceDelete({ instance_id: instanceId })
    await this.reload()
  }

  /** Typed form validation — no network, no keychain access. */
  validate(
    req: ProviderInstanceValidateRequestDto,
  ): Promise<ProviderInstanceValidateResponseDto> {
    return clientInstance.getClient().providerInstanceValidate(req)
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
