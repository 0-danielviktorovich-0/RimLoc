// Review iteration 5 P0-C: a FAILED clearDrafts must never surface as a
// saved state. Merge semantics keep the deleted key durable until its
// clear is acknowledged; the pending clear holds the sync status on an
// honest error and Retry re-issues exactly that deletion.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProjectSnapshotDto, SourceEntryDto } from '../client/types'

vi.mock('../client/instance', () => {
  type Snap = import('../client/types').ProjectSnapshotDto
  const hub = {
    calls: [] as { method: string; args: unknown[] }[],
    snapshots: new Map<string, Snap>(),
    /** When true, clearDrafts rejects (simulated write failure). */
    failClears: false,
    /** Durable stores mirrored by the mock — PER PROJECT (each project
     *  has its own sidecar file). */
    disks: new Map<string, Map<string, { text: string; updated_at: number }>>(),
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
      applyIntents: async (req: {
        projectId: string
        intents: { entry: { kind: string; key: string; def_type?: string }; locale: string; text?: string }[]
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
      saveDrafts: async (
        pid: string,
        epoch: number,
        entries: Record<string, import('../client/types').DraftEntryDto>,
      ) => {
        hub.calls.push({ method: 'saveDrafts', args: [pid, epoch, entries] })
        // MERGE semantics: upsert into THIS project's mirrored disk.
        const disk = hub.disks.get(pid) ?? new Map()
        hub.disks.set(pid, disk)
        for (const [k, v] of Object.entries(entries)) disk.set(k, v)
        return { saved: Object.keys(entries).length, project_revision: 1, updated_at: 0 }
      },
      restoreDrafts: async (pid: string): Promise<import('../client/types').DraftsRestoreReportDto> => {
        const drafts: Record<string, import('../client/types').DraftEntryDto> = {}
        for (const [k, v] of hub.disks.get(pid) ?? new Map()) drafts[k] = v
        return Object.keys(drafts).length === 0
          ? { status: 'none', restored_count: 0 }
          : { status: 'restored', drafts, restored_count: Object.keys(drafts).length }
      },
      clearDrafts: async (pid: string, epoch: number, key?: string) => {
        hub.calls.push({ method: 'clearDrafts', args: [pid, epoch, key] })
        // The disk is PER-PROJECT (each project has its own sidecar) — a
        // clear addresses only its own project's keys.
        const disk = hub.disks.get(pid) ?? new Map()
        if (hub.failClears) throw new Error('clear failed: EACCES')
        if (key) disk.delete(key)
        else disk.clear()
        return { saved: disk.size, project_revision: 1, updated_at: 0 }
      },
    }),
  })
  return { clientInstance: instance }
})

type Hub = {
  calls: { method: string; args: unknown[] }[]
  snapshots: Map<string, ProjectSnapshotDto>
  failClears: boolean
  /** Per-project durable mirrors — each project owns its sidecar. */
  disks: Map<string, Map<string, { text: string; updated_at: number }>>
}

