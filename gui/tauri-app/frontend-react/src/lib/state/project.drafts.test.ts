// Review-verbatim regression scenarios (rel25 audit DO_NOW P0-1/P0-2/P0-3),
// run against the real store module with a mocked contract client:
//   1. drafts: ru → uk → ru → restart — no unsaved translation is lost;
//   2. structural identity collision: ThingDef/Shared.label +
//      HediffDef/Shared.label — two rows, two drafts, independent commits;
//   3. fast navigation: type → immediately switch project → return;
//   4. locale switch during the background save window (the 1.5s debounce);
//   5. adoptExternal (chat batch / existing import) keeps manual drafts;
//   6. failed autosave → in-memory guard raises the work on reopen;
//   7. legacy v1 store keys re-anchor on restore (never destroyed).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type {
  DraftEntryDto,
  DraftsRestoreReportDto,
  ProjectSnapshotDto,
  SourceEntryDto,
} from '../client/types'

// The hub is recreated by the mock factory on every vi.resetModules(); tests
// reach it through the mocked module import and configure per scenario.
vi.mock('../client/instance', () => {
  type Snap = import('../client/types').ProjectSnapshotDto
  type Ack = import('../client/types').DraftsSaveAckDto
  type DraftMap = Record<string, import('../client/types').DraftEntryDto>
  const hub = {
    calls: [] as { method: string; args: unknown[] }[],
    snapshots: new Map<string, Snap>(),
    restoreReports: new Map<string, import('../client/types').DraftsRestoreReportDto>(),
    failSaves: false,
    /** Every successful save, in order: project id + payload. */
    savedHistory: [] as { pid: string; entries: DraftMap }[],
    applyCount: 0,
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
        hub.applyCount++
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
      saveDrafts: async (pid: string, epoch: number, entries: DraftMap): Promise<Ack> => {
        hub.calls.push({ method: 'saveDrafts', args: [pid, epoch, entries] })
        if (hub.failSaves) throw new Error('save_failed: disk full')
        hub.savedHistory.push({ pid, entries })
        return { saved: Object.keys(entries).length, project_revision: 1, updated_at: 0 }
      },
      restoreDrafts: async (pid: string): Promise<DraftsRestoreReportDto> => {
        const report = hub.restoreReports.get(pid)
        return report ?? { status: 'none', restored_count: 0 }
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
  failSaves: boolean
  savedHistory: { pid: string; entries: Record<string, DraftEntryDto> }[]
  applyCount: number
}

async function freshLane(): Promise<{
  hub: Hub
  project: typeof import('./project')
}> {
  vi.resetModules()
  const hub = (await import('../client/instance')).clientInstance as unknown as Hub
  const project = await import('./project')
  return { hub, project }
}

/** Real-debounce settlements (the 1.5s timer) need a patient wait. */
function settle(check: () => void): Promise<void> {
  return vi.waitFor(check, { timeout: 4000, interval: 25 })
}

/** The mock hub is created ONCE per file (the vi.mock factory does not
 *  re-run on vi.resetModules), so debounced timers of PREVIOUS lanes can
 *  still append to it mid-test. Every lane starts from a clean slate and
 *  every assertion filters by PROJECT ID — no cross-test phantoms. */
function resetHub(hub: Hub): void {
  hub.calls.length = 0
  hub.savedHistory.length = 0
  hub.restoreReports.clear()
  hub.failSaves = false
}

function savePayloads(hub: Hub, pid: string): Record<string, DraftEntryDto>[] {
  return hub.savedHistory.filter((s) => s.pid === pid).map((s) => s.entries)
}

function sourceEntry(
  kind: string,
  key: string,
  defType?: string,
  text = 'source',
): SourceEntryDto {
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

beforeEach(() => {
  vi.useRealTimers()
  window.localStorage.clear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('review scenario 1 — ru → uk → ru → restart keeps every draft', () => {
  it('a locale switch never erases another slice and restart restores both', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    hub.snapshots.set('p-locale', makeSnap('p-locale', [sourceEntry('keyed', 'Settings')]))
    expect(await project.projectStore.open('p-locale')).toBe(true)
    expect(project.getState().targetLocale).toBe('ru')

    const uid = 'keyed·Settings'
    // The debounce flushes after 1.5s — drive it with fake timers.
    vi.useFakeTimers()
    project.projectStore.setDraft(uid, 'русский черновик v2')
    await vi.advanceTimersByTimeAsync(1600)
    expect(savePayloads(hub, 'p-locale').at(-1)?.[`ru::${uid}`]?.text).toBe('русский черновик v2')
    vi.useRealTimers()

    // Switch to uk: the ru slice must survive IN THE STORE, the view flips,
    // and NO flush may fire (the old defect flushed an EMPTY map here).
    const savesBeforeSwitch = savePayloads(hub, 'p-locale').length
    project.projectStore.setTargetLocale('uk')
    expect(project.getState().drafts[uid]).toBeUndefined()
    expect(savePayloads(hub, 'p-locale')).toHaveLength(savesBeforeSwitch)

    project.projectStore.setDraft(uid, 'український чернетка')
    await settle(() =>
      expect(savePayloads(hub, 'p-locale').at(-1)?.[`uk::${uid}`]?.text).toBe(
        'український чернетка',
      ),
    )
    // The same flush persisted BOTH locale slices — parallel, not overwritten.
    expect(savePayloads(hub, 'p-locale').at(-1)?.[`ru::${uid}`]?.text).toBe('русский черновик v2')

    // Back to ru: the original draft is still on the row.
    project.projectStore.setTargetLocale('ru')
    expect(project.getState().drafts[uid]).toBe('русский черновик v2')

    // RESTART: fresh module, same localStorage locale choice, sidecar = the
    // last durable map.
    const durable = savePayloads(hub, 'p-locale').at(-1)!
    const lane2 = await freshLane()
    lane2.hub.snapshots.set('p-locale', makeSnap('p-locale', [sourceEntry('keyed', 'Settings')]))
    lane2.hub.restoreReports.set('p-locale', {
      status: 'restored',
      drafts: durable,
      restored_count: Object.keys(durable).length,
    })
    expect(await lane2.project.projectStore.open('p-locale')).toBe(true)
    expect(lane2.project.getState().targetLocale).toBe('ru')
    expect(lane2.project.getState().drafts[uid]).toBe('русский черновик v2')
    lane2.project.projectStore.setTargetLocale('uk')
    expect(lane2.project.getState().drafts[uid]).toBe('український чернетка')
  })
})

describe('review scenario 2 — structural identity collision (P0-2)', () => {
  it('two Def types sharing one key stay two rows, two drafts, two commits', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    hub.snapshots.set(
      'p-collision',
      makeSnap('p-collision', [
        sourceEntry('def_injected', 'Shared.label', 'ThingDef', 'thing source'),
        sourceEntry('def_injected', 'Shared.label', 'HediffDef', 'hediff source'),
      ]),
    )
    await project.projectStore.open('p-collision')
    const st = project.getState()
    expect(st.entries).toHaveLength(2)
    const thingUid = 'def_injected·ThingDef·Shared.label'
    const hediffUid = 'def_injected·HediffDef·Shared.label'
    expect(st.entries.map((e) => e.uid).sort()).toEqual([hediffUid, thingUid].sort())

    project.projectStore.setDraft(thingUid, 'вещь')
    project.projectStore.setDraft(hediffUid, 'эффект')
    expect(Object.keys(project.getState().drafts)).toHaveLength(2)

    // Commit ONE of them — the intent carries the FULL identity.
    expect(await project.projectStore.commit(thingUid)).toBe(true)
    const apply = hub.calls.find((c) => c.method === 'applyIntents')
    const intent = (
      apply!.args[0] as { intents: { entry: unknown; locale: string }[] }
    ).intents[0]
    expect(intent.entry).toEqual({
      kind: 'def_injected',
      key: 'Shared.label',
      def_type: 'ThingDef',
    })
    expect(intent.locale).toBe('Russian')
    // Merge semantics: the committed key leaves the store EXPLICITLY.
    const clears = hub.calls.filter((c) => c.method === 'clearDrafts')
    expect(clears.some((c) => c.args[2] === `ru::${thingUid}`)).toBe(true)

    // The committed draft left the store; the sibling draft survived.
    await settle(() => {
      const last = savePayloads(hub, 'p-collision').at(-1)!
      expect(last[`ru::${thingUid}`]).toBeUndefined()
      expect(last[`ru::${hediffUid}`]?.text).toBe('эффект')
    })
    expect(project.getState().drafts[hediffUid]).toBe('эффект')
    expect(project.getState().drafts[thingUid]).toBeUndefined()
  })
})

describe('review scenario 3 — fast navigation (P0-3)', () => {
  it('type → immediately open another project flushes BEFORE the flip', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    hub.snapshots.set('pa', makeSnap('pa', [sourceEntry('keyed', 'Alpha')]))
    hub.snapshots.set('pb', makeSnap('pb', [sourceEntry('keyed', 'Beta')]))
    await project.projectStore.open('pa')
    project.projectStore.setDraft('keyed·Alpha', 'торопливый черновик')
    // No debounce wait — the user navigates within the 1.5s window.
    await project.projectStore.open('pb')
    const callIdx = hub.calls.map((c, i) => ({ c, i }))
    const paSave = callIdx.filter(({ c }) => c.method === 'saveDrafts' && c.args[0] === 'pa').at(-1)
    const openB = callIdx.filter(({ c }) => c.method === 'openProject' && c.args[0] === 'pb').at(-1)
    expect(paSave).toBeDefined()
    expect(openB).toBeDefined()
    expect(paSave!.i).toBeLessThan(openB!.i)
    const payload = paSave!.c.args[2] as Record<string, DraftEntryDto>
    expect(payload[`ru::keyed·Alpha`]?.text).toBe('торопливый черновик')

    // Returning to A raises the draft again (durable this time).
    const durable = savePayloads(hub, 'pa').at(-1)!
    hub.restoreReports.set('pa', {
      status: 'restored',
      drafts: durable,
      restored_count: Object.keys(durable).length,
    })
    await project.projectStore.open('pa')
    expect(project.getState().drafts['keyed·Alpha']).toBe('торопливый черновик')
  })

  it('a FAILED flush on navigation preserves the work in the memory guard', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    hub.snapshots.set('pa', makeSnap('pa', [sourceEntry('keyed', 'Alpha')]))
    hub.snapshots.set('pb', makeSnap('pb', [sourceEntry('keyed', 'Beta')]))
    await project.projectStore.open('pa')
    hub.failSaves = true
    project.projectStore.setDraft('keyed·Alpha', 'спасённый черновик')
    await settle(() => expect(project.getState().draftsSync.status).toBe('error'))
    await project.projectStore.open('pb') // quiesce fails again → guard stashes
    await project.projectStore.open('pa') // no durable store; the guard raises
    expect(project.getState().drafts['keyed·Alpha']).toBe('спасённый черновик')
    expect(project.getState().draftsNotice?.kind).toBe('restored')
    // Recovery re-arms: saving works again and the guard is consumed.
    hub.failSaves = false
    project.projectStore.setDraft('keyed·Alpha', 'спасённый и записанный')
    await settle(() => {
      expect(savePayloads(hub, 'pa').at(-1)?.[`ru::keyed·Alpha`]?.text).toBe(
        'спасённый и записанный',
      )
    })
  })
})

describe('review scenario 4 — locale switch inside the save window', () => {
  it('the trailing flush persists the OLD-locale edit instead of an empty map', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    hub.snapshots.set('p-window', makeSnap('p-window', [sourceEntry('keyed', 'Settings')]))
    await project.projectStore.open('p-window')
    vi.useFakeTimers()
    project.projectStore.setDraft('keyed·Settings', 'успел до переключения')
    project.projectStore.setTargetLocale('uk')
    expect(project.getState().drafts['keyed·Settings']).toBeUndefined()
    await vi.advanceTimersByTimeAsync(1600)
    const payload = savePayloads(hub, 'p-window').at(-1)!
    expect(payload[`ru::keyed·Settings`]?.text).toBe('успел до переключения')
    expect(payload[`uk::keyed·Settings`]).toBeUndefined()
    vi.useRealTimers()
  })
})

describe('review scenario 5 — adoptExternal keeps manual drafts', () => {
  it('an external snapshot adoption (chat batch / import) never wipes drafts', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    const snap = makeSnap('p-adopt', [sourceEntry('keyed', 'Settings')])
    hub.snapshots.set('p-adopt', snap)
    await project.projectStore.open('p-adopt')
    project.projectStore.setDraft('keyed·Settings', 'ручной черновик')
    await settle(() => expect(savePayloads(hub, 'p-adopt')).toHaveLength(1))
    // External mutation: revision bump, new translation from a chat batch.
    snap.revision += 1
    snap.project.translations.push({
      source_id: { kind: 'keyed', key: 'Settings' },
      locale: 'Russian',
      text: 'подтверждённый перевод',
      completeness: 'translated',
      review: '',
      validation: 'unknown',
      lifecycle: '',
      origin: 'llm',
    })
    await project.projectStore.adoptExternal()
    expect(project.getState().drafts['keyed·Settings']).toBe('ручной черновик')
    // And no flush may have wiped the durable slice with an empty map.
    expect(savePayloads(hub, 'p-adopt')).toHaveLength(1)
  })
})

