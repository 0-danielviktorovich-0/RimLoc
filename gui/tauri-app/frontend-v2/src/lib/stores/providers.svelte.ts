// Mock AI provider registry (mandate §6, spec §12). Shared between the
// Provider Manager screen and the Settings → AI summary so both show the same
// statuses. Purely local state: test/configure mutate this store, never a
// backend. API keys are NEVER stored here in plaintext — only a hasKey flag;
// the real credential would live in the OS keychain.

export type ProviderId = 'zai' | 'openai' | 'anthropic' | 'ollama' | 'custom';
export type ProviderStatus = 'connected' | 'not_configured' | 'offline' | 'testing';

export interface ProviderData {
  id: ProviderId;
  /** i18n key for the display name (providers.example.*). */
  nameKey: string;
  /** i18n key for the one-line description (provider.<id>.desc). */
  descKey: string;
  status: ProviderStatus;
  models: string[];
  model: string;
  baseUrl: string;
  /** Flag only — a real key lives in the system keychain and is never displayed. */
  hasKey: boolean;
  privacy: 'cloud' | 'local';
}

const INITIAL: ProviderData[] = [
  {
    id: 'zai',
    nameKey: 'providers.example.zai',
    descKey: 'provider.zai.desc',
    status: 'connected',
    models: ['glm-4.6', 'glm-4.5-air'],
    model: 'glm-4.6',
    baseUrl: 'https://api.z.ai/api/anthropic',
    hasKey: true,
    privacy: 'cloud'
  },
  {
    id: 'openai',
    nameKey: 'providers.example.openai',
    descKey: 'provider.openai.desc',
    status: 'not_configured',
    models: ['gpt-4o', 'gpt-4o-mini'],
    model: 'gpt-4o-mini',
    baseUrl: 'https://api.openai.com/v1',
    hasKey: false,
    privacy: 'cloud'
  },
  {
    id: 'anthropic',
    nameKey: 'providers.example.anthropic',
    descKey: 'provider.anthropic.desc',
    status: 'not_configured',
    models: ['claude-sonnet-4', 'claude-haiku-4'],
    model: 'claude-sonnet-4',
    baseUrl: 'https://api.anthropic.com',
    hasKey: false,
    privacy: 'cloud'
  },
  {
    id: 'ollama',
    nameKey: 'providers.example.ollama',
    descKey: 'provider.ollama.desc',
    status: 'offline',
    models: ['llama3.1:8b', 'qwen2.5:7b'],
    model: 'llama3.1:8b',
    baseUrl: 'http://localhost:11434',
    hasKey: false,
    privacy: 'local'
  },
  {
    id: 'custom',
    nameKey: 'providers.example.custom',
    descKey: 'provider.custom.desc',
    status: 'not_configured',
    models: ['custom-model'],
    model: 'custom-model',
    baseUrl: 'https://your-endpoint.example/v1',
    hasKey: false,
    privacy: 'cloud'
  }
];

const TEST_LATENCY_MS = 800; // simulated probe so the "testing…" state is visible

class ProviderStore {
  list = $state<ProviderData[]>(structuredClone(INITIAL));

  byId(id: ProviderId): ProviderData {
    const p = this.list.find((x) => x.id === id);
    if (!p) throw new Error(`unknown provider: ${id}`);
    return p;
  }

  counts(): { connected: number; notConfigured: number; offline: number } {
    let connected = 0;
    let notConfigured = 0;
    let offline = 0;
    for (const p of this.list) {
      if (p.status === 'connected') connected++;
      else if (p.status === 'offline') offline++;
      else notConfigured++;
    }
    return { connected, notConfigured, offline };
  }

  setStatus(id: ProviderId, status: ProviderStatus) {
    this.byId(id).status = status;
  }

  setModel(id: ProviderId, model: string) {
    this.byId(id).model = model;
  }

  setBaseUrl(id: ProviderId, baseUrl: string) {
    this.byId(id).baseUrl = baseUrl;
  }

  /** Mock "save key": flips the flag only; no secret ever enters this store. */
  setHasKey(id: ProviderId, hasKey: boolean) {
    this.byId(id).hasKey = hasKey;
  }

  /**
   * Mock connection test: cloud providers with a key come back connected,
   * without a key they return to "not configured"; the local Ollama mock is
   * down, demonstrating the offline state with its recovery hint.
   */
  test(id: ProviderId) {
    const p = this.byId(id);
    p.status = 'testing';
    window.setTimeout(() => {
      if (p.privacy === 'local') p.status = 'offline';
      else p.status = p.hasKey ? 'connected' : 'not_configured';
    }, TEST_LATENCY_MS);
  }
}

export const providers = new ProviderStore();