function sourceEntry(kind: string, key: string): SourceEntryDto {
  return { id: { kind, key }, text: 'source', source_locale: 'en' }
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
  window.localStorage.clear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('acknowledged deletions (review iteration 5 P0-C)', () => {
  it('a failed clear holds the status on error; retry acknowledges exactly that deletion', { timeout: 15000 }, async () => {
    vi.resetModules()
    const hub = (await import('../client/instance')).clientInstance as unknown as Hub
    hub.calls.length = 0
    hub.disks.clear()
    hub.snapshots.set('p-clear', makeSnap('p-clear', [sourceEntry('keyed', 'Target')]))
    const project = await import('./project')

    await project.projectStore.open('p-clear')
    project.projectStore.setDraft('keyed·Target', 'черновик к удалению')
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 4000, interval: 25 },
    )
    expect(hub.disks.get('p-clear')?.get('ru::keyed·Target')?.text).toBe('черновик к удалению')

    // Revert: the clear FAILS — the durable key survives (merge keeps it),
    // and the status must NEVER claim saved.
    hub.failClears = true
    project.projectStore.clearDraft('keyed·Target')
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('error'),
      { timeout: 4000, interval: 25 },
    )
    expect(project.getState().draftsSync.error ?? '').toContain('delete not acknowledged')
    expect(hub.disks.get('p-clear')?.get('ru::keyed·Target')?.text).toBe('черновик к удалению')

    // Retry re-issues exactly the failed deletion; then the status is
    // honestly saved and the disk no longer holds the key.
    hub.failClears = false
    project.projectStore.retryDraftsSync()
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 4000, interval: 25 },
    )
    expect(hub.disks.get('p-clear')?.has('ru::keyed·Target')).toBe(false)
    const clears = hub.calls.filter((c) => c.method === 'clearDrafts' && c.args[2] === 'ru::keyed·Target')
    expect(clears.length).toBe(2) // failed attempt + successful retry
  })

  it('a pending clear of A is never issued through project B', { timeout: 20000 }, async () => {
    vi.resetModules()
    const hub = (await import('../client/instance')).clientInstance as unknown as Hub
    hub.calls.length = 0
    hub.disks.clear()
    hub.snapshots.set('p-a', makeSnap('p-a', [sourceEntry('keyed', 'Shared')]))
    hub.snapshots.set('p-b', makeSnap('p-b', [sourceEntry('keyed', 'Other')]))
    const project = await import('./project')

    // A: draft saved, then the delete FAILS (read-only window).
    await project.projectStore.open('p-a')
    project.projectStore.setDraft('keyed·Shared', 'черновик A')
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 4000, interval: 25 },
    )
    hub.failClears = true
    project.projectStore.clearDraft('keyed·Shared')
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('error'),
      { timeout: 4000, interval: 25 },
    )

    // Switch to B and edit: B's flush must NOT route A's deletion.
    await project.projectStore.open('p-b')
    project.projectStore.setDraft('keyed·Other', 'правка в B')
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 4000, interval: 25 },
    )
    const wrongClears = hub.calls.filter(
      (c) => c.method === 'clearDrafts' && c.args[0] === 'p-b' && String(c.args[2]).includes('Shared'),
    )
    console.log(`[cross] WRONG_CLEARS_ON_B=${wrongClears.length}`)
    expect(wrongClears).toHaveLength(0)

    // Return to A; recover permissions; retry — the deletion applies to A
    // only, and B's durable content is untouched.
    await project.projectStore.open('p-a')
    hub.failClears = false
    project.projectStore.retryDraftsSync()
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 6000, interval: 25 },
    )
    expect(hub.disks.get('p-a')?.has('ru::keyed·Shared')).toBe(false)
    expect(hub.disks.get('p-b')?.get('ru::keyed·Other')?.text).toBe('правка в B')
  })

  it('commit routes its deletion through the same acknowledged path', { timeout: 15000 }, async () => {
    vi.resetModules()
    const hub = (await import('../client/instance')).clientInstance as unknown as Hub
    hub.calls.length = 0
    hub.disks.clear()
    const snap = makeSnap('p-commit', [sourceEntry('keyed', 'Commit')])
    hub.snapshots.set('p-commit', snap)
    const project = await import('./project')
    await project.projectStore.open('p-commit')

    hub.failClears = true
    project.projectStore.setDraft('keyed·Commit', 'черновик к коммиту')
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 4000, interval: 25 },
    )
    project.projectStore.commit('keyed·Commit')
    // The commit's ack clears the in-memory draft; the durable clear FAILS
    // — the sync status must be an honest error, not a silent saved.
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('error'),
      { timeout: 6000, interval: 25 },
    )
    expect(hub.disks.get('p-commit')?.has('ru::keyed·Commit')).toBe(true)
    hub.failClears = false
    project.projectStore.retryDraftsSync()
    await vi.waitFor(
      () => expect(project.getState().draftsSync.status).toBe('saved'),
      { timeout: 6000, interval: 25 },
    )
    expect(hub.disks.get('p-commit')?.has('ru::keyed·Commit')).toBe(false)
  })
})

