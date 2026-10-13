// Review iteration 4 P0-A: the restore gate must be bound to ITS OWN
// open's token. A stale restore (project A) settling late must NEVER open
// project B's gate while B's restore is still pending.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProjectSnapshotDto } from '../client/types'

vi.mock('../client/instance', () => {
  type Snap = import('../client/types').ProjectSnapshotDto
  const hub = {
    calls: [] as { method: string; args: unknown[] }[],
    snapshots: new Map<string, Snap>(),
    release: new Map<string, () => void>(),
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
      restoreDrafts: async (pid: string): Promise<import('../client/types').DraftsRestoreReportDto> => {
        hub.calls.push({ method: 'restoreDrafts', args: [pid] })
        // Each project's restore hangs until ITS release is called — the
        // test controls the interleaving precisely.
        await new Promise<void>((r) => hub.release.set(pid, r))
        return {
          status: 'restored',
          drafts: { [`${pid === 'p-a' ? 'ru' : 'ru'}::keyed·Old-${pid}`]: { text: `old-${pid}`, updated_at: 1 } },
          restored_count: 1,
        }
      },
      clearDrafts: async () => {
        return { saved: 0, project_revision: 1, updated_at: 0 }
      },
    }),
  })
  return { clientInstance: instance }
})

type Hub = {
  calls: { method: string; args: unknown[] }[]
  snapshots: Map<string, ProjectSnapshotDto>
  release: Map<string, () => void>
}

function makeSnap(pid: string): ProjectSnapshotDto {
  return {
    project_id: pid,
    revision: 1,
    session_epoch: 1,
    project: {
      context: { active_dlc: [], active_mods: [], load_order: [], view: '' },
      entries: [{ id: { kind: 'keyed', key: `Old-${pid}` }, text: 's', source_locale: 'en' }],
      translations: [],
    },
  }
}

beforeEach(() => {
  window.localStorage.clear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('restore gate token binding (review iteration 4 P0-A)', () => {
  it('a stale restore of project A never releases project B flushes', { timeout: 15000 }, async () => {
    vi.resetModules()
    const hub = (await import('../client/instance')).clientInstance as unknown as Hub
    hub.calls.length = 0
    hub.snapshots.set('p-a', makeSnap('p-a'))
    hub.snapshots.set('p-b', makeSnap('p-b'))
    const project = await import('./project')

    // Open A (restore pending), then open B (restore pending) — B is live.
    await project.projectStore.open('p-a')
    await project.projectStore.open('p-b')
    const live = project.getState()
    expect(live.snapshot?.project_id).toBe('p-b')

    // Type into B while BOTH restores are pending: no flush may fire.
    project.projectStore.setDraft('keyed·Old-p-b', 'правка в B')
    await new Promise((r) => setTimeout(r, 2200))
    expect(hub.calls.filter((c) => c.method === 'saveDrafts')).toHaveLength(0)

    // The STALE restore of A settles: B's gate must stay CLOSED.
    hub.release.get('p-a')?.()
    await new Promise((r) => setTimeout(r, 2200))
    const savesAfterStale = hub.calls.filter((c) => c.method === 'saveDrafts')
    console.log(`[token] saves_after_stale_A=${savesAfterStale.length}`)
    expect(savesAfterStale).toHaveLength(0)

    // B's own restore settles: NOW B's pending flush runs, against the
    // merged store (merge semantics keep unknown keys — the payload holds
    // B's edit; A's keys live in A's own store file, not this payload).
    hub.release.get('p-b')?.()
    await vi.waitFor(() => {
      const saves = hub.calls.filter((c) => c.method === 'saveDrafts' && c.args[0] === 'p-b')
      expect(saves.length).toBeGreaterThan(0)
    }, { timeout: 4000, interval: 25 })
  })
})
