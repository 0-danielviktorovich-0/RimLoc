// React-lane client instance: honest mode resolution, mirroring the frozen
// Svelte instance store semantics — tauri bridge → live; anything else is a
// configuration error, never a silent mock (lead 033/010).
import { createRimLocClient, type RimLocClient } from './client'

interface TauriGlobal {
  __TAURI_INTERNALS__?: { invoke?: unknown }
  __TAURI__?: { core?: { invoke?: unknown } }
}

function tauriInvoke(): unknown | null {
  if (typeof window === 'undefined') return null
  const g = window as unknown as TauriGlobal
  return g.__TAURI_INTERNALS__?.invoke ?? g.__TAURI__?.core?.invoke ?? null
}

let cached: RimLocClient | null = null

export const clientInstance = {
  getClient(): RimLocClient {
    if (cached) return cached
    const invoke = tauriInvoke()
    if (!invoke) {
      throw new Error(
        'RimLoc React UI: no Tauri bridge in this launch. The mock is never a silent default — run the desktop app (React lane) against the real backend.',
      )
    }
    cached = createRimLocClient({ mode: 'tauri', invoke })
    return cached
  },
}
