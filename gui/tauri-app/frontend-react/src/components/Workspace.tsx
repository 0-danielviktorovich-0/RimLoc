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
import { ArrowDown, ArrowUp, BookOpen, Braces, Check, ChevronDown, CircleAlert, Copy, FileCode2, FolderOpen, ListFilter, MessagesSquare, RotateCcw, Save, Search, Sparkles } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore, findingEntryKeys, findingsByEntryKey, type WorkspaceEntry } from '../lib/state/project'
import { glossaryStore, subscribe as glossarySubscribe, getState as glossaryGetState } from '../lib/state/glossary'
import { clientInstance } from '../lib/client/instance'
import { SOURCE_LOCALE } from '../lib/languages/registry'
import { folderForm } from '../lib/languages/folderForm'
import { t, tEnum } from '../lib/i18n'
import type { GlossaryTermDto, SourceEntryIdDto, ValidationFindingDto } from '../lib/client/types'

const ROW_HEIGHT = 63

/** §15/§20 handoff: ключ sessionStorage, по которому экран чат-перевода
 *  (свой лейн) подхватывает предотмеченные строки. Значение — JSON
 *  массив display_identity()-строк; прочитал — удали. */
export const CHATBATCH_HANDOFF_KEY = 'rimloc.chatbatch.preset-keys'

/** Точная сериализация ключа батча (`SourceEntryId::display_identity()`):
 *  kind-токен · defType · key. Зеркало той же формулы, что на экране
 *  ChatBatch — полное структурное тождество, не display-key строки. */
function displayIdentity(id: SourceEntryIdDto): string {
  return id.def_type ? `${id.kind}·${id.def_type}·${id.key}` : `${id.kind}·${id.key}`
}

/** §15: перенос выбранной строки в чат-перевод — sessionStorage + навигация.
 *  Никаких автоматических вызовов провайдера: транспорт решает человек. */
function sendToChatBatch(id: SourceEntryIdDto): void {
  try {
    const prev = sessionStorage.getItem(CHATBATCH_HANDOFF_KEY)
    const parsed: unknown = prev ? JSON.parse(prev) : []
    const list = Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === 'string') : []
    const next = new Set(list)
    next.add(displayIdentity(id))
    sessionStorage.setItem(CHATBATCH_HANDOFF_KEY, JSON.stringify([...next]))
  } catch {
    /* приватный режим/битый JSON — навигация всё равно честная */
  }
  window.location.hash = '#/chatbatch'
}

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

  const snapNow = st.snapshot

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
      // §9: только строка с живой находкой валидатора; фикс → revalidate →
      // строка исчезает.
      if (filter === 'issues') return issueKeys.has(e.key)
      return true
    })
  }, [st.entries, search, filter, kindFilter, issueKeys])

  const selectedKey = st.selectedKey ?? visible[0]?.key ?? null
  const selected = st.entries.find((e) => e.key === selectedKey) ?? null

  // §17: первая находка для ТЕКУЩЕЙ строки (порядок отчёта валидатора).
  const selectedFindings = useMemo(
    () => (selected ? findingsByEntryKey(vstate.report).get(selected.key) ?? [] : []),
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
  onDraft: (v: string) => void
  onCommit: (next: boolean) => void
  onMove: (d: number) => void
}) {
  const copied = useState(false)
  // §13: единственная мёртвая кнопка-таб стала настоящими табами.
  const [tab, setTab] = useState<'source' | 'terms'>('source')
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
            style={{ width: 'auto', padding: '0 9px', fontSize: 9 }}
            aria-label={t('ws.toChatbatchHint')}
            title={t('ws.toChatbatchHint')}
            data-testid="ws.editor-to-chatbatch"
            onClick={props.onSendToChatBatch}
          >
            <MessagesSquare size={13} /> {t('ws.toChatbatch')}
          </button>
          <button className="icon-btn" aria-label={t('ws.save')} disabled={props.busy || !dirty} onClick={() => props.onCommit(false)}>
            <Save />
          </button>
          <button
            className="icon-btn"
            aria-label={t('ws.revert')}
            disabled={props.busy}
            // Revert = «чистый черновик = отсутствие записи» (§11): удаляем
            // ключ из drafts, а не пишем committed-текст как черновик —
            // иначе пилюля «Есть несохранённые» считает фантом.
            onClick={() => projectStore.clearDraft(props.entryKey)}
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
          <span style={{ fontSize: 8 }}>{t('ws.mtHint')}</span>
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
        ) : (
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
        )}
        {/* §14: Related — сиблинги того же Def (label ↔ description …).
            Переход честный: projectStore.select(key). */}
        {props.related.length > 0 && (
          <div className="related-string" data-testid="ws.related">
            <span>{t('ws.related')}</span>
            {props.related.map((r) => (
              <button
                key={r.key}
                data-testid={`ws.related.${r.key}`}
                onClick={() => projectStore.select(r.key)}
              >
                <code>{r.key}</code>
                <span>{r.source || '—'}</span>
              </button>
            ))}
          </div>
        )}
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
