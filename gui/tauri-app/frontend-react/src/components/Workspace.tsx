// Workspace (R1 REPRESENTATIVE SCREEN): LEFT project context / CENTER
// virtualized inventory / RIGHT editor. One real project state; the entry
// list is virtualized from day one (mandate §24/§27 — the prototype had
// neither panes nor virtualization).
// R4: issues-фильтр из живого валидатора (§9), inline-warning первой находки
// (§17), честные табы Источник/Термины (§13), Related одного Def (§14) и
// AI entry «В чат-перевод» без автоматических вызовов провайдера (§15/§20).
import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react'
import { Group, Panel, Separator, useDefaultLayout } from 'react-resizable-panels'
import { useVirtualizer } from '@tanstack/react-virtual'
import { ArrowDown, ArrowUp, BookOpen, Braces, Check, ChevronDown, Circle, CircleAlert, Clock3, Copy, FileCode2, FolderOpen, ListFilter, MessagesSquare, PanelRight, RotateCcw, Save, Search, Sparkles } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore, findingEntryKeys, findingsByEntryKey, type DraftsSyncStatus, type WorkspaceEntry } from '../lib/state/project'
import { entryUid } from '../lib/state/entryUid'
import { readUiDensity } from '../lib/state/density'
import {
  sourceSearchStore,
  useSourceSearch,
  visibleMatches,
  type SourceSearchState,
} from '../lib/state/source_search_store'
import { glossaryStore, subscribe as glossarySubscribe, getState as glossaryGetState } from '../lib/state/glossary'
import { clientInstance } from '../lib/client/instance'
import { folderForm } from '../lib/languages/folderForm'
import { t, tEnum } from '../lib/i18n'
import type { GlossaryTermDto, SourceEntryIdDto, SourceXmlPreviewDto, SourceXmlPreviewLineDto, ValidationFindingDto } from '../lib/client/types'

const ROW_HEIGHT = 63
// UI-04: compact density shrinks the VIRTUALIZED row too — the
// virtualizer's fixed size must match the CSS row, so the height is
// re-read from the persisted density on every render (the shell
// re-renders on the density event).
const COMPACT_ROW_HEIGHT = 40

/** Display keys carried by more than one structural entry (review P0-2):
 *  their rows get a def-type discriminator in the label, so two Def types
 *  sharing one key never render as two identical lines. */
function duplicatedDisplayKeys(entries: readonly { key: string }[]): Set<string> {
  const seen = new Set<string>()
  const dup = new Set<string>()
  for (const e of entries) {
    if (seen.has(e.key)) dup.add(e.key)
    seen.add(e.key)
  }
  return dup
}

/** §15/§20 handoff: ключ sessionStorage, по которому экран чат-перевода
 *  (свой лейн) подхватывает предотмеченные строки. Значение — JSON
 *  массив display_identity()-строк; прочитал — удали. */
export const CHATBATCH_HANDOFF_KEY = 'rimloc.chatbatch.preset-keys'

/** §15: перенос выбранной строки в чат-перевод — sessionStorage + навигация.
 *  Никаких автоматических вызовов провайдера: транспорт решает человек.
 *  Ключ — общий канон полной структурной идентичности (entryUid, зеркало
 *  `SourceEntryId::display_identity()`): ChatBatch читает тот же формат. */
function sendToChatBatch(id: SourceEntryIdDto): void {
  try {
    const prev = sessionStorage.getItem(CHATBATCH_HANDOFF_KEY)
    const parsed: unknown = prev ? JSON.parse(prev) : []
    const list = Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === 'string') : []
    const next = new Set(list)
    next.add(entryUid(id))
    sessionStorage.setItem(CHATBATCH_HANDOFF_KEY, JSON.stringify([...next]))
  } catch {
    /* приватный режим/битый JSON — навигация всё равно честная */
  }
  window.location.hash = '#/chatbatch'
}

