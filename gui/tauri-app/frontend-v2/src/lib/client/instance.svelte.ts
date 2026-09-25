// Client instance (W-built wave): the ONE place the transport mode is
// resolved for the whole app.
//
// Resolution order (lead: "prod-сборка с mock = ошибка конфигурации"):
//   1. Tauri invoke bridge present  → mode 'tauri' (the real client).
//   2. devMode enabled              → mode 'mock'  (demo/dev, EXPLICIT).
//   3. Neither                      → mode 'none': the app shows an honest
//      configuration error instead of silently shipping the mock.
//
// The Demo-data badge is unrelated to this gate and stays unconditional.
import { devMode } from '../stores/devmode.svelte';
import { createRimLocClient, type RimLocClient } from './client';
import type { TransportMode } from './transport';

export type ResolvedClientMode = TransportMode | 'none';

export class ClientConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ClientConfigError';
  }
}

interface TauriGlobal {
  __TAURI_INTERNALS__?: { invoke?: unknown };
  __TAURI__?: { core?: { invoke?: unknown } };
}

function tauriInvoke(): unknown | null {
  if (typeof window === 'undefined') return null;
  const g = window as unknown as TauriGlobal;
  return g.__TAURI_INTERNALS__?.invoke ?? g.__TAURI__?.core?.invoke ?? null;
}

class ClientInstanceStore {
  /** Resolved mode after the first boot attempt; null before it. */
  mode = $state<ResolvedClientMode | null>(null);
  /** Honest configuration failure message (mode 'none'). */
  configError = $state<string | null>(null);
  private client: RimLocClient | null = null;

  /**
   * Resolve the mode without throwing — for boot gating and UI switches.
   * 'none' means: no Tauri bridge and no explicit dev/demo opt-in.
   */
  resolveMode(): ResolvedClientMode {
    if (this.mode !== null) return this.mode;
    if (tauriInvoke()) {
      this.mode = 'tauri';
    } else if (devMode.enabled) {
      this.mode = 'mock';
    } else {
      this.mode = 'none';
      this.configError =
        'RimLoc GUI: no Tauri bridge in this launch and no dev/demo opt-in. ' +
        'The mock backend is never a silent production default — start the desktop app, ' +
        'or enable the dev mode (?dev=1 / persisted opt-in) for the demo build.';
    }
    return this.mode;
  }

  /**
   * The app client. Throws ClientConfigError in mode 'none' — callers above
   * the boot gate never reach this; the error is the honest contract failure.
   */
  getClient(): RimLocClient {
    if (this.client) return this.client;
    const mode = this.resolveMode();
    if (mode === 'none') {
      throw new ClientConfigError(this.configError ?? 'client mode unresolved');
    }
    if (mode === 'tauri') {
      this.client = createRimLocClient({ mode: 'tauri', invoke: tauriInvoke() });
    } else {
      this.client = createRimLocClient({ mode: 'mock' });
    }
    return this.client;
  }
}

export const clientInstance = new ClientInstanceStore();
