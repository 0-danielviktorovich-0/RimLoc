// Workspace (R1 REPRESENTATIVE SCREEN): LEFT project context / CENTER
// virtualized inventory / RIGHT editor. One real project state; the entry
// list is virtualized from day one (mandate §24/§27 — the prototype had
// neither panes nor virtualization).
import { useMemo, useRef, useState } from 'react'
import { PanelGroup, Panel, PanelResizeHandle } from 'react-resizable-panels'
import { useVirtualizer } from '@tanstack/react-virtual'
import { ArrowDown, ArrowUp, Check, RotateCcw, Save, Search, ListFilter, FileCode2, Copy, Braces } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { clientInstance } from '../lib/client/instance'
import { t } from '../lib/i18n'

const ROW_HEIGHT = 63

export function Workspace({ onBack }: { onBack: () => void }) {
  const st = useProjectState()
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState<'all' | 'empty' | 'issues'>('all')

  const visible = useMemo(() => {
    const q = search.trim().toLowerCase()
    return st.entries.filter((e) => {
      if (q && !`${e.key} ${e.source} ${e.target}`.toLowerCase().includes(q)) return false
      if (filter === 'empty') return !e.target.trim()
      if (filter === 'issues') return e.completeness === 'todo' || e.target.includes('{') === false && e.source.includes('{')
      return true
    })
  }, [st.entries, search, filter])

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
        <PanelGroup direction="horizontal" autoSaveId="rimloc-ws-panes">
        <Panel minSize={30} defaultSize={66}>
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

        {/* RIGHT — editor */}
        </Panel>
        <PanelResizeHandle className="ws-handle" aria-label={t('ws.resize')} />
        <Panel minSize={22} defaultSize={34}>
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
        </PanelGroup>
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
  onDraft: (v: string) => void
  onCommit: (next: boolean) => void
  onMove: (d: number) => void
}) {
  const copied = useState(false)
  const dirty = props.draft !== props.target
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
          <span className="locale">EN</span>
        </div>
        <div className="source-block">{props.source}</div>
        <div className="label-line">
          <label htmlFor="translation">{t('ws.translation')}</label>
          <span className="locale">RU</span>
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
          <button disabled={props.busy || !dirty} data-testid="ws.editor-save-next" onClick={() => props.onCommit(true)}>
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
        <div className="context-tabs">
          <button className="active">{t('ws.tabSource')}</button>
        </div>
        <div className="context-content">
          <div>
            <span>{t('ws.file')}</span>
            <code>{props.sourceFile ?? '—'}</code>
          </div>
          <div>
            <span>{t('ws.line')}</span>
            <strong>{props.sourceLine ?? '—'}</strong>
          </div>
          <div>
            <span>{t('ws.whyThisSource')}</span>
            <strong>{props.selectedBy ?? '—'}</strong>
          </div>
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
