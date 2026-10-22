// Phase 2b React sync — broker-changed listener against the REAL project
// store with a mocked contract client (same lane as project.drafts.test.ts).
// Scenarios:
//   1. higher revision → badge marked, snapshot re-adopted, DRAFTS KEPT
//      (the Lane B invariant: unsaved user input is untouchable);
//   2. self-echo / stale revision (<= local) → no badge, no refetch;
//   3. foreign project_id → ignored;
//   4. session_epoch change → treated as external (refetch runs);
//   5. malformed payload → honest console.warn, store untouched;
//   6. missing Tauri event bridge → listenBrokerChanges() REJECTS (never a
//      silent mock — the instance.ts convention).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type {
  DraftEntryDto,
  DraftsRestoreReportDto,
  ProjectSnapshotDto,
  SourceEntryDto,
} from '../client/types'

vi.mock('../client/instance', () => {
  type Snap = import('../client/types').ProjectSnapshotDto
  type Ack = import('../client/types').DraftsSaveAckDto
  type DraftMap = Record<string, import('../client/types').DraftEntryDto>
  const hub = {
    calls: [] as { method: string; args: unknown[] }[],
    snapshots: new Map<string, Snap>(),
    restoreReports: new Map<string, import('../client/types').DraftsRestoreReportDto>(),
    savedHistory: [] as { pid: string; entries: DraftMap }[],
  }
  const instance = Object.assign(hub, {
    getClient: () => ({
      handshake: async () => ({
        ui_contract_version: 1,
        capabilities: { contract_version: 1, supported: [], unsupported: [] },
      }),
      openProject: async (pid: string) => {
        hub.calls.push({ method: 'openProject', args: [pid] })
        const snap = hub.snapshots.get(pid)
        if (!snap) throw new Error('project_not_found')
        return snap
      },
      createProject: async (modRoot: string) => {
        hub.calls.push({ method: 'project_create', args: [modRoot] })
        const snap = hub.snapshots.get(modRoot)
        if (!snap) throw new Error('project_not_found')
        return snap
      },
      listProjects: async () => [],
      snapshot: async (pid: string) => {
        hub.calls.push({ method: 'snapshot', args: [pid] })
        const snap = hub.snapshots.get(pid)
        if (!snap) throw new Error('project_not_found')
        return snap
      },
      applyIntents: async (req: {
        projectId: string
        intents: {
          entry: { kind: string; key: string; def_type?: string }
          locale: string
          text?: string
        }[]
      }) => {
        hub.calls.push({ method: 'applyIntents', args: [req] })
        const snap = hub.snapshots.get(req.projectId)
        if (!snap) throw new Error('project_not_found')
        snap.revision++
        for (const intent of req.intents) {
          snap.project.translations.push({
            source_id: intent.entry,
            locale: intent.locale,
            text: intent.text ?? null,
            completeness: 'translated',
            review: '',
            validation: 'unknown',
            lifecycle: '',
            origin: 'human',
          })
        }
        return {
          job_id: 'j',
          revision: snap.revision,
          applied: req.intents.length,
          skipped: [],
          cancelled: false,
        }
      },
      validateProject: async () => ({
        job_id: 'j',
        status: 'succeeded' as const,
        findings: [],
        error_count: 0,
        warning_count: 0,
        info_count: 0,
      }),
      saveDrafts: async (pid: string, _epoch: number, entries: DraftMap): Promise<Ack> => {
        hub.calls.push({ method: 'saveDrafts', args: [pid, entries] })
        hub.savedHistory.push({ pid, entries })
        return { saved: Object.keys(entries).length, project_revision: 1, updated_at: 0 }
      },
      restoreDrafts: async (pid: string): Promise<DraftsRestoreReportDto> => {
        return hub.restoreReports.get(pid) ?? { status: 'none', restored_count: 0 }
      },
      clearDrafts: async (pid: string, epoch: number, key?: string) => {
        hub.calls.push({ method: 'clearDrafts', args: [pid, epoch, key] })
        return { saved: 0, project_revision: 1, updated_at: 0 }
      },
    }),
  })
  return { clientInstance: instance }
})

type Hub = {
  calls: { method: string; args: unknown[] }[]
  snapshots: Map<string, ProjectSnapshotDto>
  restoreReports: Map<string, DraftsRestoreReportDto>
  savedHistory: { pid: string; entries: Record<string, DraftEntryDto> }[]
}

type EventHandler = (ev: { payload?: unknown }) => void