export function Workspace({ onBack }: { onBack: () => void }) {
  const st = useProjectState()
  const [search, setSearch] = useState('')
  // Large-corpus responsiveness (12k+ E2E): typing stays live while the
  // FILTER runs debounced — per-keystroke rememo over the whole
  // inventory measured ~3s at 12k rows.
  const [searchLive, setSearchLive] = useState('')
  useEffect(() => {
    const t = window.setTimeout(() => setSearch(searchLive), 150)
    return () => window.clearTimeout(t)
  }, [searchLive])
  const [filter, setFilter] = useState<'all' | 'empty' | 'issues'>('all')
  const [kindFilter, setKindFilter] = useState<string>('all')
  const [fileFilter, setFileFilter] = useState<string>('all')
  const [treeView, setTreeView] = useState<'type' | 'file'>('type')
  const [treeOpen, setTreeOpen] = useState(true)
  // UI-02 (owner audit Oct-14): three REAL workspace modes. The mode
  // changes the VIEW only — translations, unsaved drafts (projectStore +
  // textarea state), selection and target locale live in shared state and
  // survive every switch. Persisted per app (localStorage).
  const [wsMode, setWsMode] = useState<'studio' | 'focus' | 'overview'>(() => {
    const v = window.localStorage.getItem('rimloc.ws.mode')
    return v === 'focus' || v === 'overview' ? v : 'studio'
  })
  const switchMode = (m: 'studio' | 'focus' | 'overview'): void => {
    setWsMode(m)
    window.localStorage.setItem('rimloc.ws.mode', m)
  }
  // UI-03 (owner audit Oct-14): tree toggle hides the CONTENT (items +
  // views). True width-reclaiming panel collapse breaks the current
  // react-resizable-panels group (removing a panel unmounts the route) —
  // documented as PARTIAL in UI_DEFECTS_MATRIX.md; revisit with the
  // library upgrade.
  const toggleTree = (): void => setTreeOpen(!treeOpen)
  // UI-03 (owner audit Oct-14): the inspector toggle hides the right panel
  // via CSS (the panel stays mounted, avoiding the group crash).
  const [inspectorOpen, setInspectorOpen] = useState(true)
  const toggleInspector = (): void => setInspectorOpen(!inspectorOpen)
  // react-resizable-panels v4: `autoSaveId` replaced by the useDefaultLayout
  // hook — same localStorage persistence for the pane split, new API shape.
  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: 'rimloc-ws-panes',
    storage: window.localStorage,
  })

  const snapNow = st.snapshot
  // UI-05: output XML preview — the CANONICAL serializer rendered
  // in-memory for the current target locale, loaded lazily and cached.
  // P1-C freshness fence (review Oct-15 + follow-up): a request sequence
  // + a live context ref invalidate stale responses on ANY context change
  // (project, session, locale). The useEffect clears the stored result
  // when the identity shifts, so stale XML is never rendered as current.
  const outputPreviewSeq = useRef(0)
  const outputPreviewCtx = useRef({ project_id: '', locale: '' })
  const [outputPreview, setOutputPreview] = useState<
    | { status: 'idle' | 'loading' | 'error'; error?: string }
    | {
        status: 'done'
        locale: string
        revision: number
        files: { path: string; xml: string }[]
        truncated: boolean
      }
    | null
  >(null)
  // Track the CURRENT project/locale — clear stored output when they shift.
  const currentProjectId = snapNow?.project_id ?? ''
  const currentLocale = folderForm(st.targetLocale)
  useEffect(() => {
    const prev = outputPreviewCtx.current
    if (prev.project_id !== currentProjectId || prev.locale !== currentLocale) {
      // Context switch: invalidate ALL in-flight and stored preview data.
      outputPreviewSeq.current += 1
      outputPreviewCtx.current = { project_id: currentProjectId, locale: currentLocale }
      setOutputPreview(null)
    }
  }, [currentProjectId, currentLocale])
  const loadOutputPreview = (locale: string): void => {
    if (!snapNow) return
    const seq = ++outputPreviewSeq.current
    const reqCtx = { project_id: snapNow.project_id, locale }
    outputPreviewCtx.current = reqCtx
    setOutputPreview({ status: 'loading' })
    clientInstance
      .getClient()
      .projectOutputXmlPreview(snapNow.project_id, snapNow.session_epoch, locale)
      .then((res) => {
        // Freshness fence: discard if a newer request was fired or the
        // context shifted while in flight (ref always has latest values).
        if (seq !== outputPreviewSeq.current) return
        const ctx = outputPreviewCtx.current
        if (ctx.project_id !== reqCtx.project_id || ctx.locale !== locale) return
        setOutputPreview({
          status: 'done',
          locale: res.locale,
          revision: res.revision,
          files: res.files,
          truncated: res.truncated,
        })
      })
      .catch((e: unknown) => {
        if (seq !== outputPreviewSeq.current) return
        setOutputPreview({ status: 'error', error: e instanceof Error ? e.message : String(e) })
      })
  }
  // rel27: XML-предпросмотр — чтение исходного файла через IPC (read-only,
  // containment по source root).
  const loadXmlPreview = (
    file: string,
    line?: number
  ): Promise<SourceXmlPreviewDto> =>
    clientInstance.getClient().sourceXmlPreview({
      projectId: snapNow!.project_id,
      sessionEpoch: snapNow!.session_epoch,
      file,
      line,
      before: 6,
      after: 20
    })

  // §9: живая валидация. Отчёт устарел (open/commit/adoptExternal) →
  // пере-гоняем; ошибка не ретраится молча (гвард в revalidate + эффект).
  const vstate = st.validation
  useEffect(() => {
    if (!snapNow) return
    if (!vstate.stale || vstate.busy || vstate.error) return
    void projectStore.revalidate()
  }, [snapNow, vstate])

  // §13: глоссарий кэшируется once на проект; мутации глоссария поднимают
  // ревизию — эффект перечитывает только по факту сдвига ревизии.
  const glState = useSyncExternalStore(glossarySubscribe, glossaryGetState, glossaryGetState)
  useEffect(() => {
    if (!snapNow) return
    glossaryStore.ensureLoaded(snapNow.project_id, snapNow.session_epoch, snapNow.revision)
  }, [snapNow?.project_id, snapNow?.session_epoch, snapNow?.revision])

  // «С замечаниями» = ключи строк из находок отчёта (не эвристика).
  const issueKeys = useMemo(() => findingEntryKeys(vstate.report), [vstate.report])
  // D-V1: находки по ключу для статус-колонки списка строк.
  const findingsByKey = useMemo(() => findingsByEntryKey(vstate.report), [vstate.report])

  const kindGroups = useMemo(() => {
    const m = new Map<string, number>()
    for (const e of st.entries) {
      const k = e.identity.def_type ? `${e.identity.kind}/${e.identity.def_type}` : e.identity.kind
      m.set(k, (m.get(k) ?? 0) + 1)
    }
    return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  }, [st.entries])

  // UI-13 (owner audit Oct-14): the tree previously presented def types as
  // "project files". Two HONEST projections now: by entry type (def kinds)
  // and by ACTUAL source file from the canonical provenance
  // (source_ref.file); entries without provenance land in an explicit
  // unknown bucket — invented paths are never shown.
  const UNKNOWN_FILE = '__no_source_file__'
  // UI-13 follow-up (review Oct-15): files grouped by their REAL directory
  // prefix — a two-level honest tree (dir → file) built from provenance
  // paths only. Root-level files render under '·'.
  const fileGroups = useMemo(() => {
    const m = new Map<string, number>()
    let unknown = 0
    for (const e of st.entries) {
      const f = e.sourceFile?.trim()
      if (!f) {
        unknown += 1
        continue
      }
      m.set(f, (m.get(f) ?? 0) + 1)
    }
    const groups = [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]))
    if (unknown > 0) groups.push([UNKNOWN_FILE, unknown])
    return groups
  }, [st.entries])
  const fileDirGroups = useMemo(() => {
    const dirs = new Map<string, { file: string; n: number }[]>()
    for (const [file, n] of fileGroups) {
      const dir = file === UNKNOWN_FILE ? UNKNOWN_FILE : file.includes('/') ? file.slice(0, file.lastIndexOf('/')) : '·'
      const list = dirs.get(dir) ?? []
      list.push({ file, n })
      dirs.set(dir, list)
    }
    return [...dirs.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  }, [fileGroups])

  const visible = useMemo(() => {
    const q = search.trim().toLowerCase()
    return st.entries.filter((e) => {
      if (kindFilter !== 'all') {
        const k = e.identity.def_type ? `${e.identity.kind}/${e.identity.def_type}` : e.identity.kind
        if (k !== kindFilter) return false
      }
      if (fileFilter !== 'all') {
        const f = e.sourceFile?.trim() || UNKNOWN_FILE
        if (f !== fileFilter) return false
      }
      if (q && !`${e.key} ${e.source} ${e.target}`.toLowerCase().includes(q)) return false
      if (filter === 'empty') return !e.target.trim()
      // §9: только строка с живой находкой валидатора; фикс → revalidate →
      // строка исчезает. Сопоставление в uid-пространстве (review P0-2):
      // находка с полной идентичностью адресует СВОЮ строку, а не все
      // строки с тем же display-key.
      if (filter === 'issues') return issueKeys.has(e.uid)
      return true
    })
  }, [st.entries, search, filter, kindFilter, fileFilter, issueKeys])

  // UI-14 (owner audit Oct-14): the editor must never sit on an entry the
  // current filters hide. The stored selection wins ONLY while it stays
  // visible; otherwise the first visible row is selected instead — the
  // stored key is NOT overwritten, so clearing the filters restores it.
  const storedSelectedVisible =
    st.selectedKey != null && visible.some((e) => e.uid === st.selectedKey)
  const selectedKey =
    storedSelectedVisible && st.selectedKey ? st.selectedKey : (visible[0]?.uid ?? null)
  const selected = st.entries.find((e) => e.uid === selectedKey) ?? null

  // §17: первая находка для ТЕКУЩЕЙ строки (порядок отчёта валидатора).
  const selectedFindings = useMemo(
    () => (selected ? findingsByEntryKey(vstate.report).get(selected.uid) ?? [] : []),
    [selected, vstate.report],
  )

  // §14: сиблинги текущей записи того же Def — тот же kind/def_type, ключ
  // `<DefName>.<поле>`. Детерминированно: сортировка по ключу, без
  // хардкода модов. Нет def_type или сиблингов — блока нет.
  const related = useMemo(() => {
    if (!selected || !selected.identity.def_type) return []
    const dot = selected.key.indexOf('.')
    if (dot <= 0) return [] // ключ не формы <DefName>.<field> — группировки нет
    const prefix = selected.key.slice(0, dot + 1)
    return st.entries
      .filter(
        (e) =>
          e.key !== selected.key &&
          e.identity.kind === selected.identity.kind &&
          e.identity.def_type === selected.identity.def_type &&
          e.key.startsWith(prefix),
      )
      .sort((a, b) => a.key.localeCompare(b.key))
  }, [selected, st.entries])

  const snap = snapNow
  const projectName = snap ? st.summaries.find((s) => s.project_id === snap.project_id)?.name ?? snap.project_id : ''

  if (!snap) {
    return (
      <div className="page-content narrow-page">
        <p className="page-note">{t('ws.noSnapshot')}</p>
        <button onClick={onBack}>{t('ws.backHome')}</button>
      </div>
    )
  }

  return (
    <div className="ws-root" data-testid="ws.root">
      <div className="ws-error" role="alert" hidden={!st.lastError}>
        {st.lastError}
      </div>
      {/* Data Safety: честный отчёт восстановления черновиков после open()
          (подняты / сирота от другой ревизии / хранилище недоступно).
          Inline, dismissable — никогда не молчаливое применение. */}
      {st.draftsNotice && (
        <div
          className={`inline-warning ${st.draftsNotice.kind === 'unavailable' ? 'text-destructive' : ''}`}
          role={st.draftsNotice.kind === 'restored' ? 'status' : 'alert'}
          data-testid="ws.drafts-notice"
        >
          <CircleAlert size={15} />
          <span>
            {st.draftsNotice.kind === 'restored' && (
              <>
                {t('ws.drafts.restored', { count: st.draftsNotice.count ?? 0 })}
                {st.draftsNotice.fromBackup ? ` ${t('ws.drafts.restoredFromBackup')}` : ''}
                {/* Extra honesty from the restore report (review P0-1/P0-2):
                    drafts held in OTHER target locales, legacy v1 keys that
                    were re-anchored, or the in-memory guard of a previous
                    failed autosave. */}
                {st.draftsNotice.detail ? ` — ${st.draftsNotice.detail}` : ''}
              </>
            )}
            {st.draftsNotice.kind === 'orphan' &&
              t('ws.drafts.orphan', { path: st.draftsNotice.path ?? '—' })}
            {st.draftsNotice.kind === 'unavailable' &&
              t('ws.drafts.unavailable', { detail: st.draftsNotice.detail ?? '—' })}
          </span>
          <button
            className="icon-btn"
            aria-label={t('ws.drafts.dismiss')}
            data-testid="ws.drafts-notice-dismiss"
            onClick={() => projectStore.dismissDraftsNotice()}
          >
            ×
          </button>
        </div>
      )}
      <div className="layout-switch" style={{ margin: '0 28px 10px' }} role="tablist" data-testid="ws.mode-switch">
        {(
          [
            ['studio', t('ws.mode.studio')],
            ['focus', t('ws.mode.focus')],
            ['overview', t('ws.mode.overview')],
          ] as const
        ).map(([m, label]) => (
          <button
            key={m}
            role="tab"
            aria-selected={wsMode === m}
            className={wsMode === m ? 'active' : ''}
            onClick={() => switchMode(m)}
            data-testid={`ws.mode.${m}`}
          >
            {label}
          </button>
        ))}
      </div>
      <div className={`ws-panes ws-mode-${wsMode}${!treeOpen ? ' tree-collapsed' : ''}${!inspectorOpen ? ' inspector-collapsed' : ''}`}>
        <Group orientation="horizontal" id="rimloc-ws-panes" defaultLayout={defaultLayout} onLayoutChanged={onLayoutChanged}>
        <Panel id="ws-left" minSize={30} defaultSize={66}>
        <div className="ws-center">
        <aside className="file-tree" data-testid="ws.tree">
          <div className="pane-title">
            <span>{t('ws.treeTitle')}</span>
            <button className="icon-btn" aria-label={t('ws.toggleTree')} data-testid="ws.tree-toggle" onClick={toggleTree}>
              <ChevronDown />
            </button>
          </div>
          <div className="tree-views" role="tablist" style={{ display: 'flex', gap: 4, padding: '0 6px 4px' }}>
            <button
              role="tab"
              aria-selected={treeView === 'type'}
              className={`tree-view-tab ${treeView === 'type' ? 'active' : ''}`}
              onClick={() => setTreeView('type')}
              data-testid="ws.tree.view-type"
            >
              {t('ws.byType')}
            </button>
            <button
              role="tab"
              aria-selected={treeView === 'file'}
              className={`tree-view-tab ${treeView === 'file' ? 'active' : ''}`}
              onClick={() => setTreeView('file')}
              data-testid="ws.tree.view-file"
            >
              {t('ws.byFile')}
            </button>
          </div>
          <button
            className={`tree-root ${kindFilter === 'all' && fileFilter === 'all' ? 'active' : ''}`}
            onClick={() => {
              setKindFilter('all')
              setFileFilter('all')
            }}
          >
            <FolderOpen /> {t('ws.allStrings')} <span>{st.entries.length}</span>
          </button>
          {treeOpen &&
            treeView === 'type' &&
            kindGroups.map(([k, n]) => (
              <button
                key={k}
                className={`file-item ${kindFilter === k ? 'active' : ''}`}
                onClick={() => {
                  setKindFilter(kindFilter === k ? 'all' : k)
                }}
                data-testid={`ws.tree.${k}`}
              >
                <FileCode2 />
                <span>{k}</span>
                <small>{n}</small>
              </button>
            ))}
          {treeOpen &&
            treeView === 'file' &&
            fileDirGroups.map(([dir, files]) => (
              <div key={dir}>
                {dir !== '·' && (
                  <div className="tree-group">
                    <FolderOpen /> <span>{dir}</span>
                  </div>
                )}
                {files.map(({ file, n }) => (
                  <button
                    key={file}
                    className={`file-item ${fileFilter === file ? 'active' : ''}`}
                    onClick={() => {
                      setFileFilter(fileFilter === file ? 'all' : file)
                    }}
                    data-testid={`ws.tree.file.${file}`}
                    title={file === UNKNOWN_FILE ? t('ws.unknownFile') : file}
                  >
                    <FileCode2 />
                    <span>
                      {file === UNKNOWN_FILE
                        ? t('ws.unknownFile')
                        : dir === '·'
                          ? file
                          : file.slice(dir.length + 1)}
                    </span>
                    <small>{n}</small>
                  </button>
                ))}
              </div>
            ))}
          <div className="tree-bottom">
            <span className="eyebrow">{t('ws.translationFolder')}</span>
            <code>Languages/{folderForm(st.targetLocale)}</code>
            <span className="text-success">{t('ws.sourcesSafe')}</span>
          </div>
        </aside>
        {/* CENTER — inventory */}
        <section className="entries-pane" data-testid="ws.entries">
          <div className="entry-toolbar">
            <div className="search-field">
              <Search size={16} />
              <input
                aria-label={t('ws.search')}
                placeholder={t('ws.searchPlaceholder')}
                value={searchLive}
                onChange={(e) => setSearchLive(e.target.value)}
              />
            </div>
            <button
              className="icon-btn"
              aria-label={t('ws.clearAllFilters')}
              title={t('ws.clearAllFilters')}
              onClick={() => {
                setSearchLive('')
                setSearch('')
                setFilter('all')
                setKindFilter('all')
                setFileFilter('all')
              }}
              data-testid="ws.clear-filters"
            >
              <ListFilter />
            </button>
            <button
              className="icon-btn"
              aria-label={t('ws.toggleInspector')}
              title={t('ws.toggleInspector')}
              data-testid="ws.inspector-toggle"
              onClick={toggleInspector}
            >
              <PanelRight size={14} />
            </button>
            {!treeOpen && (
              <button
                className="icon-btn"
                aria-label={t('ws.toggleTree')}
                title={t('ws.toggleTree')}
                data-testid="ws.tree-reopen"
                onClick={toggleTree}
              >
                <FolderOpen size={14} />
              </button>
            )}
          </div>
          <div className="entry-filters">
            {(['all', 'empty', 'issues'] as const).map((f) => (
              <button
                key={f}
                className={filter === f ? 'active' : ''}
                onClick={() => setFilter(f)}
                data-testid={`ws.filter.${f}`}
              >
                {t(`ws.filter.${f}`)}
              </button>
            ))}
            <span>{visible.length} {t('ws.rows')}</span>
            {/* Data Safety: статус durable-автосохранения черновиков.
                Error + Retry — провал записи НЕ теряет работу (карта живёт
                в памяти), но и не молчит: честный статус и повтор. */}
            <DraftsSyncIndicator status={st.draftsSync.status} error={st.draftsSync.error} />
          </div>
          <EntryList
            entries={visible}
            selectedKey={selectedKey}
            findings={findingsByKey}
            sourceLabel={folderForm(st.sourceLocale)}
            targetLabel={folderForm(st.targetLocale)}
            onSelect={(k) => projectStore.select(k)}
          />
          <SourceSearchPanel snap={snap} />
          <div className="list-footer">
            <span>
              {visible.length} {t('ws.of')} {st.entries.length} {t('ws.rows')}
            </span>
            <span>
              <span className="dot success" /> {t('ws.sessionChanges')}
            </span>
          </div>
        </section>
        </div>
        </Panel>
        <Separator className="ws-handle" aria-label={t('ws.resize')} />
        <Panel id="ws-right" minSize={22} defaultSize={34}>
        {/* RIGHT — editor */}
        {selected && (
          <EntryEditor
            key={selected.uid}
            entryUid={selected.uid}
            labelKey={
              duplicatedDisplayKeys(st.entries).has(selected.key) && selected.identity.def_type
                ? `${selected.key} · ${selected.identity.def_type}`
                : selected.key
            }
            source={selected.source}
            target={selected.target}
            draft={st.drafts[selected.uid] ?? selected.target}
            busy={st.busy}
            sourceFile={selected.sourceFile}
            sourceLine={selected.sourceLine}
            selectedBy={selected.selectedBy}
            provenance={selected.provenance}
            tkey={selected.tkey}
            sourceRoot={snap.source_root?.path}
            sourceLabel={folderForm(st.sourceLocale)}
            targetLabel={folderForm(st.targetLocale)}
            // §17: первая находка валидатора для этой строки (+ хвост).
            firstFinding={selectedFindings[0]}
            extraFindings={Math.max(0, selectedFindings.length - 1)}
            // §13: термины глоссария (кэш-стор) + честный статус загрузки.
            terms={glState.projectId === snap.project_id ? glState.terms : []}
            termsBusy={glState.busy}
            termsError={glState.error}
            // §14: сиблинги того же Def.
            related={related}
            // §15/§20: AI entry — только навигация, без вызовов провайдера.
            onSendToChatBatch={() => sendToChatBatch(selected.identity)}
            onLoadXmlPreview={loadXmlPreview}
            entryKey={selected.key}
            entryDefType={selected.identity.def_type ?? undefined}
            targetLocale={folderForm(st.targetLocale)}
            onLoadOutputPreview={loadOutputPreview}
            outputPreview={outputPreview}
            projectRevision={snapNow?.revision}
            onToggleInspector={toggleInspector}
            onDraft={(v) => projectStore.setDraft(selected.uid, v)}
            onCommit={(next) => void projectStore.commit(selected.uid, next)}
            onMove={(d) => {
              const idx = st.entries.findIndex((e) => e.uid === selected.uid)
              const nx = st.entries[idx + d]
              if (nx) projectStore.select(nx.uid)
            }}
          />
        )}
        </Panel>
        </Group>
      </div>
      <div className="ws-footer">
        <button onClick={onBack}>{t('ws.backHome')}</button>
        <span>
          {projectName} · rev {snap.revision} · epoch {snap.session_epoch}
          {snap.dirty ? ` · ${t('ws.dirty')}` : ''}
        </span>
      </div>
    </div>
  )
}

