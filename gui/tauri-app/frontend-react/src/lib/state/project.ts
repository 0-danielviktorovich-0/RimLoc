// Project state (React lane) — domain mirror over the RimLocClient contract.
// Semantics mirror the frozen Svelte store: snapshot → workspace entries
// (entries × translations for the target locale), dirty drafts, busy/error,
// persist-before-ack (the acked revision is the apply base).
import type {
  ApplyIntentsRequestDto,
  ProjectSnapshotDto,
  ProjectSummaryDto,
  SourceEntryIdDto,
  SourceProvenanceDto,
  TKeyMetaDto,
  TranslationIntentDto,
  ValidateProjectResponseDto,
} from '../client/types'
import { clientInstance } from '../client/instance'
import { folderForm } from '../languages/folderForm'

export interface WorkspaceEntry {
  /** FULL structural identity — intents address entries by exact match. */
  identity: SourceEntryIdDto
  /** Display key (entry.id wire = key rendering). */
  key: string
  source: string
  target: string
  completeness: string
  review: string
  lifecycle: string
  origin: string
  /** Per-entry Source Inspector projection (additive; absent = unknown). */
  sourceFile?: string
  sourceLine?: number
  selectedBy?: string
  /** View-selection facts behind the entry (additive Source Inspector):
   *  version, conditional LoadFolders branch, patch stage, winner reason. */
  provenance?: SourceProvenanceDto
  /** TKey serialization metadata with primary + other-usages locations. */
  tkey?: TKeyMetaDto
}

export interface PerfMarks {
  T0_openRequested?: number
  T1_serviceReady?: number
  T2_ipcComplete?: number
  T3_stateReceived?: number
  T4_entriesMapped?: number
  T5_firstRowRendered?: number
}

export type LoadState =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'error'; message: string }

/** §10 nav badge state: the LAST live validate report for the open project.
 *  `report: null` = stale/unknown — the shell honestly hides the badge
 *  instead of showing a count it cannot back. */
export interface ValidationState {
  report: ValidateProjectResponseDto | null
  busy: boolean
}

export interface ProjectState {
  /** Target locale the workspace edits (multi-target §32): picks which
   *  Project.translations slice the editor shows; switching re-maps. */
  targetLocale: string
  summaries: ProjectSummaryDto[]
  snapshot: ProjectSnapshotDto | null
  entries: WorkspaceEntry[]
  selectedKey: string | null
  drafts: Record<string, string>
  busy: boolean
  load: LoadState
  lastError: string | null
  validation: ValidationState
  perfMarks: PerfMarks
}

let state: ProjectState = {
  targetLocale: 'ru',
  summaries: [],
  snapshot: null,
  entries: [],
  selectedKey: null,
  drafts: {},
  busy: false,
  load: { kind: 'idle' },
  lastError: null,
  validation: { report: null, busy: false },
  perfMarks: {},
}

const listeners = new Set<() => void>()

/** §10: the validation report describes ONE snapshot — every mutation that
 *  adopts a different snapshot makes the previous report stale. */
const VALIDATION_STALE: ValidationState = { report: null, busy: false }