async function freshLane(): Promise<{
  hub: Hub
  project: typeof import('./project')
  broker: typeof import('./broker_events')
}> {
  vi.resetModules()
  const hub = (await import('../client/instance')).clientInstance as unknown as Hub
  const project = await import('./project')
  const broker = await import('./broker_events')
  return { hub, project, broker }
}

/** Register the fake injected Tauri global and capture the handlers the
 *  bootstrap subscribes — the test drives the REAL listenBrokerChanges()
 *  path (bridge resolution → registration → handler), never the internals. */
function installBridge(): Map<string, EventHandler> {
  const handlers = new Map<string, EventHandler>()
  ;(window as unknown as Record<string, unknown>).__TAURI__ = {
    event: {
      listen: (event: string, handler: EventHandler) => {
        handlers.set(event, handler)
        return Promise.resolve(() => handlers.delete(event))
      },
    },
  }
  return handlers
}

function snapshotCalls(hub: Hub): number {
  return hub.calls.filter((c) => c.method === 'snapshot').length
}

function savePayloads(hub: Hub, pid: string): Record<string, DraftEntryDto>[] {
  return hub.savedHistory.filter((s) => s.pid === pid).map((s) => s.entries)
}

function sourceEntry(kind: string, key: string, defType?: string, text = 'source'): SourceEntryDto {
  return {
    id: defType ? { kind, key, def_type: defType } : { kind, key },
    text,
    source_locale: 'en',
  }
}

function makeSnap(pid: string, entries: SourceEntryDto[], epoch = 1): ProjectSnapshotDto {
  return {
    project_id: pid,
    revision: 1,
    session_epoch: epoch,
    project: {
      context: { active_dlc: [], active_mods: [], load_order: [], view: '' },
      entries,
      translations: [],
    },
  }
}

const changed = (payload: unknown) => ({ payload })

beforeEach(() => {
  vi.useRealTimers()
  window.localStorage.clear()
})

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI__
})