/** Data Safety: компактный индикатор конвейера durable-автосохранения
 *  черновиков. idle молчит (чистое состояние — шума нет); error показывает
 *  честную ошибку и Retry (повтор записи в sidecar-хранилище). */
function DraftsSyncIndicator({
  status,
  error,
}: {
  status: DraftsSyncStatus
  error: string | null
}) {
  if (status === 'idle') return null
  if (status === 'error') {
    return (
      <span className="status-label text-destructive" data-testid="ws.drafts-sync" title={error ?? undefined}>
        <CircleAlert size={13} />
        {t('ws.drafts.saveError')}
        <button
          className="icon-btn"
          aria-label={t('ws.drafts.retry')}
          title={t('ws.drafts.retry')}
          data-testid="ws.drafts-retry"
          onClick={() => projectStore.retryDraftsSync()}
        >
          <RotateCcw size={13} />
        </button>
      </span>
    )
  }
  const label =
    status === 'saving'
      ? t('ws.drafts.saving')
      : status === 'saved'
        ? t('ws.drafts.saved')
        : t('ws.drafts.dirty')
  return (
    <span className="status-label" data-testid="ws.drafts-sync">
      <Save size={13} />
      {label}
    </span>
  )
}

/** D-V1: compact status indicator of the canon third column (22px).
 *  Priority: a live validator finding (color + title by severity) → an
 *  empty target («нет перевода») → translated. Everything comes from the
 *  live validator report and the snapshot — nothing is synthesized. */
