// Workspace (R1 REPRESENTATIVE SCREEN): LEFT project context / CENTER
// virtualized inventory / RIGHT editor. One real project state; the entry
// list is virtualized from day one (mandate §24/§27 — the prototype had
// neither panes nor virtualization).
import { useMemo, useRef, useState } from 'react'
import { Group, Panel, Separator, useDefaultLayout } from 'react-resizable-panels'
import { useVirtualizer } from '@tanstack/react-virtual'
import { ArrowDown, ArrowUp, Braces, Check, ChevronDown, Copy, FileCode2, FolderOpen, ListFilter, RotateCcw, Save, Search } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore, type WorkspaceEntry } from '../lib/state/project'
import { clientInstance } from '../lib/client/instance'
import { SOURCE_LOCALE } from '../lib/languages/registry'
import { folderForm } from '../lib/languages/folderForm'
import { t, tEnum } from '../lib/i18n'

const ROW_HEIGHT = 63

export function Workspace({ onBack }: { onBack: () => void }) {
  const st = useProjectState()
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState<'all' | 'empty' | 'issues'>('all')
  const [kindFilter, setKindFilter] = useState<string>('all')
  const [treeOpen, setTreeOpen] = useState(true)
  // react-resizable-panels v4: `autoSaveId` replaced by the useDefaultLayout
  // hook — same localStorage persistence for the pane split, new API shape.
  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: 'rimloc-ws-panes',
    storage: window.localStorage,
  })

  const kindGroups = useMemo(() => {
    const m = new Map<string, number>()
    for (const e of st.entries) {
      const k = e.identity.def_type ? `${e.identity.kind}/${e.identity.def_type}` : e.identity.kind
      m.set(k, (m.get(k) ?? 0) + 1)
    }
    return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  }, [st.entries])

  const visible = useMemo(() => {
    const q = search.trim().toLowerCase()
    return st.entries.filter((e) => {
      if (kindFilter !== 'all') {
        const k = e.identity.def_type ? `${e.identity.kind}/${e.identity.def_type}` : e.identity.kind
        if (k !== kindFilter) return false
      }
      if (q && !`${e.key} ${e.source} ${e.target}`.toLowerCase().includes(q)) return false
      if (filter === 'empty') return !e.target.trim()
      if (filter === 'issues') return e.completeness === 'todo' || e.target.includes('{') === false && e.source.includes('{')
      return true
    })
  }, [st.entries, search, filter, kindFilter])

  const selectedKey = st.selectedKey ?? visible[0]?.key ?? null
  const selected = st.entries.find((e) => e.key === selectedKey) ?? null

  const snap = st.snapshot
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
      <div className="ws-panes">
        <Group orientation="horizontal" id="rimloc-ws-panes" defaultLayout={defaultLayout} onLayoutChanged={onLayoutChanged}>
        <Panel id="ws-left" minSize={30} defaultSize={66}>
        <div className="ws-center">
        <aside className="file-tree" data-testid="ws.tree">
          <div className="pane-title">
            <span>{t('ws.treeTitle')}</span>
            <button className="icon-btn" aria-label={t('ws.toggleTree')} onClick={() => setTreeOpen(!treeOpen)}>
              <ChevronDown />
            </button>
          </div>
          <button className={`tree-root ${kindFilter === 'all' ? 'active' : ''}`} onClick={() => setKindFilter('all')}>
            <FolderOpen /> {t('ws.allStrings')} <span>{st.entries.length}</span>
          </button>
          {treeOpen &&
            kindGroups.map(([k, n]) => (
              <button
                key={k}
                className={`file-item ${kindFilter === k ? 'active' : ''}`}
                onClick={() => setKindFilter(kindFilter === k ? 'all' : k)}
                data-testid={`ws.tree.${k}`}
              >
                <FileCode2 />
                <span>{k}</span>
                <small>{n}</small>
              </button>
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
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
            <button
              className="icon-btn"
              aria-label={t('ws.resetFilters')}
              onClick={() => {
                setSearch('')
                setFilter('all')
              }}
            >
              <ListFilter />
            </button>
          </div>
          <div className="entry-filters">
            {(['all', 'empty', 'issues'] as const).map((f) => (
              <button key={f} className={filter === f ? 'active' : ''} onClick={() => setFilter(f)}>
                {t(`ws.filter.${f}`)}
              </button>
            ))}
            <span>{visible.length} {t('ws.rows')}</span>
          </div>
          <EntryList entries={visible} selectedKey={selectedKey} onSelect={(k) => projectStore.select(k)} />
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
            key={selected.key}
            entryKey={selected.key}
            source={selected.source}
            target={selected.target}
            draft={st.drafts[selected.key] ?? selected.target}
            busy={st.busy}
            sourceFile={selected.sourceFile}
            sourceLine={selected.sourceLine}
            selectedBy={selected.selectedBy}
            provenance={selected.provenance}
            tkey={selected.tkey}
            sourceRoot={snap.source_root?.path}
            sourceLabel={folderForm(SOURCE_LOCALE)}
            targetLabel={folderForm(st.targetLocale)}
            onDraft={(v) => projectStore.setDraft(selected.key, v)}
            onCommit={(next) => void projectStore.commit(selected.key, next)}
            onMove={(d) => {
              const idx = st.entries.findIndex((e) => e.key === selected.key)
              const nx = st.entries[idx + d]
              if (nx) projectStore.select(nx.key)
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

function EntryList({
  entries,
  selectedKey,
  onSelect,
}: {
  entries: { key: string; source: string; target: string }[]
  selectedKey: string | null
  onSelect: (key: string) => void
}) {
  const parentRef = useRef<HTMLDivElement>(null)
  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  })

  return (
    <div className="entry-list" ref={parentRef} style={{ overflowY: 'auto', flex: 1, minHeight: 0 }}>
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map((vi) => {
          const e = entries[vi.index]!
          return (
            <button
              key={e.key}
              className={`entry-row ${selectedKey === e.key ? 'selected' : ''}`}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: ROW_HEIGHT,
                transform: `translateY(${vi.start}px)`,
                ...(selectedKey === e.key ? { backgroundColor: 'var(--accent)' } : {}),
              }}
              onClick={() => onSelect(e.key)}
              data-testid={`ws.entry.${vi.index}`}
            >
              <div className="row-source">
                <span>{e.source}</span>
                <code>{e.key}</code>
              </div>
              <div className={`row-target ${!e.target ? 'untranslated' : ''}`}>{e.target || t('ws.addTarget')}</div>
            </button>
          )
        })}
      </div>
    </div>
  )
}

