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
  ValidationFindingDto,
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

/** §9 Issues-фильтр / §17 inline-warning: кэш живой валидации проекта.
 *  Отчёт отдаёт client.validateProject(projectId, epoch); `stale`
 *  поднимается на open/commit/adoptExternal (состояние проекта уехало
 *  вперёд отчёта) — воркспейс пере-гоняет валидацию по stale-флагу. */
export interface ValidationState {
  report: ValidateProjectResponseDto | null
  stale: boolean
  busy: boolean
  error: string | null
}

const initialValidation: ValidationState = {
  report: null,
  stale: false,
  busy: false,
  error: null,
}

// F1: the target locale is PER-PROJECT state — it survives restarts in
// localStorage under `rimloc.target.<project_id>`. Without it, every fresh
// session opened the project on the default 'ru' while the corpus lived in
// the locale the user had switched to: the workspace silently showed an
// empty translation column and a build/export reparsed 0 keys.
const TARGET_LOCALE_KEY_PREFIX = 'rimloc.target.'
const DEFAULT_TARGET_LOCALE = 'ru'

function loadTargetLocale(projectId: string): string | null {
  try {
    const raw = window.localStorage.getItem(TARGET_LOCALE_KEY_PREFIX + projectId)
    return raw && raw.trim() !== '' ? raw : null
  } catch {
    return null
  }
}

function persistTargetLocale(projectId: string, locale: string): void {
  try {
    window.localStorage.setItem(TARGET_LOCALE_KEY_PREFIX + projectId, locale)
  } catch {
    /* storage unavailable (private mode) — the choice stays session-only */
  }
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
  validation: initialValidation,
  perfMarks: {},
}

const listeners = new Set<() => void>()

/** §10: the validation report describes ONE snapshot — every mutation that
 *  adopts a different snapshot makes the previous report stale. */

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

// --- §9/§17: чистые проекции отчёта валидатора на инвентарь. ---

/** Ключ инвентаря, к которому относится находка: `id` несёт ПОЛНУЮ
 *  структурную идентичность (бэкенд резолвит её через (key, path), не по
 *  сериализационному ключу); находки без `id` — проектного уровня
 *  (case-collision, source-drift), их `key` — сырой текст валидатора.
 *  Маппинг по контракту задачи: f.id?.key / f.key. */
export function findingEntryKey(f: ValidationFindingDto): string {
  return f.id?.key ?? f.key
}

/** «С замечаниями»: множество ключей строк, присутствующих в находках. */
export function findingEntryKeys(report: ValidateProjectResponseDto | null): Set<string> {
  const keys = new Set<string>()
  if (!report) return keys
  for (const f of report.findings) keys.add(findingEntryKey(f))
  return keys
}

/** Находки по ключу строки (порядок отчёта сохранён) — «первая находка»
 *  для inline-warning редактора это первый элемент списка. */
export function findingsByEntryKey(
  report: ValidateProjectResponseDto | null,
): Map<string, ValidationFindingDto[]> {
  const byKey = new Map<string, ValidationFindingDto[]>()
  if (!report) return byKey
  for (const f of report.findings) {
    const k = findingEntryKey(f)
    const list = byKey.get(k)
    if (list) list.push(f)
    else byKey.set(k, [f])
  }
  return byKey
}

