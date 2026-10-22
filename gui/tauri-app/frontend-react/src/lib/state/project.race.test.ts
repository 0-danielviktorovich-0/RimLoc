// Review iteration 3 P0-3: the autosave flush persists the FULL map with
// REPLACE semantics — a flush that fires before the restore merge would
// overwrite durable work with an incomplete map. The restore gate must
// defer every flush until adoptRestoredDrafts settles, even when the user
// starts typing immediately after open().
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProjectSnapshotDto, SourceEntryDto } from '../client/types'

vi.mock('../client/instance', () => {
  type Snap = import('../client/types').ProjectSnapshotDto
  const hub = {
    calls: [] as { method: string; args: unknown[] }[],
    snapshots: new Map<string, Snap>(),
    /** Resolved when the test lets the delayed restore finish. */
    releaseRestore: (() => {}) as () => void,
  }
  const instance = Object.assign(hub, {
    getClient: () => ({
      openProject: async (pid: string) => {
        hub.calls.push({ method: 'openProject', args: [pid] })
        const snap = hub.snapshots.get(pid)
        if (!snap) throw new Error('project_not_found')
        return snap
      },
      createProject: async () => {
        throw new Error('not used')
      },
      listProjects: async () => [],
      snapshot: async (pid: string) => {
        const snap = hub.snapshots.get(pid)
        if (!snap) throw new Error('project_not_found')
        return snap
      },
      applyIntents: async () => {
        throw new Error('not used')
      },
      validateProject: async () => ({
        job_id: 'j',
        status: 'succeeded' as const,
        findings: [],
        error_count: 0,
        warning_count: 0,
        info_count: 0,
      }),
      saveDrafts: async (
        pid: string,
        epoch: number,
        entries: Record<string, import('../client/types').DraftEntryDto>,
      ) => {
        hub.calls.push({ method: 'saveDrafts', args: [pid, epoch, entries] })
        return { saved: Object.keys(entries).length, project_revision: 1, updated_at: 0 }
      },
      restoreDrafts: async (): Promise<import('../client/types').DraftsRestoreReportDto> => {
        hub.calls.push({ method: 'restoreDrafts', args: [] })
        // The durable store holds OLD work from the previous session; the
        // merge lands only when the test releases the gate.
        await new Promise<void>((r) => {
          const w = window as unknown as { __release?: () => void }
          w.__release = r
        })
        return {
          status: 'restored',
          drafts: {
            'ru::keyed·Old': { text: 'старый черновик', updated_at: 1 },
          },
          restored_count: 1,
        }
      },
    }),
  })
  return { clientInstance: instance }
})

function sourceEntry(kind: string, key: string, defType?: string): import('../client/types').SourceEntryDto {
  return {
    id: defType ? { kind, key, def_type: defType } : { kind, key },
    text: 'source',
    source_locale: 'en',
  }
}

function makeSnap(pid: string, entries: SourceEntryDto[]): ProjectSnapshotDto {
  return {
    project_id: pid,
    revision: 1,
    session_epoch: 1,
    project: {
      context: { active_dlc: [], active_mods: [], load_order: [], view: '' },
      entries,
      translations: [],
    },
  }
}

beforeEach(() => {
  vi.useRealTimers()
  window.localStorage.clear()
  delete (window as unknown as { __release?: () => void }).__release
})

afterEach(() => {
  vi.useRealTimers()
})

describe('restore gate (review P0-3) — flush cannot race the restore merge', () => {
  it('typing right after open defers the flush until the durable store is merged', { timeout: 15000 }, async () => {
    vi.resetModules()
    const hub = (await import('../client/instance')).clientInstance as unknown as {
      calls: { method: string; args: unknown[] }[]
      snapshots: Map<string, ProjectSnapshotDto>
    }
    hub.calls.length = 0
    hub.snapshots.set('p-race', makeSnap('p-race', [sourceEntry('keyed', 'Old'), sourceEntry('keyed', 'New')]))
    const project = await import('./project')

    await project.projectStore.open('p-race')
    // The user types IMMEDIATELY — adoptRestoredDrafts is still awaiting
    // the delayed store read.
    project.projectStore.setDraft('keyed·New', 'новая правка поверх рестарта')
    // The debounce fires at 1.5s — but the gate must hold the flush.
    await new Promise((r) => setTimeout(r, 2200))
    const savesBeforeRelease = hub.calls.filter((c) => c.method === 'saveDrafts')
    console.log(`[race] saves_before_release=${savesBeforeRelease.length}`)
    expect(savesBeforeRelease).toHaveLength(0)

    // Release the restore: the merge lands, THEN the pending flush runs and
    // its replace-semantics payload contains OLD and NEW together.
    ;(window as unknown as { __release?: () => void }).__release?.()
    await vi.waitFor(() => {
      const saves = hub.calls.filter((c) => c.method === 'saveDrafts')
      expect(saves.length).toBeGreaterThan(0)
      const payload = saves[0].args[2] as Record<string, { text: string }>
      expect(payload['ru::keyed·Old']?.text).toBe('старый черновик')
      expect(payload['ru::keyed·New']?.text).toBe('новая правка поверх рестарта')
    }, { timeout: 4000, interval: 25 })
  })
})