function EntryEditor(props: {
  entryKey: string
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
  onDraft: (v: string) => void
  onCommit: (next: boolean) => void
  onMove: (d: number) => void
}) {
  const copied = useState(false)
  const dirty = props.draft !== props.target
  const patchStage = props.provenance?.patch_stage
  const tkeyLocs = props.tkey?.locations ?? []
  // Document order: the LAST location is the effective one (last field
  // assignment wins); earlier entries are other usages.
  const tkeyPrimary = tkeyLocs.length > 0 ? tkeyLocs[tkeyLocs.length - 1] : undefined
  const tkeyOther = tkeyLocs.slice(0, -1)
  const fmtLoc = (l: { file: string; line?: number }) => (l.line != null ? `${l.file}:${l.line}` : l.file)
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
        </div>
      </div>
      <div className="detail-body">
        <div className="detail-key">
          <code>{props.entryKey}</code>
          <button
            className="icon-btn"
            aria-label={t('ws.copyKey')}
            onClick={() => void navigator.clipboard.writeText(props.entryKey)}
          >
            <Copy />
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
        <div className="detail-actions">
          <button className="btn-primary" disabled={props.busy || !dirty} data-testid="ws.editor-save-next" onClick={() => props.onCommit(true)}>
            <Check /> {t('ws.saveAndNext')}
          </button>
          <button className="icon-btn" aria-label={t('ws.save')} disabled={props.busy || !dirty} onClick={() => props.onCommit(false)}>
            <Save />
          </button>
          <button
            className="icon-btn"
            aria-label={t('ws.revert')}
            disabled={props.busy}
            onClick={() => props.onDraft(props.target)}
          >
            <RotateCcw />
          </button>
        </div>
        {/* SOURCE block (Source Inspector, live): every row is backed by the
            contract snapshot — file+line (source_ref), winner reason
            (selected_by), view-selection facts (provenance) and the TKey
            primary + other-usages locations. Absent data renders an honest
            '—', never a fabricated value. */}
        <div className="context-tabs">
          <button className="active">{t('ws.tabSource')}</button>
        </div>
        <div className="context-content" data-testid="src.block">
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
        {copied[0] && <span hidden />}
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