function RowStatus({
  finding,
  target,
}: {
  finding?: ValidationFindingDto
  target: string
}) {
  const empty = !target.trim()
  const cls = finding
    ? finding.severity === 'error'
      ? 'text-destructive'
      : finding.severity === 'warning'
        ? 'text-warning'
        : 'text-info'
    : empty
      ? 'text-warning'
      : 'text-muted-foreground'
  const Icon = finding ? CircleAlert : empty ? Clock3 : Circle
  const title = finding ? finding.message : empty ? t('ws.status.empty') : t('ws.status.translated')
  return (
    <span className={`status-label ${cls}`} title={title} data-testid="ws.entry-status">
      <Icon size={13} />
    </span>
  )
}

function EntryList({
  entries,
  selectedKey,
  findings,
  sourceLabel,
  targetLabel,
  onSelect,
}: {
  entries: WorkspaceEntry[]
  selectedKey: string | null
  findings: Map<string, ValidationFindingDto[]>
  sourceLabel: string
  targetLabel: string
  onSelect: (key: string) => void
}) {
  const parentRef = useRef<HTMLDivElement>(null)
  // UI-04: the row size follows the persisted density (comfortable 63 /
  // compact 40); measure() re-locks the virtualizer when it changes.
  const rowHeight = readUiDensity() === 'compact' ? COMPACT_ROW_HEIGHT : ROW_HEIGHT
  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    overscan: 12,
  })
  useEffect(() => {
    virtualizer.measure()
    // virtualizer is stable per @tanstack/react-virtual contract
  }, [rowHeight, virtualizer])
  // UI-08 (owner audit Oct-14): after a filter change / keyboard move the
  // selected row must be scrolled INTO the viewport, not hidden above or
  // below the fold. Index-based — fires on selection change only.
  const selectedIndex = selectedKey ? entries.findIndex((e) => e.uid === selectedKey) : -1
  // UI-08 carry-over (review Oct-15): the effect keys on the selected
  // IDENTITY — a different entry landing at the same index must scroll
  // into view just the same.
  useEffect(() => {
    if (selectedIndex >= 0) {
      virtualizer.scrollToIndex(selectedIndex, { align: 'auto' })
    }
    // virtualizer is stable per @tanstack/react-virtual contract
  }, [selectedIndex, selectedKey])

  return (
    <>
      {/* D-V1: canon table head (ИСХОДНИК/ПЕРЕВОД/СТАТУС) — hidden on
          narrow panes by CSS, matching the ref layout rule. */}
      <div className="table-head" data-testid="ws.table-head">
        <span>
          {t('ws.headSource')} <small>{sourceLabel}</small>
        </span>
        <span>
          {t('ws.headTarget')} <small>{targetLabel}</small>
        </span>
        <span>{t('ws.headStatus')}</span>
      </div>
      <div className="entry-list" ref={parentRef} style={{ overflowY: 'auto', flex: 1, minHeight: 0 }}>
        <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
          {virtualizer.getVirtualItems().map((vi) => {
            const e = entries[vi.index]!
            // Full-identity key-space (review P0-2): selection and findings
            // match by uid; the LABEL still shows the display key, with the
            // def-type discriminator when another Def type shares it.
            const label =
              duplicatedDisplayKeys(entries).has(e.key) && e.identity.def_type
                ? `${e.key} · ${e.identity.def_type}`
                : e.key
            return (
              <button
                key={e.uid}
                className={`entry-row ${selectedKey === e.uid ? 'selected' : ''}`}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: rowHeight,
                  transform: `translateY(${vi.start}px)`,
                  ...(selectedKey === e.uid ? { backgroundColor: 'var(--accent)' } : {}),
                }}
                onClick={() => onSelect(e.uid)}
                data-testid={`ws.entry.${vi.index}`}
              >
                <div className="row-source">
                  <span>{e.source}</span>
                  <code>{label}</code>
                </div>
                <div className={`row-target ${!e.target ? 'untranslated' : ''}`}>{e.target || t('ws.addTarget')}</div>
                <RowStatus finding={findings.get(e.uid)?.[0]} target={e.target} />
              </button>
            )
          })}
        </div>
      </div>
    </>
  )
}

