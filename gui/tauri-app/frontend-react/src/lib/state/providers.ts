// Providers store (React lane) — honest mock mirroring the frozen Svelte
// store semantics: purely client-side provider templates + statuses, never
// a backend. API keys are NEVER stored here — only a hasKey flag; the real
// credential lives in the OS keychain. The production connection flow is
// Phase E (LLM lane); this store powers the honest Providers UI.

export type ProviderId = 'zai' | 'openai' | 'anthropic' | 'ollama' | 'custom'
export type ProviderStatus = 'connected' | 'not_configured' | 'offline' | 'testing'

export interface ProviderData {
  id: ProviderId
  nameKey: string
  descKey: string
  status: ProviderStatus
  models: string[]
  model: string
  baseUrl: string
  /** Flag only — a real key lives in the system keychain and is never displayed. */
  hasKey: boolean
  privacy: 'cloud' | 'local'
}

const INITIAL: ProviderData[] = [
  { id: 'zai', nameKey: 'providers.example.zai', descKey: 'provider.zai.desc', status: 'connected', models: ['glm-4.6', 'glm-4.5-air'], model: 'glm-4.6', baseUrl: 'https://api.z.ai/api/anthropic', hasKey: true, privacy: 'cloud' },
  { id: 'openai', nameKey: 'providers.example.openai', descKey: 'provider.openai.desc', status: 'not_configured', models: ['gpt-4o', 'gpt-4o-mini'], model: 'gpt-4o-mini', baseUrl: 'https://api.openai.com/v1', hasKey: false, privacy: 'cloud' },
  { id: 'anthropic', nameKey: 'providers.example.anthropic', descKey: 'provider.anthropic.desc', status: 'not_configured', models: ['claude-sonnet-4', 'claude-haiku-4'], model: 'claude-sonnet-4', baseUrl: 'https://api.anthropic.com', hasKey: false, privacy: 'cloud' },
  { id: 'ollama', nameKey: 'providers.example.ollama', descKey: 'provider.ollama.desc', status: 'offline', models: ['llama3.1:8b', 'qwen2.5:7b'], model: 'llama3.1:8b', baseUrl: 'http://localhost:11434', hasKey: false, privacy: 'local' },
  { id: 'custom', nameKey: 'providers.example.custom', descKey: 'provider.custom.desc', status: 'not_configured', models: ['custom-model'], model: 'custom-model', baseUrl: 'https://your-endpoint.example/v1', hasKey: false, privacy: 'cloud' },
]

const TEST_LATENCY_MS = 800

class ProviderStore {
  list: ProviderData[] = structuredClone(INITIAL)
  private listeners = new Set<() => void>()

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  getSnapshot(): ProviderData[] {
    return this.list
  }

  private notify(): void {
    this.listeners.forEach((l) => l())
  }

  byId(id: ProviderId): ProviderData {
    const p = this.list.find((x) => x.id === id)
    if (!p) throw new Error(`unknown provider: ${id}`)
    return p
  }

  setStatus(id: ProviderId, status: ProviderStatus): void {
    this.byId(id).status = status
    this.notify()
  }

  setModel(id: ProviderId, model: string): void {
    this.byId(id).model = model
    this.notify()
  }

  setBaseUrl(id: ProviderId, baseUrl: string): void {
    this.byId(id).baseUrl = baseUrl
    this.notify()
  }

  setHasKey(id: ProviderId, hasKey: boolean): void {
    this.byId(id).hasKey = hasKey
    this.notify()
  }

  test(id: ProviderId): void {
    const p = this.byId(id)
    p.status = 'testing'
    this.notify()
    window.setTimeout(() => {
      if (p.privacy === 'local') p.status = 'offline'
      else p.status = p.hasKey ? 'connected' : 'not_configured'
      this.notify()
    }, TEST_LATENCY_MS)
  }
}

export const providers = new ProviderStore()