describe('broker_events — Phase 2b React sync', () => {
  it('higher revision: badge + refetch, translation updates, DRAFTS KEPT', async () => {
    const { hub, project, broker } = await freshLane()
    const handlers = installBridge()
    const unlisten = await broker.listenBrokerChanges()
    expect(handlers.has(broker.BROKER_CHANGED_EVENT)).toBe(true)

    const snap = makeSnap('p-broker', [sourceEntry('keyed', 'Settings')])
    hub.snapshots.set('p-broker', snap)
    expect(await project.projectStore.open('p-broker')).toBe(true)

    const uid = 'keyed·Settings'
    project.projectStore.setDraft(uid, 'ручной черновик')
    await vi.waitFor(() => expect(savePayloads(hub, 'p-broker')).toHaveLength(1), {
      timeout: 4000,
      interval: 25,
    })

    // External mutation via the broker: the durable project moves on and
    // the refetch returns a NEW wire object — the store's adopted snapshot
    // object stays at the old revision (that's exactly what the guard
    // compares against; mutating it in place would fake a stale event).
    const external: ProjectSnapshotDto = {
      ...snap,
      revision: 2,
      project: {
        ...snap.project,
        translations: [
          ...snap.project.translations,
          {
            source_id: { kind: 'keyed', key: 'Settings' },
            locale: 'Russian',
            text: 'подтверждённый перевод',
            completeness: 'translated',
            review: '',
            validation: 'unknown',
            lifecycle: '',
            origin: 'llm',
          },
        ],
      },
    }
    hub.snapshots.set('p-broker', external)
    handlers.get(broker.BROKER_CHANGED_EVENT)?.(
      changed({ project_id: 'p-broker', revision: 2, session_epoch: 1, kind: 'apply' }),
    )
    // Wait on the honest signals: the refetch ran AND the badge cleared
    // (which only happens after a successful adoption).
    await vi.waitFor(() => expect(snapshotCalls(hub)).toBeGreaterThan(0), {
      timeout: 4000,
      interval: 25,
    })
    await vi.waitFor(() => expect(project.getState().externalChange).toBeNull(), {
      timeout: 4000,
      interval: 25,
    })

    // The refetch adopted the external translation…
    expect(project.getState().entries.find((e) => e.uid === uid)?.target).toBe(
      'подтверждённый перевод',
    )
    // …the badge cleared after the SUCCESSFUL refetch…
    expect(project.getState().externalChange).toBeNull()
    // …and the unsaved draft SURVIVED (durable slice untouched: still
    // exactly the one flush, the adoptExternal path never writes draftsAll).
    expect(project.getState().drafts[uid]).toBe('ручной черновик')
    expect(savePayloads(hub, 'p-broker')).toHaveLength(1)

    expect(typeof unlisten).toBe('function')
  })

  it('self-echo and stale revisions (<= local) raise nothing', async () => {
    const { hub, project, broker } = await freshLane()
    const handlers = installBridge()
    await broker.listenBrokerChanges()
    const snap = makeSnap('p-echo', [sourceEntry('keyed', 'Settings')])
    hub.snapshots.set('p-echo', snap)
    await project.projectStore.open('p-echo')
    const snapshotsBefore = snapshotCalls(hub)

    // The GUI's own mutations also traverse the broker → the event fires
    // with the revision we already adopted; an old event fires lower.
    for (const revision of [1, 0]) {
      handlers.get(broker.BROKER_CHANGED_EVENT)?.(
        changed({ project_id: 'p-echo', revision, session_epoch: 1, kind: 'apply' }),
      )
    }
    await vi.waitFor(() => new Promise((r) => setTimeout(r, 20)))
    expect(snapshotCalls(hub)).toBe(snapshotsBefore)
    expect(project.getState().externalChange).toBeNull()
  })

  it('a foreign project_id never touches this store', async () => {
    const { hub, project, broker } = await freshLane()
    const handlers = installBridge()
    await broker.listenBrokerChanges()
    hub.snapshots.set('p-mine', makeSnap('p-mine', [sourceEntry('keyed', 'Settings')]))
    await project.projectStore.open('p-mine')
    const snapshotsBefore = snapshotCalls(hub)

    handlers.get(broker.BROKER_CHANGED_EVENT)?.(
      changed({ project_id: 'p-other', revision: 9, session_epoch: 1, kind: 'apply' }),
    )
    await vi.waitFor(() => new Promise((r) => setTimeout(r, 20)))
    expect(snapshotCalls(hub)).toBe(snapshotsBefore)
    expect(project.getState().externalChange).toBeNull()
  })

  it('a session_epoch change counts as external even at the same revision', async () => {
    const { hub, project, broker } = await freshLane()
    const handlers = installBridge()
    await broker.listenBrokerChanges()
    const snap = makeSnap('p-epoch', [sourceEntry('keyed', 'Settings')])
    hub.snapshots.set('p-epoch', snap)
    await project.projectStore.open('p-epoch')
    const snapshotsBefore = snapshotCalls(hub)

    handlers.get(broker.BROKER_CHANGED_EVENT)?.(
      changed({ project_id: 'p-epoch', revision: 1, session_epoch: 2, kind: 'project_open' }),
    )
    await vi.waitFor(() => expect(snapshotCalls(hub)).toBe(snapshotsBefore + 1), {
      timeout: 4000,
      interval: 25,
    })
    // Refetch succeeded → badge cleared again.
    await vi.waitFor(() => expect(project.getState().externalChange).toBeNull(), {
      timeout: 4000,
      interval: 25,
    })
  })

  it('malformed payloads: honest warn, store untouched', async () => {
    const { hub, project, broker } = await freshLane()
    const handlers = installBridge()
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    await broker.listenBrokerChanges()
    hub.snapshots.set('p-bad', makeSnap('p-bad', [sourceEntry('keyed', 'Settings')]))
    await project.projectStore.open('p-bad')
    const snapshotsBefore = snapshotCalls(hub)

    for (const payload of [
      null,
      'revision_changed',
      {},
      { project_id: 'p-bad', revision: '2', session_epoch: 1, kind: 'apply' },
      { project_id: '', revision: 2, session_epoch: 1, kind: 'apply' },
      { project_id: 'p-bad', revision: 2, session_epoch: 1 },
    ]) {
      handlers.get(broker.BROKER_CHANGED_EVENT)?.(changed(payload))
    }
    await vi.waitFor(() => new Promise((r) => setTimeout(r, 20)))
    expect(warn).toHaveBeenCalled()
    expect(snapshotCalls(hub)).toBe(snapshotsBefore)
    expect(project.getState().externalChange).toBeNull()
    warn.mockRestore()
  })

  it('missing Tauri event bridge: listenBrokerChanges rejects (no silent mock)', async () => {
    const { broker } = await freshLane()
    delete (window as unknown as Record<string, unknown>).__TAURI__
    await expect(broker.listenBrokerChanges()).rejects.toThrow(/no Tauri event bridge/)
  })
})