export const projectStore = {
  /** J1: real create (the Rust scan IS the preview) → adopted snapshot. */
  async createContractProject(modRoot: string, targetVersion?: string): Promise<boolean> {
    set({ busy: true, lastError: null })
    try {
      const snap = await clientInstance.getClient().createProject(modRoot, targetVersion)
      // F1: the fresh project starts on the CURRENT editing locale; the
      // wizard's own choice lands right after via setTargetLocale. Both
      // writes persist under rimloc.target.<project_id>.
      const locale = state.targetLocale
      persistTargetLocale(snap.project_id, locale)
      set({
        snapshot: snap,
        targetLocale: locale,
        entries: mapSnapshot(snap, locale),
        selectedKey: null,
        drafts: {},
        busy: false,
        // Новый проект — прежний отчёт валидации неприменим.
        validation: { ...initialValidation, stale: true },
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
      // F1: hydrate the per-project target BEFORE mapping — a fresh session
      // restores the locale the user last edited this project in; no stored
      // key falls back to the historical default 'ru'.
      const locale = loadTargetLocale(projectId) ?? DEFAULT_TARGET_LOCALE
      const T2 = performance.now()
      const mapped = mapSnapshot(snap, locale)
      const T4 = performance.now()
      set({
        perfMarks: { ...state.perfMarks, T2_ipcComplete: T2, T3_stateReceived: T2, T4_entriesMapped: T4 },
        snapshot: snap,
        targetLocale: locale,
        entries: mapped,
        selectedKey: null,
        drafts: {},
        busy: false,
        // Открытие — прежний отчёт валидации устарел (§9): новый проект или
        // свежая сессия; воркспейс пере-гоняет валидацию по stale-флагу.
        validation: { ...initialValidation, stale: true },
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
   *  re-map from the SAME snapshot (per-locale slices live side by side).
   *  F1: the choice persists under rimloc.target.<project_id> so the next
   *  session opens the project on the same corpus slice. */
  setTargetLocale(locale: string): void {
    if (!state.snapshot || state.targetLocale === locale) return
    persistTargetLocale(state.snapshot.project_id, locale)
    set({ targetLocale: locale, entries: mapSnapshot(state.snapshot, locale), drafts: {} })
  },

  setDraft(key: string, text: string): void {
    set({ drafts: { ...state.drafts, [key]: text } })
  },

  /** Revert (§11): a clean draft is the ABSENCE of the record — editor and
   *  the shell dirty-pill derive dirtiness from that (same convention as
   *  commit, which deletes the key on ack). Writing the committed text back
   *  as a draft would leave a phantom unsaved record. */
  clearDraft(key: string): void {
    if (!(key in state.drafts)) return
    const drafts = { ...state.drafts }
    delete drafts[key]
    set({ drafts })
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
        // Регрессия-логика (§9): фикс уехал в durable-состояние — отчёт
        // устарел, revalidate уберёт строку из «С замечаниями».
        validation: { ...state.validation, stale: true, error: null },
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
      // F1: re-hydrate the per-project locale when a stored choice exists
      // (mid-session adopt must never CLOBBER the in-session edit target —
      // hence state.targetLocale as the fallback, not the 'ru' default
      // used on fresh open). Snapshot adoption therefore never resets the
      // locale (WDIO regression note: locale → adopt keeps it).
      const locale = loadTargetLocale(snap.project_id) ?? state.targetLocale
      // Перерисовка в АКТИВНУЮ цель (тот же класс бага, что закрыт в commit():
      // захардкоженная 'ru' игнорировала targetLocale).
      set({
        snapshot: fresh,
        targetLocale: locale,
        entries: mapSnapshot(fresh, locale),
        drafts: {},
        // §9: внешняя мутация (existing-import, chat-batch apply) — отчёт
        // валидации устарел так же, как после commit().
        validation: { ...state.validation, stale: true, error: null },
      })
    } catch {
      /* refresh failures surface via the next contract call */
    }
  },

  /** §9/§17: живая валидация открытого проекта (read-only, epoch-guarded).
   *  Вызывается воркспейсом, когда `validation.stale`; успешный отчёт
   *  гасит stale, ошибка остаётся видимой и НЕ ретраится молча. */
  /** Shell-facing alias (nav badge §10): same live validation run. */
  async refreshValidation(): Promise<void> {
    await projectStore.revalidate()
  },

  async revalidate(): Promise<void> {
    const snap = state.snapshot
    if (!snap || state.validation.busy) return
    set({ validation: { ...state.validation, busy: true, error: null } })
    try {
      const report = await clientInstance.getClient().validateProject(snap.project_id, snap.session_epoch)
      set({ validation: { report, stale: false, busy: false, error: null } })
    } catch (e) {
      // Отчёт не обновлён: прежний (если был) помечен stale и остаётся
      // честным «прошлым срезом»; ошибка видима, авто-ретрая нет.
      set({
        validation: {
          ...state.validation,
          busy: false,
          stale: true,
          error: e instanceof Error ? e.message : String(e),
        },
      })
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
      validation: initialValidation,
    })
  },
}