function EntryEditor(props: {
  /** Full-identity key (store space): drafts address THIS. */
  entryUid: string
  /** Display key for humans (row/code label + clipboard copy), with the
   *  def-type discriminator when another Def type shares the key. */
  labelKey: string
  source: string
  target: string
  draft: string
  busy: boolean
  sourceFile?: string
  sourceLine?: number
  selectedBy?: string
  // Source Inspector (live): view-selection facts + TKey locations.
  provenance?: WorkspaceEntry['provenance']
  tkey?: WorkspaceEntry['tkey']
  /** Read-only mod root the snapshot was built from — anchors the
   *  project-relative source file (M-7: honest unknown, no placeholders). */
  sourceRoot?: string
  // Локали-бейджи из состояния проекта (audit v2 #3): folder-форма реестра
  // вместо жёстких EN/RU-литералов.
  sourceLabel: string
  targetLabel: string
  // §17: первая находка валидатора для этой строки (null = чисто).
  firstFinding?: ValidationFindingDto
  extraFindings: number
  // §13: термины глоссария проекта (кэш-стор) и статус его загрузки.
  terms: GlossaryTermDto[]
  termsBusy: boolean
  termsError: string | null
  // §14: сиблинги того же Def (пусто = блока нет).
  related: WorkspaceEntry[]
  // §15/§20: перенос строки в чат-перевод (sessionStorage + навигация).
  onSendToChatBatch: () => void
  // rel27: загрузить XML-предпросмотр исходного файла строки.
  onLoadXmlPreview?: (file: string, line?: number) => Promise<SourceXmlPreviewDto>
  /** UI-05: entry display key (e.g. `Apple.label`) used to locate the
   *  canonical output element; target locale of the workspace. */
  entryKey?: string
  entryDefType?: string
  targetLocale?: string
  onLoadOutputPreview?: (locale: string) => void
  onToggleInspector?: () => void
  /** Current session revision — the output preview binds its displayed
   *  result to it and flags itself stale on mismatch (P1-C). */
  projectRevision?: number
  outputPreview?:
    | { status: 'idle' | 'loading' | 'error'; error?: string }
    | {
        status: 'done'
        locale: string
        revision: number
        files: { path: string; xml: string }[]
        truncated: boolean
      }
    | null
  onDraft: (v: string) => void
  onCommit: (next: boolean) => void
  onMove: (d: number) => void
}) {
  // D-V5: the copy affordance answers honestly — Check replaces Copy for
  // 1.5s after a SUCCESSFUL write; a failed clipboard stays silent (no
  // fabricated success), same contract as the ref's toast.
  const [copied, setCopied] = useState(false)
  const copyTimer = useRef<number | null>(null)
  const copyKey = (): void => {
    void navigator.clipboard.writeText(props.labelKey).then(
      () => {
        setCopied(true)
        if (copyTimer.current !== null) window.clearTimeout(copyTimer.current)
        copyTimer.current = window.setTimeout(() => setCopied(false), 1500)
      },
      () => {
        /* clipboard unavailable — honest silence */
      },
    )
  }
  // §13: единственная мёртвая кнопка-таб стала настоящими табами.
  // rel27: третья вкладка — настоящий XML-предпросмотр исходного файла.
  const [tab, setTab] = useState<'source' | 'terms' | 'xml' | 'output'>('source')
  const [xmlPreview, setXmlPreview] = useState<
    | { status: 'idle' | 'loading' | 'error'; error?: string }
    | {
        status: 'done'
        lines: SourceXmlPreviewLineDto[]
        target?: number
        file: string
        total: number
        truncated_head: boolean
        truncated_tail: boolean
      }
    | null
  >(null)
  const loadXmlPreview = (): void => {
    if (!props.onLoadXmlPreview) return
    setXmlPreview({ status: 'loading' })
    props.onLoadXmlPreview(props.sourceFile ?? '', props.sourceLine)
      .then((res) =>
        setXmlPreview({
          status: 'done',
          lines: res.lines,
          target: res.target_line,
          file: res.file,
          total: res.total_lines,
          truncated_head: res.truncated_head,
          truncated_tail: res.truncated_tail
        })
      )
      .catch((e: unknown) => setXmlPreview({ status: 'error', error: e instanceof Error ? e.message : String(e) }))
  }
  const dirty = props.draft !== props.target
  const patchStage = props.provenance?.patch_stage
  const tkeyLocs = props.tkey?.locations ?? []
  // Document order: the LAST location is the effective one (last field
  // assignment wins); earlier entries are other usages.
  const tkeyPrimary = tkeyLocs.length > 0 ? tkeyLocs[tkeyLocs.length - 1] : undefined
  const tkeyOther = tkeyLocs.slice(0, -1)
  const fmtLoc = (l: { file: string; line?: number }) => (l.line != null ? `${l.file}:${l.line}` : l.file)
  // §13: термины, чьё source-имя входит в исходник строки (без учёта
  // регистра). Порядок — по первой позиции вхождения, детерминированно.
  const termMatches = useMemo(() => {
    const src = props.source.toLowerCase()
    if (!src) return []
    return props.terms
      .filter((tm) => tm.term.trim() !== '' && src.includes(tm.term.toLowerCase()))
      .map((tm) => ({ tm, at: src.indexOf(tm.term.toLowerCase()) }))
      .sort((a, b) => a.at - b.at || a.tm.term.localeCompare(b.tm.term))
      .map((m) => m.tm)
  }, [props.source, props.terms])
  return (
    <aside className="detail-pane" data-testid="ws.editor">
      <div className="detail-header">
        <span>{t('ws.editorTitle')}</span>
        <div>
          <button className="icon-btn" aria-label={t('ws.prev')} onClick={() => props.onMove(-1)}>
            <ArrowUp />
          </button>
          <button className="icon-btn" aria-label={t('ws.next')} onClick={() => props.onMove(1)}>
            <ArrowDown />
          </button>
          <button
            className="icon-btn"
            aria-label={t('ws.toggleInspector')}
            title={t('ws.toggleInspector')}
            data-testid="ws.inspector-close"
            onClick={props.onToggleInspector}
          >
            <PanelRight size={13} />
          </button>
        </div>
      </div>
      <div className="detail-body">
        <div className="detail-key">
          <code>{props.labelKey}</code>
          <button
            className="icon-btn"
            aria-label={t('ws.copyKey')}
            title={copied ? t('ws.copied') : t('ws.copyKey')}
            data-testid="ws.copy-key"
            onClick={copyKey}
          >
            {copied ? <Check /> : <Copy />}
          </button>
        </div>
        <div className="label-line">
          <span>{t('ws.source')}</span>
          <span className="locale">{props.sourceLabel}</span>
        </div>
        <div className="source-block">{props.source}</div>
        <div className="label-line">
          <label htmlFor="translation">{t('ws.translation')}</label>
          <span className="locale">{props.targetLabel}</span>
        </div>
        <textarea
          id="translation"
          value={props.draft}
          onChange={(e) => props.onDraft(e.target.value)}
          placeholder={t('ws.translationPlaceholder')}
          spellCheck={false}
          data-testid="ws.editor-textarea"
        />
        <div className="char-count">
          <span>{dirty ? t('ws.unsaved') : t('ws.saved')}</span>
          <span>{props.draft.length} {t('ws.chars')}</span>
        </div>
        {dirty && props.draft.trim() === '' && (
          <div className="inline-warning">
            <Braces size={15} />
            <span>{t('ws.emptyWarning')}</span>
          </div>
        )}
        {/* §17: первая находка живого валидатора для этой строки. Честно
            из отчёта project_validate — ничего не эмулируется. */}
        {props.firstFinding && (
          <div className="inline-warning" role="status" data-testid="ws.editor-finding">
            <CircleAlert size={15} />
            <span>
              {tEnum('ws.sev', props.firstFinding.severity)}: {props.firstFinding.message}
              {props.extraFindings > 0 ? ` · ${t('ws.finding.more', { count: props.extraFindings })}` : ''}
            </span>
          </div>
        )}
        <div className="detail-actions">
          <button className="btn-primary" disabled={props.busy || !dirty} data-testid="ws.editor-save-next" onClick={() => props.onCommit(true)}>
            <Check /> {t('ws.saveAndNext')}
          </button>
          {/* §15: контекстный AI entry — только перенос строки в чат-перевод
              (sessionStorage + навигация); никаких вызовов провайдера. */}
          <button
            className="icon-btn"
            style={{ width: 'auto', padding: '0 9px', fontSize: 10 }}
            aria-label={t('ws.toChatbatchHint')}
            title={t('ws.toChatbatchHint')}
            data-testid="ws.editor-to-chatbatch"
            onClick={props.onSendToChatBatch}
          >
            <MessagesSquare size={13} /> {t('ws.toChatbatch')}
          </button>
          <button className="icon-btn" aria-label={t('ws.save')} disabled={props.busy || !dirty} data-testid="ws.editor-save" onClick={() => props.onCommit(false)}>
            <Save />
          </button>
          <button
            className="icon-btn"
            aria-label={t('ws.revert')}
            disabled={props.busy}
            // Revert = «чистый черновик = отсутствие записи» (§11): удаляем
            // ключ из drafts, а не пишем committed-текст как черновик —
            // иначе пилюля «Есть несохранённые» считает фантом.
            onClick={() => projectStore.clearDraft(props.entryUid)}
          >
            <RotateCcw />
          </button>
        </div>
        {/* §20: полоса «Машинный перевод» — строка-действие в тот же
            no-API flow (чат-перевод). Только shortcut: выбор транспорта
            всегда за человеком. */}
        <button
          className="icon-btn"
          style={{ width: '100%', justifyContent: 'space-between', marginTop: 8, color: 'var(--muted-foreground)' }}
          aria-label={`${t('ws.mtTitle')} — ${t('ws.mtHint')}`}
          title={`${t('ws.mtTitle')} — ${t('ws.mtHint')}`}
          data-testid="ws.editor-mt"
          onClick={props.onSendToChatBatch}
        >
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Sparkles size={13} /> {t('ws.mtTitle')}
          </span>
          <span style={{ fontSize: 10 }}>{t('ws.mtHint')}</span>
        </button>
        {/* SOURCE block (Source Inspector, live): every row is backed by the
            contract snapshot — file+line (source_ref), winner reason
            (selected_by), view-selection facts (provenance) and the TKey
            primary + other-usages locations. Absent data renders an honest
            '—', never a fabricated value. Табы настоящие: Источник | Термины. */}
        <div className="context-tabs" role="tablist" data-testid="ws.context-tabs">
          <button
            role="tab"
            aria-selected={tab === 'source'}
            className={tab === 'source' ? 'active' : ''}
            data-testid="ws.tab.source"
            onClick={() => setTab('source')}
          >
            {t('ws.tabSource')}
          </button>
          <button
            role="tab"
            aria-selected={tab === 'terms'}
            className={tab === 'terms' ? 'active' : ''}
            data-testid="ws.tab.terms"
            onClick={() => setTab('terms')}
          >
            <BookOpen size={12} /> {t('ws.tabTerms')}
          </button>
          <button
            role="tab"
            aria-selected={tab === 'xml'}
            className={tab === 'xml' ? 'active' : ''}
            data-testid="ws.tab.xml"
            onClick={() => {
              setTab('xml')
              if (props.sourceFile) loadXmlPreview()
            }}
          >
            <Braces size={12} /> {t('ws.tabXml')}
          </button>
          {/* UI-05: the CANONICAL OUTPUT xml for the current locale — the
              exact bytes an export writes, rendered in-memory. */}
          <button
            role="tab"
            aria-selected={tab === 'output'}
            className={tab === 'output' ? 'active' : ''}
            data-testid="ws.tab.output"
            onClick={() => {
              setTab('output')
              if (props.targetLocale) props.onLoadOutputPreview?.(props.targetLocale)
            }}
          >
            <Braces size={12} /> {t('ws.tabOutput')}
          </button>
        </div>
        {tab === 'source' ? (
          <div className="context-content" role="tabpanel" data-testid="src.block">
            <div data-testid="src.file">
              <span>{t('ws.file')}</span>
              <code title={props.sourceFile}>{props.sourceFile ?? '—'}</code>
            </div>
            <div data-testid="src.line">
              <span>{t('ws.line')}</span>
              <strong>{props.sourceLine ?? '—'}</strong>
            </div>
            <div data-testid="src.selected-by">
              <span>{t('ws.whyThisSource')}</span>
              <strong>{tEnum('ws.why', props.selectedBy)}</strong>
            </div>
            {props.provenance?.version_selected && (
              <div data-testid="src.version">
                <span>{t('ws.src.version')}</span>
                <strong>{props.provenance.version_selected}</strong>
              </div>
            )}
            {props.provenance?.conditional_branch && (
              <div data-testid="src.conditional">
                <span>{t('ws.src.conditional')}</span>
                <strong>{t('ws.src.conditionalValue')}</strong>
              </div>
            )}
            {patchStage && patchStage !== 'none' && (
              <div data-testid="src.patch">
                <span>{t('ws.src.patch')}</span>
                <strong>{tEnum('ws.src.patchStage', patchStage)}</strong>
              </div>
            )}
            {props.tkey && (
              <div className="related-string" data-testid="src.tkey">
                <span>{t('ws.src.tkey')}</span>
                <div className="src-tkey-meta" data-testid="src.tkey-meta">
                  <code>{props.tkey.def_type}</code>
                  <span>{props.tkey.strategy}</span>
                </div>
                {tkeyPrimary && (
                  <div data-testid="src.tkey-primary">
                    <span>{t('ws.src.tkeyPrimary')}</span>
                    <code title={fmtLoc(tkeyPrimary)}>{fmtLoc(tkeyPrimary)}</code>
                  </div>
                )}
                {tkeyOther.length > 0 && (
                  <div data-testid="src.tkey-other">
                    <span>{t('ws.src.tkeyOther', { count: tkeyOther.length })}</span>
                    {tkeyOther.map((loc, i) => (
                      <code key={`${loc.file}:${loc.line ?? 'x'}:${i}`} title={fmtLoc(loc)}>
                        {fmtLoc(loc)}
                      </code>
                    ))}
                  </div>
                )}
              </div>
            )}
            {props.sourceRoot && (
              <div data-testid="src.root">
                <span>{t('ws.src.root')}</span>
                <code title={props.sourceRoot}>{props.sourceRoot}</code>
              </div>
            )}
          </div>
        ) : tab === 'terms' ? (
          <div className="context-content" role="tabpanel" data-testid="ws.terms-block">
            {props.termsError && (
              <div className="inline-warning" role="alert" data-testid="ws.terms-error">
                <CircleAlert size={15} />
                <span>{props.termsError}</span>
              </div>
            )}
            {!props.termsError && (
              <>
                <div data-testid="ws.terms-note">
                  <span>{t('ws.tabTerms.note')}</span>
                </div>
                {termMatches.map((tm) => (
                  <div key={tm.id} data-testid="ws.terms-match">
                    <span>
                      <strong>{tm.term}</strong>
                      {tm.note ? <small>{` · ${tm.note}`}</small> : null}
                    </span>
                    <strong>{tm.translation}</strong>
                  </div>
                ))}
                {termMatches.length === 0 && (
                  <div data-testid="ws.terms-empty">
                    <span>
                      {props.termsBusy && props.terms.length === 0
                        ? t('ws.tabTerms.loading')
                        : t('ws.tabTerms.empty')}
                    </span>
                  </div>
                )}
              </>
            )}
          </div>
        ) : tab === 'output' ? (
          <div className="context-content" role="tabpanel" data-testid="ws.output-block">
            {props.outputPreview?.status === 'loading' && (
              <p className="page-note" data-testid="ws.output-loading">{t('about.loading')}</p>
            )}
            {props.outputPreview?.status === 'error' && (
              <div className="inline-warning" role="alert" data-testid="ws.output-error">
                <CircleAlert size={15} />
                <span>{props.outputPreview.error}</span>
              </div>
            )}
            {props.outputPreview?.status === 'done' &&
              props.projectRevision !== undefined &&
              props.outputPreview.revision !== props.projectRevision && (
                <div className="inline-warning" role="alert" data-testid="ws.output-stale">
                  {t('ws.output.stale')}
                </div>
              )}
            {props.outputPreview?.status === 'done' && (
              <>
                <div data-testid="ws.output.meta" style={{ fontSize: 10, color: 'var(--muted-foreground)', marginBottom: 4 }}>
                  {t('ws.output.canonical')} · {props.outputPreview.locale} · {props.outputPreview.files.length}{' '}
                  {t('ws.output.files')}
                  {props.outputPreview.truncated ? ` · ${t('ws.output.truncated')}` : ''}
                </div>
                {/* P1-C: suppress stale content — when the revision doesn't
                    match, show only the stale warning, not the XML. */}
                {props.projectRevision !== undefined &&
                  props.outputPreview.revision !== props.projectRevision && (
                    <div className="inline-warning" role="alert" data-testid="ws.output-stale">
                      {t('ws.output.stale')}
                    </div>
                  )}
                {props.projectRevision === undefined ||
                  props.outputPreview.revision === props.projectRevision ? (
                  (() => {
                    const elementTag = props.entryKey ? `<${props.entryKey}>` : null
                    const dirTag = props.entryDefType ? `DefInjected/${props.entryDefType}/` : null
                    const hit = props.outputPreview.files.find(
                      (f) =>
                        (!elementTag || f.xml.includes(elementTag)) &&
                        (!dirTag || f.path.includes(dirTag)),
                    )
                    if (!hit) {
                      return (
                        <p className="page-note" data-testid="ws.output-empty">
                          {t('ws.output.empty')}
                        </p>
                      )
                    }
                    return (
                      <>
                        <div data-testid="ws.output.file" style={{ fontSize: 10, color: 'var(--muted-foreground)', marginBottom: 4 }}>
                          <code>{hit.path}</code>
                        </div>
                        <pre data-testid="ws.output-pre" style={{ fontSize: 10, lineHeight: 1.45, overflowX: 'auto', margin: 0, whiteSpace: 'pre-wrap' }}>
                          {hit.xml}
                        </pre>
                      </>
                    )
                  })()
                ) : null}
              </>
            )}
          </div>
        ) : (
          <div className="context-content" role="tabpanel" data-testid="ws.xml-block">
            {!props.sourceFile ? (
              <div data-testid="ws.xml-nosource">
                <span style={{ fontSize: 10, color: 'var(--muted-foreground)' }}>
                  {t('ws.xml.noSource')}
                </span>
              </div>
            ) : xmlPreview?.status === 'error' ? (
              <div className="inline-warning text-destructive" role="alert" data-testid="ws.xml-error">
                <CircleAlert size={15} /> {t('ws.xml.error')}: {xmlPreview.error}
              </div>
            ) : xmlPreview?.status !== 'done' ? (
              <div data-testid="ws.xml-loading" style={{ fontSize: 10, color: 'var(--muted-foreground)' }}>
                {t('ws.xml.loading')}
              </div>
            ) : (
              <>
                <div data-testid="ws.xml.meta" style={{ fontSize: 10, color: 'var(--muted-foreground)', marginBottom: 4 }}>
                  {xmlPreview.file} · {xmlPreview.total} {t('ws.xml.lines')}
                  {(xmlPreview.truncated_head || xmlPreview.truncated_tail) ? ' · windowed' : ''}
                </div>
                <pre data-testid="ws.xml-pre" style={{ fontSize: 10, lineHeight: 1.45, overflowX: 'auto', margin: 0 }}>
                  {xmlPreview.lines.map((l) => (
                    <div
                      key={l.n}
                      style={{
                        display: 'flex',
                        gap: 8,
                        background: xmlPreview.target === l.n ? 'var(--accent)' : 'transparent',
                      }}
                    >
                      <span style={{ color: 'var(--muted-foreground)', userSelect: 'none', minWidth: 28, textAlign: 'right' }}>
                        {l.n}
                      </span>
                      <code style={{ whiteSpace: 'pre-wrap' }}>{l.text}</code>
                    </div>
                  ))}
                </pre>
              </>
            )}
          </div>
        )}
        {/* §14: Related — сиблинги того же Def (label ↔ description …).
            Переход честный: projectStore.select(key). */}
        {props.related.length > 0 && (
          <div className="related-string" data-testid="ws.related">
            <span>{t('ws.related')}</span>
            {props.related.map((r) => (
              <button
                key={r.uid}
                data-testid={`ws.related.${r.key}`}
                onClick={() => projectStore.select(r.uid)}
              >
                <code>{r.key}</code>
                <span>{r.source || '—'}</span>
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="detail-foot">
        <FileCode2 size={13} />
        <span>{props.sourceFile ?? ''}</span>
      </div>
    </aside>
  )
}

// Keep the client import referenced for the honest-bridge note in dev builds.
void clientInstance


/** Source Discovery (Stage A): read-only full-project source search,
 *  reconciled against the canonical inventory. Lives inside the workspace
 *  left pane; the workspace string search stays untouched. */
function SourceSearchPanel({ snap }: { snap: NonNullable<ReturnType<typeof useProjectState>['snapshot']> }) {
  const ss = useSourceSearch()
  const [open, setOpen] = useState(false)
  const epoch = snap.session_epoch
  const run = (): void => {
    void sourceSearchStore.run(snap.project_id, epoch)
  }
  const loadMore = (): void => {
    void sourceSearchStore.loadMore(snap.project_id, epoch)
  }
  const view = (s: SourceSearchState) => s

  return (
    <div style={{ margin: '8px 0' }}>
      {!open && (
        <button
          className="icon-btn"
          style={{ width: 'auto', padding: '0 9px', fontSize: 10 }}
          data-testid="ss.toggle"
          onClick={() => setOpen(true)}
        >
          <Search size={12} /> {t('ss.toggle')}
        </button>
      )}
      {open && (
        <div data-testid="ss.panel" style={{ border: '1px solid var(--border)', borderRadius: 8, padding: 8 }}>
          <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
            <strong style={{ fontSize: 11 }}>{t('ss.title')}</strong>
            <button
              className="icon-btn"
              style={{ marginLeft: 'auto' }}
              aria-label={t('ws.resetFilters')}
              onClick={() => {
                setOpen(false)
                sourceSearchStore.clear()
              }}
            >
              ×
            </button>
          </div>
          <p style={{ fontSize: 10, color: 'var(--muted-foreground)', margin: '4px 0' }}>
            {t('ss.hint')}
          </p>
          <div style={{ display: 'flex', gap: 6 }}>
            <input
              data-testid="ss.input"
              value={view(ss).query}
              placeholder={t('ss.placeholder')}
              onChange={(e) => sourceSearchStore.setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') run()
              }}
              style={{ flex: 1, fontSize: 12 }}
            />
            <button className="btn-primary" data-testid="ss.run" disabled={view(ss).status === 'running'} onClick={run}>
              {t('ss.run')}
            </button>
          </div>
          <label style={{ display: 'flex', gap: 4, alignItems: 'center', fontSize: 10, marginTop: 4 }}>
            <input
              type="checkbox"
              data-testid="ss.scope"
              checked={view(ss).scope === 'all_text'}
              onChange={(e) => sourceSearchStore.setScope(e.target.checked ? 'all_text' : 'standard')}
            />
            {t('ss.scope')}
          </label>
          {view(ss).status === 'error' && (
            <div className="inline-warning text-destructive" role="alert" data-testid="ss.error">
              {t('ss.error')}: {view(ss).error ?? '—'}
            </div>
          )}
          {view(ss).report && (
            <>
              <div data-testid="ss.results" style={{ fontSize: 10, margin: '6px 0' }}>
                {t('ss.results', { total: view(ss).report!.total_matches })}
              </div>
              {/* Stage B: classification filter — whole-tree counts on every
                  chip; toggling narrows the page to one honest class. */}
              <div
                data-testid="ss.filters"
                role="group"
                aria-label={t('ss.filterGroup')}
                style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 4 }}
              >
                {view(ss).report!.status_counts.map(([status, count]) => {
                  const active = view(ss).statusFilter.has(status)
                  return (
                    <button
                      key={status}
                      data-testid={`ss.filter.${status}`}
                      aria-pressed={active}
                      onClick={() => sourceSearchStore.toggleStatusFilter(status)}
                      className="status-label"
                      style={{
                        fontSize: 10,
                        padding: '1px 6px',
                        borderRadius: 999,
                        border: '1px solid var(--border)',
                        cursor: 'pointer',
                        background: active ? 'var(--accent)' : 'transparent',
                        color: active ? 'var(--accent-foreground)' : 'var(--muted-foreground)',
                      }}
                    >
                      {t(`ss.status.${status}`)} ({count})
                    </button>
                  )
                })}
                {view(ss).statusFilter.size > 0 && (
                  <button
                    data-testid="ss.filter.clear"
                    onClick={() => sourceSearchStore.clearStatusFilter()}
                    className="status-label"
                    style={{ fontSize: 10, padding: '1px 6px', cursor: 'pointer', border: 'none', background: 'transparent', textDecoration: 'underline' }}
                  >
                    {t('ss.filterAll')}
                  </button>
                )}
              </div>
              <div style={{ maxHeight: 260, overflowY: 'auto' }}>
                {visibleMatches(view(ss)).length === 0 ? (
                  <div data-testid="ss.filtered-empty" style={{ fontSize: 10, color: 'var(--muted-foreground)', padding: '4px 0' }}>
                    {t('ss.filterEmpty')}
                  </div>
                ) : (
                  visibleMatches(view(ss)).map((m, i) => (
                  <div key={`${m.file}:${m.line}:${i}`} data-testid={`ss.match.${i}`} style={{ borderBottom: '1px solid var(--border)', padding: '4px 0' }}>
                    <div style={{ display: 'flex', gap: 6, alignItems: 'baseline' }}>
                      <code style={{ fontSize: 10 }}>{m.file}:{m.line}</code>
                      <span className="status-label text-muted-foreground" style={{ fontSize: 10 }}>
                        {t(`ss.status.${m.status}`)}
                      </span>
                    </div>
                    <div style={{ fontSize: 11 }}>{m.line_text}</div>
                    {m.xml_context && (
                      <div style={{ fontSize: 10, color: 'var(--muted-foreground)' }}>{m.xml_context}</div>
                    )}
                    <div style={{ fontSize: 10, color: 'var(--muted-foreground)' }}>
                      {t('ss.detail')}: {m.detail}
                    </div>
                    {(m.status === 'suspected_extraction_omission' ||
                      m.status === 'unknown_eligibility') && (
                      <button
                        className="icon-btn"
                        style={{ fontSize: 10, padding: '1px 8px', marginTop: 2 }}
                        data-testid={`ss.include.${i}`}
                        disabled={
                          view(ss).includeBusyKey === `${m.file}:${m.line}` ||
                          view(ss).status === 'running'
                        }
                        title={t('ss.includeHint')}
                        onClick={() => {
                          void sourceSearchStore
                            .includeCandidate(
                              snap.project_id,
                              snap.session_epoch,
                              snap.acked_revision ?? snap.revision,
                              m.file,
                              m.line,
                              m.identity?.key ?? m.line_text,
                            )
                            .then(async (ok) => {
                              if (!ok) return
                              // The include moved the durable revision —
                              // adopt the fresh snapshot so the workspace
                              // list and the re-run search both see the
                              // recovered row.
                              await projectStore.adoptExternal()
                              run()
                            })
                        }}
                      >
                        {t('ss.include')}
                      </button>
                    )}
                  </div>
                  ))
                )}
                {view(ss).report!.truncated && (
                  <button className="icon-btn" style={{ width: '100%', marginTop: 4 }} data-testid="ss.more" onClick={loadMore}>
                    {t('ss.more')}
                  </button>
                )}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  )
}