// Review iteration 7 P0-F: a re-edit after a FAILED revert SUPERSEDES the
// pending clear — the newer text must ride the next save and survive.
it('edit after a failed revert supersedes the deletion (P0-F)', { timeout: 20000 }, async () => {
  vi.resetModules()
  const hub = (await import('../client/instance')).clientInstance as unknown as Hub
  hub.calls.length = 0
  hub.disks.clear()
  hub.snapshots.set('p-f', makeSnap('p-f', [sourceEntry('keyed', 'Target')]))
  const project = await import('./project')
  await project.projectStore.open('p-f')

  project.projectStore.setDraft('keyed·Target', 'ORIGINAL')
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('saved'), {
    timeout: 4000,
    interval: 25,
  })
  hub.failClears = true
  project.projectStore.clearDraft('keyed·Target')
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('error'), {
    timeout: 4000,
    interval: 25,
  })

  // The user re-edits the SAME row: NEW_UNCOMMITTED_WORK.
  hub.failClears = false
  project.projectStore.setDraft('keyed·Target', 'NEW_UNCOMMITTED_WORK')
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('saved'), {
    timeout: 4000,
    interval: 25,
  })
  expect(hub.disks.get('p-f')?.get('ru::keyed·Target')?.text).toBe('NEW_UNCOMMITTED_WORK')

  // Reopen: the newer text is what comes back — nothing lost.
  await project.projectStore.open('p-f')
  expect(project.getState().drafts['keyed·Target']).toBe('NEW_UNCOMMITTED_WORK')
})

// Review iteration 8 P1-B: the round trip is a REAL A → B → A navigation
// (the previous version opened p-g twice — 'in' on a Map plus the late
// registration made the second project unreachable).
it('a pending-clear key does not resurrect across a project round trip (P0-G)', { timeout: 25000 }, async () => {
  vi.resetModules()
  const hub = (await import('../client/instance')).clientInstance as unknown as Hub
  hub.calls.length = 0
  hub.disks.clear()
  hub.snapshots.set('p-g', makeSnap('p-g', [sourceEntry('keyed', 'Shared'), sourceEntry('keyed', 'Other')]))
  hub.snapshots.set('p-g2', makeSnap('p-g2', [sourceEntry('keyed', 'Other')]))
  const project = await import('./project')
  await project.projectStore.open('p-g')

  project.projectStore.setDraft('keyed·Shared', 'SHOULD_BE_REVERTED')
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('saved'), {
    timeout: 4000,
    interval: 25,
  })
  hub.failClears = true
  project.projectStore.clearDraft('keyed·Shared')
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('error'), {
    timeout: 4000,
    interval: 25,
  })

  // TRUE round trip: A → B → A (distinct project ids).
  await project.projectStore.open('p-g2')
  expect(project.getState().snapshot?.project_id).toBe('p-g2')
  await project.projectStore.open('p-g')
  expect(project.getState().snapshot?.project_id).toBe('p-g')

  // Retry on A: the deletion acks; the pending-clear key must not
  // resurrect in memory or on disk.
  hub.failClears = false
  project.projectStore.retryDraftsSync()
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('saved'), {
    timeout: 6000,
    interval: 25,
  })
  expect(hub.disks.get('p-g')?.has('ru::keyed·Shared')).toBe(false)
  expect(project.getState().drafts['keyed·Shared']).toBeUndefined()

  // Editing ANOTHER row (in B) afterwards must not resurrect it.
  await project.projectStore.open('p-g2')
  project.projectStore.setDraft('keyed·Other', 'правка в B')
  await vi.waitFor(() => expect(project.getState().draftsSync.status).toBe('saved'), {
    timeout: 4000,
    interval: 25,
  })
  await project.projectStore.open('p-g')
  // A clean reopen has nothing to autosave — the sync status honestly
  // stays idle. The resurrection check is the durable store + the view.
  await new Promise((r) => setTimeout(r, 1200))
  expect(hub.disks.get('p-g')?.has('ru::keyed·Shared')).toBe(false)
  expect(project.getState().drafts['keyed·Shared']).toBeUndefined()
  expect(hub.disks.get('p-g2')?.get('ru::keyed·Other')?.text).toBe('правка в B')
})