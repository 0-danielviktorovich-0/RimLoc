// Project state (React lane) — domain mirror over the RimLocClient contract.
// Semantics mirror the frozen Svelte store: snapshot → workspace entries
// (entries × translations for the target locale), dirty drafts, busy/error,
// persist-before-ack (the acked revision is the apply base).
import type {
  ApplyIntentsRequestDto,
  ProjectSnapshotDto,
  ProjectSummaryDto,
  SourceEntryIdDto,
  TranslationIntentDto,
} from '../client/types'
import { clientInstance } from '../client/instance'

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
}

export type LoadState =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'error'; message: string }

export interface ProjectState {
  summaries: ProjectSummaryDto[]
  snapshot: ProjectSnapshotDto | null
  entries: WorkspaceEntry[]
  selectedKey: string | null
  drafts: Record<string, string>
  busy: boolean
  load: LoadState
  lastError: string | null
}

let state: ProjectState = {
  summaries: [],
  snapshot: null,
  entries: [],
  selectedKey: null,
  drafts: {},
  busy: false,
  load: { kind: 'idle' },
  lastError: null,
}

const listeners = new Set<() => void>()

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
    })
  }
  for (const tr of snap.project.translations) {
    const hit = byKey.get(tr.source_id.key)
    if (!hit) continue
    if (tr.locale === locale) {
      hit.target = tr.text ?? ''
      hit.completeness = tr.completeness
      hit.review = tr.review
      hit.origin = tr.origin
    }
  }
  return [...byKey.values()]
}

export const projectStore = {
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
    set({ busy: true, lastError: null })
    try {
      const snap = await clientInstance.getClient().openProject(projectId)
      set({
        snapshot: snap,
        entries: mapSnapshot(snap, 'ru'),
        selectedKey: null,
        drafts: {},
        busy: false,
      })
      return true
    } catch (e) {
      set({ busy: false, lastError: e instanceof Error ? e.message : String(e) })
      return false
    }
  },

  select(key: string): void {
    set({ selectedKey: key })
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
      locale: 'ru',
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
        entries: mapSnapshot(fresh, 'ru'),
        drafts,
        busy: false,
      })
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
    })
  },
}