function set(patch: Partial<ProjectState>): void {
  state = { ...state, ...patch }
  listeners.forEach((l) => l())
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function getState(): ProjectState {
  return state
}

export function getPerfMarks(): PerfMarks {
  return state.perfMarks
}

function mapSnapshot(snap: ProjectSnapshotDto, locale: string): WorkspaceEntry[] {
  const byKey = new Map<string, WorkspaceEntry>()
  for (const e of snap.project.entries) {
    byKey.set(e.id.key, {
      identity: e.id,
      key: e.id.key,
      source: e.text,
      target: '',
      completeness: 'untranslated',
      review: '',
      lifecycle: '',
      origin: '',
      sourceFile: e.source_ref?.file,
      sourceLine: e.source_ref?.line,
      selectedBy: e.source_ref?.selected_by,
      provenance: e.provenance,
      tkey: e.tkey,
    })
  }
  for (const tr of snap.project.translations) {
    const hit = byKey.get(tr.source_id.key)
    if (!hit) continue
    // Locale forms: the store keeps the registry id ('ru'), while every
    // contract write path persists the strict folder form ('Russian' —
    // P1-2). Match both so folder-form applies (chat-batch, existing-import,
    // provider translate) display on a fresh open; the raw form stays
    // matched for legacy corpora written before the folder-form wave.
    if (tr.locale === locale || tr.locale === folderForm(locale)) {
      hit.target = tr.text ?? ''
      hit.completeness = tr.completeness
      hit.review = tr.review
      hit.origin = tr.origin
    }
  }
  return [...byKey.values()]
}

export const projectStore = {
  /** J1: real create (the Rust scan IS the preview) → adopted snapshot. */
  async createContractProject(modRoot: string, targetVersion?: string): Promise<boolean> {
    set({ busy: true, lastError: null })
    try {
      const snap = await clientInstance.getClient().createProject(modRoot, targetVersion)
      set({
        snapshot: snap,
        entries: mapSnapshot(snap, state.targetLocale),
        selectedKey: null,
        drafts: {},
        busy: false,
        validation: VALIDATION_STALE,
        perfMarks: { ...state.perfMarks, T3_stateReceived: performance.now(), T4_entriesMapped: performance.now() },
      })
      void projectStore.refreshValidation()
      return true
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      set({ busy: false, lastError: message })
      state.lastError = message
      return false
    }
  },

  get lastErrorText(): string | null {
    return state.lastError
  },

  async listProjects(): Promise<void> {
    set({ load: { kind: 'loading' }, lastError: null })
    try {
      const summaries = await clientInstance.getClient().listProjects()
      set({ summaries, load: { kind: 'idle' } })
    } catch (e) {
      set({ load: { kind: 'error', message: e instanceof Error ? e.message : String(e) } })
    }
  },

  async open(projectId: string): Promise<boolean> {
    const T0 = performance.now()
    set({ busy: true, lastError: null, perfMarks: { T0_openRequested: T0 } })
    try {
      const snap = await clientInstance.getClient().openProject(projectId)
      const T2 = performance.now()
      const mapped = mapSnapshot(snap, state.targetLocale)
      const T4 = performance.now()
      set({
        perfMarks: { ...state.perfMarks, T2_ipcComplete: T2, T3_stateReceived: T2, T4_entriesMapped: T4 },
        snapshot: snap,
        entries: mapped,
        selectedKey: null,
        drafts: {},
        busy: false,
        validation: VALIDATION_STALE,
      })
      void projectStore.refreshValidation()
      return true
    } catch (e) {
      set({ busy: false, lastError: e instanceof Error ? e.message : String(e) })
      return false
    }
  },

  select(key: string): void {
    set({ selectedKey: key })
  },

  /** Multi-target (§32): switch the edited target locale — translations
   *  re-map from the SAME snapshot (per-locale slices live side by side). */
  setTargetLocale(locale: string): void {
    if (!state.snapshot || state.targetLocale === locale) return
    set({ targetLocale: locale, entries: mapSnapshot(state.snapshot, locale), drafts: {} })
  },

  setDraft(key: string, text: string): void {
    set({ drafts: { ...state.drafts, [key]: text } })
  },

  /** Commit one draft as a set_translation intent (persist-before-ack). */
  async commit(key: string, next = false): Promise<boolean> {
    const snap = state.snapshot
    const entry = state.entries.find((e) => e.key === key)
    const draft = state.drafts[key]
    if (!snap || !entry || draft === undefined) return false
    const intent: TranslationIntentDto = {
      entry: entry.identity,
      // Folder contract (P1-2): every session.apply write persists the
      // strict language-folder form ('Russian'), same as chat-batch,
      // existing-import and the export/build calls — the raw registry id
      // ('ru') would split the translation corpus into a dead locale.
      locale: folderForm(state.targetLocale),
      action: 'set_translation',
      text: draft,
    }
    const req: ApplyIntentsRequestDto = {
      project_id: snap.project_id,
      expected_revision: snap.acked_revision ?? snap.revision,
      session_epoch: snap.session_epoch,
      intents: [intent],
    }
    set({ busy: true, lastError: null })
    try {
      const client = clientInstance.getClient()
      const resp = await client.applyIntents({
        projectId: req.project_id,
        expectedRevision: req.expected_revision,
        sessionEpoch: req.session_epoch,
        intents: req.intents,
      })
      // The ack moved the durable revision — adopt the fresh snapshot.
      const fresh = await client.snapshot(req.project_id)
      const drafts = { ...state.drafts }
      delete drafts[key]
      set({
        snapshot: fresh,
        // Перерисовка в АКТИВНУЮ цель (acceptance MUST-FIX: после коммита в uk
        // список на миг показывал ru — `mapSnapshot(fresh, 'ru')` игнорировал
        // targetLocale; данные не терялись, но фаза была видна живьём).
        entries: mapSnapshot(fresh, state.targetLocale),
        drafts,
        busy: false,
        validation: VALIDATION_STALE,
      })
      void projectStore.refreshValidation()
      if (next) {
        const idx = state.entries.findIndex((e) => e.key === key)
        const nx = state.entries[idx + 1]
        if (nx) set({ selectedKey: nx.key })
      }
      void resp
      return true
    } catch (e) {
      set({ busy: false, lastError: e instanceof Error ? e.message : String(e) })
      return false
    }
  },

  /** Re-adopt the durable snapshot after a contract mutation that did not
   *  flow through commit() (e.g. apply_existing on the Existing screen). */
  async adoptExternal(): Promise<void> {
    const snap = state.snapshot
    if (!snap) return
    try {
      const fresh = await clientInstance.getClient().snapshot(snap.project_id)
      // Перерисовка в АКТИВНУЮ цель (тот же класс бага, что закрыт в commit():
      // захардкоженная 'ru' игнорировала targetLocale).
      set({ snapshot: fresh, entries: mapSnapshot(fresh, state.targetLocale), drafts: {}, validation: VALIDATION_STALE })
      void projectStore.refreshValidation()
    } catch {
      /* refresh failures surface via the next contract call */
    }
  },

  /** §10: live validation over the open project's trusted session state.
   *  Failure or a mid-flight epoch/project change lands as `report: null`
   *  (honest unknown) — never as a fabricated clean report. */
  async refreshValidation(): Promise<void> {
    const snap = state.snapshot
    if (!snap) {
      set({ validation: VALIDATION_STALE })
      return
    }
    set({ validation: { report: state.validation.report, busy: true } })
    try {
      const report = await clientInstance.getClient().validateProject(snap.project_id, snap.session_epoch)
      const cur = state.snapshot
      // The session moved on while the request was in flight — drop it.
      if (cur && cur.project_id === snap.project_id && cur.session_epoch === snap.session_epoch) {
        set({ validation: { report, busy: false } })
      } else {
        set({ validation: VALIDATION_STALE })
      }
    } catch {
      set({ validation: VALIDATION_STALE })
    }
  },

  reset(): void {
    set({
      summaries: [],
      snapshot: null,
      entries: [],
      selectedKey: null,
      drafts: {},
      busy: false,
      load: { kind: 'idle' },
      lastError: null,
      validation: VALIDATION_STALE,
    })
  },
}