describe('legacy v1 store keys re-anchor on restore (P0-2 back-compat)', () => {
  it('unique display keys migrate into the persisted locale; orphans survive', async () => {
    const { hub, project } = await freshLane()
    resetHub(hub)
    hub.snapshots.set(
      'p-legacy',
      makeSnap('p-legacy', [
        sourceEntry('keyed', 'Settings'),
        sourceEntry('def_injected', 'Shared.label', 'ThingDef'),
        sourceEntry('def_injected', 'Shared.label', 'HediffDef'),
      ]),
    )
    hub.restoreReports.set('p-legacy', {
      status: 'restored',
      drafts: {
        Settings: { text: 'черновик первого поколения', updated_at: 1 },
        'Shared.label': { text: 'неоднозначный ключ', updated_at: 1 },
        'Ghost.key': { text: 'сирота', updated_at: 1 },
      },
      restored_count: 3,
    })
    await project.projectStore.open('p-legacy')
    const st = project.getState()
    expect(st.drafts['keyed·Settings']).toBe('черновик первого поколения')
    // Ambiguous display key: replicated to BOTH structural entries (a draft
    // is review-before-commit — over-preserving beats dropping work).
    expect(st.drafts['def_injected·ThingDef·Shared.label']).toBe('неоднозначный ключ')
    expect(st.drafts['def_injected·HediffDef·Shared.label']).toBe('неоднозначный ключ')
    expect(st.draftsNotice?.kind).toBe('restored')
    // The orphan survives verbatim in the durable map (next flush keeps it).
    project.projectStore.setDraft('keyed·Settings', 'bump for flush')
    await settle(() => {
      const last = savePayloads(hub, 'p-legacy').at(-1)!
      expect(last['Ghost.key']?.text).toBe('сирота')
      expect(last[`ru::keyed·Settings`]?.text).toBe('bump for flush')
      expect(last[`ru::def_injected·ThingDef·Shared.label`]?.text).toBe('неоднозначный ключ')
    })
  })
})
