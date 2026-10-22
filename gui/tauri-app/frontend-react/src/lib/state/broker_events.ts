// Phase 2b React sync (docs/development/MCP_SERVER_PHASE2.md, «React sync»):
// the SINGLE bootstrap listener for the GUI-session broker's external-change
// event. The Rust host forwards every broker `revision_changed` frame to the
// webview as `emit("broker-changed", …)` (gui/tauri-app/src-tauri/src/
// broker_host.rs `BROKER_CHANGED_EVENT`); the payload is the protocol's
// RevisionChangedPayload — {project_id, revision, session_epoch, kind}
// (crates/rimloc-broker/src/protocol.rs:245). On a matching open project the
// handler marks the store and re-adopts the durable snapshot via
// projectStore.adoptExternal() — the project's own fetch/refetch mechanic
// (lib/state/project.ts), the same one existing-import and chat-batch use.
//
// DRAFTS GUARANTEE (Lane B invariant: unsaved drafts are untouchable):
// adoptExternal() NEVER writes draftsAll. The cross-locale draft truth is a
// module-level map in project.ts, mutated only by the user's own edit paths
// (setDraft/clearDraft/commit) and the restore pipeline; adoptExternal only
// re-derives the ACTIVE-LOCALE VIEW from it
// (`drafts: draftsViewOf(draftsAll, locale)`, project.ts, adoptExternal).
// This is regression-locked by project.drafts.test.ts scenario 5
// («adoptExternal keeps manual drafts») and re-checked for the broker path
// by broker_events.test.ts below.
//
// Transport honesty (mirrors lib/client/instance.ts, never a silent mock):
// this lane has no `@tauri-apps/api` dependency by design — the app injects
// the Tauri global bundle (`withGlobalTauri: true` in src-tauri/
// tauri.conf.json), so the bridge is `window.__TAURI__.event.listen`.
// An absent bridge is a configuration fact, not a fallback: listenBrokerChanges()
// REJECTS, main.tsx warns and the UI keeps living without broker sync.
import { getState, projectStore, type ExternalChangeMark } from './project'

/** Wire name of the host event (broker_host.rs BROKER_CHANGED_EVENT). */
export const BROKER_CHANGED_EVENT = 'broker-changed'

/** The injected Tauri global bundle, narrowed to the event surface we need. */
interface TauriEventBridge {
  __TAURI__?: {
    event?: {
      listen?: (
        event: string,
        handler: (ev: { payload?: unknown }) => void,
      ) => Promise<() => void>
    }
  }
}

/** Strict shape check of the wire payload — fail-closed: a malformed frame
 *  is warned about and dropped, never half-applied to the store. */
function parsePayload(raw: unknown): ExternalChangeMark | null {
  if (typeof raw !== 'object' || raw === null) return null
  const p = raw as Record<string, unknown>
  if (typeof p.project_id !== 'string' || p.project_id === '') return null
  if (typeof p.revision !== 'number' || !Number.isFinite(p.revision) || p.revision < 0) return null
  if (typeof p.session_epoch !== 'number' || !Number.isFinite(p.session_epoch)) return null
  if (typeof p.kind !== 'string') return null
  return {
    project_id: p.project_id,
    revision: p.revision,
    session_epoch: p.session_epoch,
    kind: p.kind,
  }
}

async function onBrokerChanged(payload: unknown): Promise<void> {
  const change = parsePayload(payload)
  if (!change) {
    console.warn('[broker-events] malformed broker-changed payload ignored:', payload)
    return
  }
  const snap = getState().snapshot
  // Чужой (или никакой) открытый проект — наш стор не инвалидируем.
  if (!snap || snap.project_id !== change.project_id) return
  // Эхо собственных мутаций GUI (брокер эмитит после КАЖДОЙ успешной
  // мутации, включая наши) и уже принятые события: локальная ревизия
  // актуальна — бейджа и лишнего refetch не будет.
  if (change.revision <= snap.revision && change.session_epoch === snap.session_epoch) return
  // 1) честная отметка для UI (бейдж в шапке воркспейса);
  projectStore.markExternalChange(change)
  // 2) invalidate + refetch по паттерну проекта: adoptExternal усыновляет
  //    свежий снапшот и гасит бейдж. Провал fetch бейдж ОСТАВЛЯЕТ
  //    (fail-closed: пользователь видит, что внешнее изменение было).
  await projectStore.adoptExternal()
}

/** Subscribe once at bootstrap (main.tsx). Resolves with the unlisten
 *  function once the host listener is registered; rejects honestly when
 *  the Tauri event bridge is absent (browser dev, broken webview). */
export async function listenBrokerChanges(): Promise<() => void> {
  if (typeof window === 'undefined') {
    throw new Error('RimLoc React UI: no window — broker sync unavailable outside the webview')
  }
  const listen = (window as unknown as TauriEventBridge).__TAURI__?.event?.listen
  if (typeof listen !== 'function') {
    throw new Error(
      'RimLoc React UI: no Tauri event bridge (window.__TAURI__.event.listen missing) — broker sync unavailable in this launch',
    )
  }
  return listen(BROKER_CHANGED_EVENT, (ev) => {
    // Обработчик события не может уронить UI: асинхронные сбои — warn.
    void onBrokerChanged(ev?.payload).catch((e: unknown) => {
      console.warn('[broker-events] broker-changed handling failed — UI keeps living:', e)
    })
  })
}
