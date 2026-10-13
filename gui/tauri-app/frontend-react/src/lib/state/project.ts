// Project state (React lane) — domain mirror over the RimLocClient contract.
// Semantics mirror the frozen Svelte store: snapshot → workspace entries
// (entries × translations for the target locale), dirty drafts, busy/error,
// persist-before-ack (the acked revision is the apply base).
import type {
  ApplyIntentsRequestDto,
  DraftEntryDto,
  DraftsRestoreReportDto,
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
import { draftsStoreKey, entryUid, isLocaleScopedStoreKey } from './entryUid'

export interface WorkspaceEntry {
  /** FULL structural identity — intents address entries by exact match. */
  identity: SourceEntryIdDto
  /** Canonical UI key: the full-identity serialization (`entryUid`). Every
   *  map here — selection, drafts, validator findings, chat-batch handoff —
   *  addresses entries by THIS, never by the bare display key (two Def
   *  types can share one display key; review P0-2). */
  uid: string
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

// --- Durable draft autosave (Data Safety, priority #1). setDraft больше не
//  держит текст только в памяти: через debounce 1.5s вся карта черновиков
//  уходит контрактной командой project_save_drafts в sidecar-хранилище
//  `<id>.drafts.json` (epoch-guard, живую ревизию ставит сервис). Ошибка
//  автосейва — честный статус + Retry, тихой потери работы нет. Черновики
//  НИКОГДА не становятся подтверждёнными переводами: никакой apply/origin/
//  completeness через этот конвейер не идёт. ---
//
//  Store-key convention (review P0-1/P0-2 fix): the durable map is keyed by
//  `<target-locale>::<entryUid>` (draftsStoreKey), so parallel target
//  locales keep PARALLEL unsaved slices — a locale switch re-points the
//  view at another slice and never erases the previous one. `state.drafts`
//  stays the ACTIVE-LOCALE view (uid → text); `draftsAll` below is the
//  full cross-locale map that every flush persists. Legacy v1 stores held
//  bare display keys; adoptRestoredDrafts re-anchors them on restore.
export type DraftsSyncStatus = 'idle' | 'dirty' | 'saving' | 'saved' | 'error'

export interface DraftsSyncState {
  status: DraftsSyncStatus
  error: string | null
}

/** Восстановление при open(): поднятые черновики и честные уведомления. */
export interface DraftsNotice {
  kind: 'restored' | 'orphan' | 'unavailable'
  /** Для restored — сколько правок поднято; для остальных — путь/деталь. */
  count?: number
  fromBackup?: boolean
  path?: string
  detail?: string
}

const initialDraftsSync: DraftsSyncState = { status: 'idle', error: null }

const DRAFT_FLUSH_DELAY_MS = 1500

// F1: the target locale is PER-PROJECT state — it survives restarts in
// localStorage under `rimloc.target.<project_id>`. Without it, every fresh
// session opened the project on the default 'ru' while the corpus lived in
// the locale the user had switched to: the workspace silently showed an
// empty translation column and a build/export reparsed 0 keys.
const TARGET_LOCALE_KEY_PREFIX = 'rimloc.target.'
const DEFAULT_TARGET_LOCALE = 'ru'

/** Legacy/default source side: projects without the context marker ARE
 *  English by contract (backend fallback is the same constant). */
const DEFAULT_SOURCE_LOCALE = 'en'
const DEFAULT_SOURCE_DIR = 'English'

/** Read the source marker off an adopted snapshot (additive context fields;
 *  absent/undefined = English — same fallback the backend applies). */
function sourceMarkerOf(snap: ProjectSnapshotDto): { locale: string; dir: string } {
  const locale = snap.project.context.source_locale?.trim() || DEFAULT_SOURCE_LOCALE
  const dir = snap.project.context.source_dir?.trim() || folderForm(locale)
  return { locale, dir }
}

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
  /** SOURCE language marker (source-language wave): the locale id the
   *  backend built the inventory from ('ja'), read from the adopted
   *  snapshot's project.context. Absent on legacy snapshots = English.
   *  The UI renders THIS — never the app-level SOURCE_LOCALE constant —
   *  so a Japanese-source project can never display as English while the
   *  backend treats it as Japanese. */
  sourceLocale: string
  /** RimWorld Languages/<folder> of the source side ('Japanese');
   *  folder-form companion of sourceLocale (context.source_dir). */
  sourceDir: string
  summaries: ProjectSummaryDto[]
  snapshot: ProjectSnapshotDto | null
  entries: WorkspaceEntry[]
  selectedKey: string | null
  /** ACTIVE-LOCALE draft view (entryUid → text). The cross-locale truth
   *  lives in the module-level `draftsAll` and rides every flush. */
  drafts: Record<string, string>
  /** Автосейв черновиков: статус конвейера durable-хранилища (Error + Retry). */
  draftsSync: DraftsSyncState
  /** Inline-уведомление восстановления после open() (dismissable). */
  draftsNotice: DraftsNotice | null
  busy: boolean
  load: LoadState
  lastError: string | null
  validation: ValidationState
  perfMarks: PerfMarks
}

let state: ProjectState = {
  targetLocale: 'ru',
  sourceLocale: DEFAULT_SOURCE_LOCALE,
  sourceDir: DEFAULT_SOURCE_DIR,
  summaries: [],
  snapshot: null,
  entries: [],
  selectedKey: null,
  drafts: {},
  draftsSync: initialDraftsSync,
  draftsNotice: null,
  busy: false,
  load: { kind: 'idle' },
  lastError: null,
  validation: initialValidation,
  perfMarks: {},
}

// Cross-locale draft truth: durable-store key → text. Reset ONLY on a
// project transition (open/create/reset) — locale switches keep it whole.
let draftsAll: Record<string, string> = {}

/** Last-resort in-memory guard (review P0-3): when a project transition
 *  cannot durably flush pending edits, the map is preserved here per
 *  project id — reopening raises it with an honest notice instead of
 *  destroying the work. Never consulted while a durable copy exists. */
let unsavedMemory: Record<string, Record<string, string>> = {}

const listeners = new Set<() => void>()

/** §10: the validation report describes ONE snapshot — every mutation that
 *  adopts a different snapshot makes the previous report stale. */

function set(patch: Partial<ProjectState>): void {
  state = { ...state, ...patch }
  listeners.forEach((l) => l())
}

// --- Автосейв-конвейер черновиков: monotonic версия карты (гонки flush),
// серийная очередь записей (последняя карта попадает в хранилище последней)
// и debounce 1.5s на setDraft. Каждая запись уходит epoch-guarded; сервис
// ставит живую ревизию. ---

let draftsVersion = 0
let draftFlushTimer: number | null = null
let draftSyncChain: Promise<void> = Promise.resolve()

/** The ACTIVE-LOCALE slice of the full map — what the UI sees. */
function draftsViewOf(all: Record<string, string>, locale: string): Record<string, string> {
  const prefix = `${locale}::`
  const view: Record<string, string> = {}
  for (const [k, v] of Object.entries(all)) {
    if (k.startsWith(prefix)) view[k.slice(prefix.length)] = v
  }
  return view
}

function currentDraftEntries(): Record<string, DraftEntryDto> {
  const now = Date.now()
  const entries: Record<string, DraftEntryDto> = {}
  for (const [storeKey, text] of Object.entries(draftsAll)) {
    entries[storeKey] = { text, updated_at: now }
  }
  return entries
}

/** Немедленная запись всей карты черновиков в durable-хранилище. Записи
 *  сериализованы цепочкой (запись N+1 стартует после N), статус честно
 *  отражает исход: saved / error (+ Retry). Flush устаревшей сессии
 *  (epoch/project сменились) молча отпускается — за состояние отвечает
 *  уже новая сессия. */
function flushDraftsNow(): Promise<void> {
  const snap = state.snapshot
  if (!snap) return Promise.resolve()
  const capturedVersion = draftsVersion
  const capturedPid = snap.project_id
  const capturedEpoch = snap.session_epoch
  const entries = currentDraftEntries()
  draftSyncChain = draftSyncChain.then(async () => {
    // К моменту старта записи сессия могла смениться — не пишем мимо неё.
    const live = state.snapshot
    if (!live || live.project_id !== capturedPid || live.session_epoch !== capturedEpoch) return
    if (state.draftsSync.status !== 'error') {
      set({ draftsSync: { status: 'saving', error: null } })
    }
    try {
      await clientInstance.getClient().saveDrafts(capturedPid, capturedEpoch, entries)
      // Пока шла запись, были новые правки — статус не «saved», в очередь
      // встанет свежий flush (trailing timer уже поставлен setDraft'ом).
      if (
        state.snapshot?.project_id === capturedPid &&
        state.snapshot?.session_epoch === capturedEpoch &&
        draftsVersion === capturedVersion
      ) {
        set({ draftsSync: { status: 'saved', error: null } })
      } else if (state.draftsSync.status === 'saving') {
        set({ draftsSync: { status: 'dirty', error: null } })
      }
    } catch (e) {
      if (
        state.snapshot?.project_id === capturedPid &&
        state.snapshot?.session_epoch === capturedEpoch
      ) {
        // persist-before-ack в reverse: провал durable-черновика НЕ трогает
        // карту в памяти — работа не теряется, статус зовёт Retry.
        set({
          draftsSync: {
            status: 'error',
            error: e instanceof Error ? e.message : String(e),
          },
        })
      }
    }
  })
  return draftSyncChain
}

function scheduleDraftFlush(delay = DRAFT_FLUSH_DELAY_MS): void {
  if (draftFlushTimer !== null) window.clearTimeout(draftFlushTimer)
  draftFlushTimer = window.setTimeout(() => {
    draftFlushTimer = null
    void flushDraftsNow()
  }, delay)
}

function cancelScheduledDraftFlush(): void {
  if (draftFlushTimer !== null) {
    window.clearTimeout(draftFlushTimer)
    draftFlushTimer = null
  }
}

/** Project-transition guard (review P0-3): a debounced flush CANCELLED by
 *  navigation used to silently drop edits younger than the 1.5s debounce.
 *  Every project transition now QUIESCES first — cancel the timer, run the
 *  flush to its durable outcome — and a failed write preserves the full
 *  map in unsavedMemory for the project it belongs to. */
async function quiesceDraftsForTransition(): Promise<void> {
  const status = state.draftsSync.status
  const pending =
    draftFlushTimer !== null ||
    status === 'dirty' ||
    status === 'saving' ||
    status === 'error'
  if (!pending) return
  if (draftFlushTimer !== null) cancelScheduledDraftFlush()
  const pid = state.snapshot?.project_id
  await flushDraftsNow()
  if (state.draftsSync.status === 'error' && pid && Object.keys(draftsAll).length > 0) {
    unsavedMemory[pid] = { ...unsavedMemory[pid], ...draftsAll }
  }
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
  // Keyed by the FULL structural identity (review P0-2): two Def types
  // sharing one display key are TWO rows, two drafts, two translations.
  const byUid = new Map<string, WorkspaceEntry>()
  for (const e of snap.project.entries) {
    byUid.set(entryUid(e.id), {
      identity: e.id,
      uid: entryUid(e.id),
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
    const hit = byUid.get(entryUid(tr.source_id))
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
  return [...byUid.values()]
}

// --- §9/§17: чистые проекции отчёта валидатора на инвентарь. ---

/** Ключ инвентаря (entryUid-пространство), к которому относится находка:
 *  `id` несёт ПОЛНУЮ структурную идентичность — сериализуем её тем же
 *  каноном, что и весь воркспейс (review P0-2); находки без `id` —
 *  проектного уровня (case-collision, source-drift), их `key` — сырой
 *  текст валидатора, до строки воркспейса он не резолвится. */
export function findingEntryKey(f: ValidationFindingDto): string {
  return f.id ? entryUid(f.id) : f.key
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
  /** J1: real create (the Rust scan IS the preview) → adopted snapshot.
   *  Additive (source-language wave): `source` names the SOURCE language —
   *  the wizard's select passes the registry id + folder form; omitted =
   *  English (the exact legacy call). */
  async createContractProject(
    modRoot: string,
    targetVersion?: string,
    source?: { locale?: string; dir?: string },
  ): Promise<boolean> {
    // Pending edits of the PREVIOUS project go to storage (or the memory
    // guard) before the workspace flips — never into the void (review P0-3).
    await quiesceDraftsForTransition()
    set({ busy: true, lastError: null })
    try {
      const snap = await clientInstance.getClient().createProject(modRoot, targetVersion, source)
      // F1: the fresh project starts on the CURRENT editing locale; the
      // wizard's own choice lands right after via setTargetLocale. Both
      // writes persist under rimloc.target.<project_id>.
      const locale = state.targetLocale
      const sourceMarker = sourceMarkerOf(snap)
      persistTargetLocale(snap.project_id, locale)
      draftsAll = {}
      set({
        snapshot: snap,
        targetLocale: locale,
        sourceLocale: sourceMarker.locale,
        sourceDir: sourceMarker.dir,
        entries: mapSnapshot(snap, locale),
        selectedKey: null,
        drafts: {},
        draftsSync: initialDraftsSync,
        draftsNotice: null,
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
    // Review P0-3: quiesce BEFORE the flip — edits younger than the
    // debounce are flushed to durable storage (or preserved in
    // unsavedMemory on failure), never cancelled into the void.
    await quiesceDraftsForTransition()
    const T0 = performance.now()
    set({
      busy: true,
      lastError: null,
      draftsSync: initialDraftsSync,
      draftsNotice: null,
      perfMarks: { T0_openRequested: T0 },
    })
    try {
      const snap = await clientInstance.getClient().openProject(projectId)
      // F1: hydrate the per-project target BEFORE mapping — a fresh session
      // restores the locale the user last edited this project in; no stored
      // key falls back to the historical default 'ru'.
      const locale = loadTargetLocale(projectId) ?? DEFAULT_TARGET_LOCALE
      const T2 = performance.now()
      const mapped = mapSnapshot(snap, locale)
      const T4 = performance.now()
      const sourceMarker = sourceMarkerOf(snap)
      // In-memory guard of a PREVIOUS failed autosave for THIS project:
      // raise it alongside whatever the durable store still holds (the
      // guard is the newest generation — it wins per key).
      const carried = unsavedMemory[projectId]
      delete unsavedMemory[projectId]
      draftsAll = carried ? { ...carried } : {}
      set({
        perfMarks: { ...state.perfMarks, T2_ipcComplete: T2, T3_stateReceived: T2, T4_entriesMapped: T4 },
        snapshot: snap,
        targetLocale: locale,
        // Source marker rides the snapshot (source-language wave): legacy
        // snapshots carry no marker and stay English here as well.
        sourceLocale: sourceMarker.locale,
        sourceDir: sourceMarker.dir,
        entries: mapped,
        selectedKey: null,
        drafts: draftsViewOf(draftsAll, locale),
        draftsSync: initialDraftsSync,
        draftsNotice: null,
        busy: false,
        // Открытие — прежний отчёт валидации устарел (§9): новый проект или
        // свежая сессия; воркспейс пере-гоняет валидацию по stale-флагу.
        validation: { ...initialValidation, stale: true },
      })
      // Data Safety: поднять несохранённую работу из sidecar-хранилища
      // (ревизионный гард и quarantine — на сервисе). Отдельный async-шаг:
      // черновики не могут уронить открытие проекта.
      void projectStore.adoptRestoredDrafts(projectId, snap.session_epoch)
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
   *  session opens the project on the same corpus slice. Review P0-1:
   *  черновики живут в durable-хранилище ПАРАЛЛЕЛЬНЫМИ locale-срезами —
   *  смена цели перепригивает вью на другой срез и НЕ стирает предыдущий
   *  (прежний код обнулял карту и записывал ПУСТОЙ store, уничтожая
   *  несохранённую работу всех остальных языков). Store не менялся —
   *  flush не нужен. */
  setTargetLocale(locale: string): void {
    if (!state.snapshot || state.targetLocale === locale) return
    persistTargetLocale(state.snapshot.project_id, locale)
    set({
      targetLocale: locale,
      entries: mapSnapshot(state.snapshot, locale),
      drafts: draftsViewOf(draftsAll, locale),
    })
  },

  setDraft(key: string, text: string): void {
    draftsVersion++
    // The full cross-locale map is the durable truth; the view gets the
    // active-locale record under its locale-scoped store key.
    draftsAll = { ...draftsAll, [draftsStoreKey(state.targetLocale, key)]: text }
    set({
      drafts: { ...state.drafts, [key]: text },
      draftsSync: { status: 'dirty', error: state.draftsSync.error },
    })
    scheduleDraftFlush()
  },

  /** Revert (§11): a clean draft is the ABSENCE of the record — editor and
   *  the shell dirty-pill derive dirtiness from that (same convention as
   *  commit, which deletes the key on ack). Writing the committed text back
   *  as a draft would leave a phantom unsaved record. Удаление уходит и в
   *  durable-хранилище — сразу, без debounce (только активный locale-срез
   *  ключа; чужие языки не трогаются). */
  clearDraft(key: string): void {
    if (!(key in state.drafts)) return
    const drafts = { ...state.drafts }
    delete drafts[key]
    const all = { ...draftsAll }
    delete all[draftsStoreKey(state.targetLocale, key)]
    draftsAll = all
    draftsVersion++
    set({ drafts, draftsSync: { status: 'dirty', error: state.draftsSync.error } })
    void flushDraftsNow()
  },

  /** Commit one draft as a set_translation intent (persist-before-ack). */
  async commit(key: string, next = false): Promise<boolean> {
    const snap = state.snapshot
    const entry = state.entries.find((e) => e.uid === key)
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
      const all = { ...draftsAll }
      delete all[draftsStoreKey(state.targetLocale, key)]
      draftsAll = all
      draftsVersion++
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
      // Подтверждённый ключ уходит и из durable-хранилища черновиков —
      // немедленная запись полной карты (сервис поставит свежую ревизию).
      void flushDraftsNow()
      void projectStore.refreshValidation()
      if (next) {
        const idx = state.entries.findIndex((e) => e.uid === key)
        const nx = state.entries[idx + 1]
        if (nx) set({ selectedKey: nx.uid })
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
      const sourceMarker = sourceMarkerOf(fresh)
      // Review P0-1: внешняя мутация (existing-import, chat-batch apply)
      // больше НЕ стирает черновики — ни из памяти, ни из durable-хранилища
      // (locale-скоуп ключей делает их независимыми от принятого снапшота;
      // реверт вручную, работа не уничтожается молча).
      set({
        snapshot: fresh,
        targetLocale: locale,
        // Source marker re-read from the fresh snapshot — an external
        // mutation never flips the displayed source away from the backend's.
        sourceLocale: sourceMarker.locale,
        sourceDir: sourceMarker.dir,
        entries: mapSnapshot(fresh, locale),
        drafts: draftsViewOf(draftsAll, locale),
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

  /** Data Safety: поднять несохранённую работу после open(). Решение
   *  принимает сервис (ревизионный гард, quarantine сироты, восстановление
   *  из бэкапа) — фронтенд только честно показывает отчёт. Никогда не
   *  роняет открытие проекта.
   *
   *  Review P0-1/P0-2: restored keys are LOCALE-SCOPED composite store
   *  keys; legacy v1 keys (bare display keys) are re-anchored here —
   *  unique display-key match migrates into the persisted target locale's
   *  slice, an ambiguous match replicates to EVERY candidate (a draft is
   *  review-before-commit: over-preserving beats dropping work), and an
   *  unresolvable key survives verbatim, self-healing on a later
   *  inventory. Nothing is destroyed on restore. */
  async adoptRestoredDrafts(projectId: string, sessionEpoch: number): Promise<void> {
    let report: DraftsRestoreReportDto
    try {
      report = await clientInstance.getClient().restoreDrafts(projectId, sessionEpoch)
    } catch (e) {
      // Restore недоступен (например, транспорт ещё не знает команду) —
      // молча терять работу нельзя: честная заметка, проект открыт.
      if (state.snapshot?.project_id === projectId) {
        set({
          draftsNotice: {
            kind: 'unavailable',
            detail: e instanceof Error ? e.message : String(e),
          },
        })
      }
      return
    }
    // Сессия уже уехала вперёд (повторный open) — отчёт устарел.
    if (state.snapshot?.project_id !== projectId || state.snapshot?.session_epoch !== sessionEpoch) {
      return
    }
    const raiseView = (): void =>
      set({ drafts: draftsViewOf(draftsAll, state.targetLocale) })
    if (
      report.status === 'restored' &&
      report.drafts !== undefined &&
      Object.keys(report.drafts).length > 0
    ) {
      const legacyLocale = loadTargetLocale(projectId) ?? DEFAULT_TARGET_LOCALE
      const migrated: Record<string, string> = {}
      let legacyReanchored = 0
      let legacyAmbiguous = 0
      for (const [storeKey, entry] of Object.entries(report.drafts)) {
        if (isLocaleScopedStoreKey(storeKey)) {
          migrated[storeKey] = entry.text
          continue
        }
        const matches = state.entries.filter((e) => e.key === storeKey)
        if (matches.length === 1) {
          migrated[draftsStoreKey(legacyLocale, matches[0].uid)] = entry.text
          legacyReanchored++
        } else if (matches.length > 1) {
          for (const m of matches) migrated[draftsStoreKey(legacyLocale, m.uid)] = entry.text
          legacyAmbiguous++
        } else {
          migrated[storeKey] = entry.text
        }
      }
      // The in-memory guard (when present) is the newest generation — it
      // wins per key; the store fills everything else.
      draftsAll = { ...migrated, ...draftsAll }
      raiseView()
      const viewCount = Object.keys(state.drafts).length
      const extras: string[] = []
      if (report.restored_count > viewCount) {
        extras.push(`+${report.restored_count - viewCount} in other target locales`)
      }
      if (legacyReanchored + legacyAmbiguous > 0) {
        extras.push(
          `legacy drafts re-anchored: ${legacyReanchored}` +
            (legacyAmbiguous > 0 ? `, ambiguous replicated: ${legacyAmbiguous}` : ''),
        )
      }
      set({
        draftsNotice: {
          kind: 'restored',
          count: report.restored_count,
          fromBackup: report.recovered_from_backup !== undefined,
          detail: extras.length > 0 ? extras.join('; ') : undefined,
        },
      })
    } else if (report.status === 'orphan_quarantined') {
      raiseView()
      set({
        draftsNotice: {
          kind: 'orphan',
          path: report.orphan_path?.path,
          detail: report.detail,
        },
      })
    } else if (report.status === 'unavailable') {
      raiseView()
      set({ draftsNotice: { kind: 'unavailable', detail: report.detail } })
    } else if (Object.keys(draftsAll).length > 0) {
      // The durable store had nothing, but the in-memory guard of a
      // previous failed autosave did — raise it honestly.
      raiseView()
      set({
        draftsNotice: {
          kind: 'restored',
          count: Object.keys(state.drafts).length,
          detail: 'restored after a failed autosave of the previous session',
        },
      })
    }
  },

  /** Retry автосейва после ошибки: немедленный повтор записи хранилища. */
  retryDraftsSync(): void {
    void flushDraftsNow()
  },

  /** Скрыть inline-уведомление восстановления (dismissable). */
  dismissDraftsNotice(): void {
    set({ draftsNotice: null })
  },

  reset(): void {
    // Teardown keeps the same honesty as a transition: whatever the debounce
    // had not yet written is preserved in the per-project memory guard.
    const pid = state.snapshot?.project_id
    if (pid && Object.keys(draftsAll).length > 0) {
      unsavedMemory[pid] = { ...unsavedMemory[pid], ...draftsAll }
    }
    if (draftFlushTimer !== null) cancelScheduledDraftFlush()
    draftsAll = {}
    draftsVersion++
    set({
      summaries: [],
      snapshot: null,
      entries: [],
      selectedKey: null,
      drafts: {},
      draftsSync: initialDraftsSync,
      draftsNotice: null,
      busy: false,
      load: { kind: 'idle' },
      lastError: null,
      validation: initialValidation,
      sourceLocale: DEFAULT_SOURCE_LOCALE,
      sourceDir: DEFAULT_SOURCE_DIR,
    })
  },
}

// Crash/close guard (review P0-3 scenario list): the debounced autosave
// must not depend on the 1.5s timer surviving a window close or a hidden
// tab — any pagehide / visibility loss flushes immediately. Best-effort by
// nature (an async IPC may not complete during teardown); the
// quiesce-on-transition guard and the sidecar backups remain the durable
// layers beneath it.
if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  const flushOnTeardown = (): void => {
    const status = state.draftsSync.status
    if (draftFlushTimer === null && status !== 'dirty' && status !== 'saving') return
    if (draftFlushTimer !== null) cancelScheduledDraftFlush()
    void flushDraftsNow()
  }
  window.addEventListener('pagehide', flushOnTeardown)
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') flushOnTeardown()
  })
}
