// Chat batch (external-AI workflow WITHOUT an API): the mandate's canonical
// path as ONE screen —
//   workspace checkboxes → select all visible → auto-split into numbered
//   batches → export prompt → Copy prompt → the user pastes/imports the
//   LLM-chat response → Import (strict parse + gates) → preview split into
//   safe rows and problem rows (conflicts / needs_review, never applied
//   silently) → Apply (origin=import, persist-before-ack via session.apply).
// Plus the durable batch HISTORY (mandate §27): the list survives restarts
// and any unfinished batch resumes where it stopped.
// The backend owns the parser, the identity/revision/source-hash/conflict
// gates and the durable batch state; this screen is the human transport
// (clipboard + file) — it NEVER calls a provider: no paid calls, by design.
import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  Check,
  ClipboardCopy,
  ClipboardPaste,
  Download,
  FileUp,
  History,
  Play,
  RefreshCw,
} from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { projectStore } from '../lib/state/project'
import { useProjectState } from '../lib/state/useProjectState'
import { contractErrorText } from '../lib/client/messagesError'
import { folderForm } from '../lib/languages/folderForm'
import { t, tEnum } from '../lib/i18n'
import { CHATBATCH_HANDOFF_KEY } from './Workspace'
import type {
  ChatBatchDto,
  ChatBatchStatusDto,
  ChatBatchPreviewItemDto,
  SourceEntryIdDto,
} from '../lib/client/types'

/** The exact serialization key the backend pins
 *  (`SourceEntryId::display_identity()`): kind token · defType · key. */
function displayIdentity(id: SourceEntryIdDto): string {
  return id.def_type ? `${id.kind}·${id.def_type}·${id.key}` : `${id.kind}·${id.key}`
}

// --- local persistence (screen preferences + the selection wave) ---

const SPLIT_KEY = 'rimloc.chatbatch.split'
const WAVE_KEY = 'rimloc.chatbatch.wave.'

interface SplitPrefs {
  maxRows: number
  maxChars: number
}

const DEFAULT_SPLIT: SplitPrefs = { maxRows: 50, maxChars: 6000 }

function loadSplit(): SplitPrefs {
  try {
    const raw = window.localStorage.getItem(SPLIT_KEY)
    if (!raw) return DEFAULT_SPLIT
    const parsed = JSON.parse(raw) as Partial<SplitPrefs>
    const maxRows = Number(parsed.maxRows)
    const maxChars = Number(parsed.maxChars)
    return {
      maxRows: Number.isFinite(maxRows) && maxRows > 0 ? Math.floor(maxRows) : DEFAULT_SPLIT.maxRows,
      maxChars:
        Number.isFinite(maxChars) && maxChars > 0 ? Math.floor(maxChars) : DEFAULT_SPLIT.maxChars,
    }
  } catch {
    return DEFAULT_SPLIT
  }
}

function saveSplit(prefs: SplitPrefs): void {
  try {
    window.localStorage.setItem(SPLIT_KEY, JSON.stringify(prefs))
  } catch {
    /* private mode — the choice stays session-only */
  }
}

/** The numbered selection wave: batch ids in their #1/#2/… order, per
 *  project — survives restarts so the progress line stays honest. */
function loadWave(projectId: string): string[] {
  try {
    const raw = window.localStorage.getItem(WAVE_KEY + projectId)
    if (!raw) return []
    const parsed = JSON.parse(raw) as unknown
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === 'string') : []
  } catch {
    return []
  }
}

function saveWave(projectId: string, ids: string[]): void {
  try {
    window.localStorage.setItem(WAVE_KEY + projectId, JSON.stringify(ids))
  } catch {
    /* private mode */
  }
}

/** Sequential split of the selection into numbered chunks: a chunk grows
 *  until the next row would exceed the row OR the char threshold; a single
 *  oversized row gets its own chunk (never dropped, never merged). */
export function splitSelection(
  rows: Array<{ key: string; source: string }>,
  prefs: SplitPrefs,
): Array<Array<string>> {
  const chunks: string[][] = []
  let cur: string[] = []
  let chars = 0
  for (const r of rows) {
    const wouldRows = cur.length + 1 > prefs.maxRows
    const wouldChars = chars + r.source.length > prefs.maxChars && cur.length > 0
    if (wouldRows || wouldChars) {
      chunks.push(cur)
      cur = []
      chars = 0
    }
    cur.push(r.key)
    chars += r.source.length
  }
  if (cur.length > 0) chunks.push(cur)
  return chunks
}

export function ChatBatch() {
  const st = useProjectState()
  const snap = st.snapshot

  // Selection (workspace checkboxes over the live inventory).
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [untranslatedOnly, setUntranslatedOnly] = useState(true)

  // Auto-split preferences (configurable thresholds).
  const [split, setSplit] = useState<SplitPrefs>(() => loadSplit())

  // Batch lifecycle (the ACTIVE batch the sections below work on).
  const [batch, setBatch] = useState<ChatBatchDto | null>(null)
  const [stale, setStale] = useState(false)
  const [prompt, setPrompt] = useState('')
  const [responseText, setResponseText] = useState('')
  const [copied, setCopied] = useState(false)

  // Problem-row overrides: keys the human EXPLICITLY confirmed despite a
  // conflict / needs_review flag (never applied silently without it).
  const [overrides, setOverrides] = useState<Set<string>>(new Set())

  // Durable batch history (mandate §27) + the numbered selection wave.
  const [history, setHistory] = useState<ChatBatchDto[]>([])
  const [wave, setWave] = useState<string[]>(() => (snap ? loadWave(snap.project_id) : []))

  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Selection candidates follow the open project + its target locale.
  const candidates = useMemo(() => {
    if (untranslatedOnly) return st.entries.filter((e) => !e.target.trim())
    return st.entries
  }, [st.entries, untranslatedOnly])

  const refreshHistory = useCallback(async (): Promise<ChatBatchDto[]> => {
    const snapNow = st.snapshot
    if (!snapNow) return []
    try {
      const res = await clientInstance.getClient().chatBatchList({ project_id: snapNow.project_id })
      setHistory(res.batches)
      return res.batches
    } catch {
      return [] // the history is a read surface; errors surface elsewhere
    }
  }, [st.snapshot])

  // History loads once per open project; the wave view survives restarts.
  useEffect(() => {
    if (!snap) {
      setBatch(null)
      setPrompt('')
      setResponseText('')
      setSelected(new Set())
      setStale(false)
      setHistory([])
      setWave([])
      setOverrides(new Set())
      return
    }
    setWave(loadWave(snap.project_id))
    void refreshHistory()
    // Only on project change: the history refetches after operations.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snap?.project_id])

  // A closed project invalidates the screen-local batch view.
  useEffect(() => {
    if (!snap) {
      setBatch(null)
      setPrompt('')
      setResponseText('')
      setSelected(new Set())
      setStale(false)
    }
  }, [snap])

  // §15/§20 handoff: Workspace пишет display_identity()-строки в
  // sessionStorage перед навигацией сюда. При монте читаем ключ один раз,
  // предотмечаем совпавшие строки-кандидаты и удаляем ключ (прочитал —
  // удали). Тултип «Строка будет предотмечена» становится правдой.
  useEffect(() => {
    let raw: string | null = null
    try {
      raw = sessionStorage.getItem(CHATBATCH_HANDOFF_KEY)
    } catch {
      return // приватный режим — подхватывать нечего
    }
    if (!raw) return
    sessionStorage.removeItem(CHATBATCH_HANDOFF_KEY)
    let parsed: unknown
    try {
      parsed = JSON.parse(raw)
    } catch {
      return // битый JSON — честно игнорируем, ничего не отмечаем
    }
    if (!Array.isArray(parsed)) return
    const handed = new Set(parsed.filter((x): x is string => typeof x === 'string'))
    if (handed.size === 0) return
    setSelected((prev) => {
      const next = new Set(prev)
      for (const e of st.entries) {
        const id = displayIdentity(e.identity)
        if (handed.has(id)) next.add(id)
      }
      return next
    })
    // Только при монте: инвентарь уже загружен Workspace до навигации,
    // а ключ одноразовый (потреблён выше — повторный запуск стал бы no-op).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Poll the durable batch status while it can still drift (exported /
  // imported): the badge shows drift without waiting for a refused op.
  useEffect(() => {
    if (!batch || (batch.status !== 'exported' && batch.status !== 'imported')) return
    let cancelled = false
    const tick = () => {
      clientInstance
        .getClient()
        .chatBatchStatus({ batch_id: batch.batch_id })
        .then((res) => {
          if (cancelled) return
          setBatch(res.batch)
          setStale(res.stale)
        })
        .catch(() => undefined)
    }
    const timer = window.setInterval(tick, 2000)
    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [batch?.batch_id, batch?.status])

  // Live translation presence per key — the honest progress basis: a row
  // counts as translated when the project holds a non-empty target.
  const translatedByKey = useMemo(() => {
    const m = new Map<string, boolean>()
    for (const e of st.entries) m.set(displayIdentity(e.identity), e.target.trim() !== '')
    return m
  }, [st.entries])

  /** Progress over the selection wave: applied rows per batch + totals. */
  const waveProgress = useMemo(() => {
    const byId = new Map(history.map((b) => [b.batch_id, b]))
    const items = wave
      .map((id, idx) => {
        const b = byId.get(id)
        if (!b) return null
        const applied = b.items.filter((i) => translatedByKey.get(i.key)).length
        return { idx, batch: b, applied }
      })
      .filter((x): x is { idx: number; batch: ChatBatchDto; applied: number } => x !== null)
    const total = items.reduce((acc, x) => acc + x.batch.items.length, 0)
    const translated = items.reduce((acc, x) => acc + x.applied, 0)
    return { items, total, translated }
  }, [wave, history, translatedByKey])

  if (!snap) {
    return (
      <div className="page-content narrow-page">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{t('cb.eyebrow')}</span>
            <h2>{t('cb.title')}</h2>
            <p>{t('cb.needProject')}</p>
          </div>
        </div>
        <a className="btn-primary" href="#/home">
          {t('ws.backHome')}
        </a>
      </div>
    )
  }

  const toggle = (key: string): void =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })

  const run = async (fn: () => Promise<void>): Promise<void> => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await fn()
    } catch (e) {
      setError(
        contractErrorText(
          (e as { code?: string }).code ?? 'internal',
          e instanceof Error ? e.message : String(e),
        ),
      )
    } finally {
      setBusy(false)
    }
  }

  const selectAllVisible = (): void =>
    setSelected(new Set(candidates.map((e) => displayIdentity(e.identity))))

  // 1. canonical selection → numbered batches (auto-split by thresholds).
  // Each chunk is its own durable backend batch; the wave order is the
  // numbering (#1/#2/…). No provider is ever called — the transport is
  // the user's clipboard, by design (no automatic paid calls exist here).
  const create = (): Promise<void> => {
    const snapNow = st.snapshot
    if (!snapNow || selected.size === 0) return Promise.resolve()
    return run(async () => {
      const byKey = new Map(st.entries.map((e) => [displayIdentity(e.identity), e]))
      const rows = [...selected]
        .map((k) => ({ key: k, source: byKey.get(k)?.source ?? '' }))
        .filter((r) => byKey.has(r.key))
      const chunks = splitSelection(rows, split)
      const created: string[] = []
      let first: ChatBatchDto | null = null
      for (const chunk of chunks) {
        const dto = await clientInstance.getClient().chatBatchCreate({
          project_id: snapNow.project_id,
          session_epoch: snapNow.session_epoch,
          locale: folderForm(st.targetLocale),
          entry_keys: chunk,
        })
        created.push(dto.batch_id)
        if (!first) first = dto
      }
      const waveNext = created
      setWave(waveNext)
      saveWave(snapNow.project_id, waveNext)
      setBatch(first)
      setStale(false)
      setPrompt('')
      setResponseText('')
      setSelected(new Set())
      setOverrides(new Set())
      await refreshHistory()
    })
  }

  // 2. export batch → the prompt (records revision + source hash gates).
  const exportBatch = (): Promise<void> => {
    const snapNow = st.snapshot
    if (!snapNow || !batch) return Promise.resolve()
    return run(async () => {
      const res = await clientInstance.getClient().chatBatchExport({
        batch_id: batch.batch_id,
        session_epoch: snapNow.session_epoch,
      })
      setBatch(res.batch)
      setPrompt(res.prompt)
      setStale(false)
      setCopied(false)
      await refreshHistory()
    })
  }

  const copyPrompt = async (): Promise<void> => {
    if (!prompt) return
    await navigator.clipboard.writeText(prompt)
    setCopied(true)
  }

  // 3. paste OR file-import the response → strict parse → preview.
  const importResponse = (): Promise<void> => {
    const snapNow = st.snapshot
    if (!snapNow || !batch || !responseText.trim()) return Promise.resolve()
    return run(async () => {
      const res = await clientInstance.getClient().chatBatchImport({
        batch_id: batch.batch_id,
        session_epoch: snapNow.session_epoch,
        response_text: responseText,
      })
      setBatch(res.batch)
      setStale(false)
      setOverrides(new Set())
      await refreshHistory()
    })
  }

  const importFile = (file: File): void => {
    const reader = new FileReader()
    reader.onload = () => setResponseText(String(reader.result ?? ''))
    reader.onerror = () => setError(t('cb.fileError'))
    reader.readAsText(file)
  }

  // 4. preview → apply: clean rows always, problem rows ONLY when the
  // human listed them (override) — the backend holds the rest back and
  // reports `held_back`.
  const applyPreview = (): Promise<void> => {
    const snapNow = st.snapshot
    if (!snapNow || !batch) return Promise.resolve()
    return run(async () => {
      const res = await clientInstance.getClient().chatBatchApply({
        batch_id: batch.batch_id,
        expected_revision: snapNow.acked_revision ?? snapNow.revision,
        session_epoch: snapNow.session_epoch,
        override_keys: [...overrides],
      })
      setBatch(res.batch)
      setPrompt('')
      setResponseText('')
      setOverrides(new Set())
      // The ack moved the durable revision — re-adopt the snapshot.
      await projectStore.adoptExternal()
      await refreshHistory()
    })
  }

  // Resume a historical batch: the durable state comes back (preview for
  // `imported`; for `exported` the prompt re-renders via re-export, since
  // the prompt itself is not persisted — honest, and the gates re-check).
  const resume = (b: ChatBatchDto): void => {
    setBatch(b)
    setStale(false)
    setPrompt('')
    setResponseText('')
    setOverrides(new Set())
  }

  const badgeStatus: ChatBatchStatusDto | null = batch ? (stale ? 'stale' : batch.status) : null

  // Preview partition: safe rows vs problems (never applied silently).
  const preview = batch?.preview?.items ?? []
  const isProblem = (p: ChatBatchPreviewItemDto): boolean =>
    !!p.needs_review || !!p.conflict || !!p.structural_problem
  const safeRows = preview.filter((p) => !isProblem(p))
  const problemRows = preview.filter(isProblem)

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('cb.eyebrow')}</span>
          <h2>{t('cb.title')}</h2>
          <p>{t('cb.subtitle')}</p>
        </div>
        {badgeStatus && (
          <span
            className={`cb-status cb-status-${badgeStatus}`}
            data-testid="cb.status-badge"
            role="status"
          >
            {tEnum('cb.status', badgeStatus)}
          </span>
        )}
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="cb.error">
          <span>{error}</span>
        </div>
      )}

      {/* 1. Selection + auto-split thresholds. */}
      <div className="cb-selection">
        <div className="entry-filters">
          <button
            className={untranslatedOnly ? 'active' : ''}
            onClick={() => setUntranslatedOnly(!untranslatedOnly)}
            data-testid="cb.filter-untranslated"
          >
            {t('cb.untranslatedOnly')}
          </button>
          <button
            onClick={selectAllVisible}
            disabled={candidates.length === 0 || !!batch}
            data-testid="cb.select-all"
          >
            {t('cb.selectAll')}
          </button>
          <span data-testid="cb.selection-count">
            {t('cb.selected', { count: selected.size })}
          </span>
          <label>
            {t('cb.splitRows')}
            <input
              type="number"
              min={1}
              value={split.maxRows}
              disabled={!!batch}
              onChange={(e) => {
                const next = { ...split, maxRows: Math.max(1, Math.floor(Number(e.target.value) || 1)) }
                setSplit(next)
                saveSplit(next)
              }}
              data-testid="cb.split-rows"
            />
          </label>
          <label>
            {t('cb.splitChars')}
            <input
              type="number"
              min={1}
              value={split.maxChars}
              disabled={!!batch}
              onChange={(e) => {
                const next = { ...split, maxChars: Math.max(1, Math.floor(Number(e.target.value) || 1)) }
                setSplit(next)
                saveSplit(next)
              }}
              data-testid="cb.split-chars"
            />
          </label>
          <button
            className="btn-primary"
            disabled={busy || selected.size === 0 || !!batch}
            onClick={() => void create()}
            data-testid="cb.create"
          >
            {t('cb.createBatch')}
          </button>
        </div>
        <p className="page-note" data-testid="cb.split-note">
          {t('cb.splitNote', { rows: split.maxRows, chars: split.maxChars })}
        </p>
        <div className="glossary-table" data-testid="cb.entries">
          {candidates.map((e) => {
            const id = displayIdentity(e.identity)
            return (
              <label key={id} className="cb-entry-row" data-testid={`cb.entry`}>
                <input
                  type="checkbox"
                  checked={selected.has(id)}
                  onChange={() => toggle(id)}
                  disabled={busy || !!batch}
                  data-testid={`cb.entry.${id}`}
                  aria-label={`${t('cb.select')} ${e.key}`}
                />
                <span>{e.source}</span>
                <code>{e.key}</code>
              </label>
            )
          })}
          {candidates.length === 0 && <p className="page-note">{t('cb.noCandidates')}</p>}
        </div>
      </div>

      {/* Selection wave progress (numbered batches, honest per-row basis). */}
      {waveProgress.items.length > 0 && (
        <div className="cb-step" data-testid="cb.wave">
          <p className="page-note" data-testid="cb.progress">
            {t('cb.progress', {
              translated: waveProgress.translated,
              total: waveProgress.total,
              count: waveProgress.items.length,
            })}
          </p>
          {waveProgress.items.map((x) => (
            <div key={x.batch.batch_id} className="import-actions" data-testid={`cb.wave-item.${x.batch.batch_id}`}>
              <button
                className={batch?.batch_id === x.batch.batch_id ? 'active' : ''}
                onClick={() => resume(x.batch)}
                data-testid={`cb.wave-open.${x.batch.batch_id}`}
              >
                {t('cb.batchN', { n: x.idx + 1 })} · {tEnum('cb.status', x.batch.status)} ·{' '}
                {t('cb.appliedOf', { applied: x.applied, total: x.batch.items.length })}
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Durable batch history (mandate §27): unfinished batches visible,
          resume after a restart — the state lives in the managed store. */}
      <div className="cb-step" data-testid="cb.history">
        <p className="page-note">
          <History size={14} /> {t('cb.history')}
        </p>
        {history.length === 0 && <p className="page-note">{t('cb.historyEmpty')}</p>}
        {history.map((b) => (
          <div key={b.batch_id} className="import-actions" data-testid={`cb.history-item.${b.batch_id}`}>
            <code>{b.batch_id}</code>
            <span
              className={`cb-status cb-status-${b.status}`}
              data-testid={`cb.history-status.${b.batch_id}`}
            >
              {tEnum('cb.status', b.status)}
            </span>
            <span>
              {t('cb.historyMeta', {
                count: b.items.length,
                locale: b.locale,
                date: new Date(b.created_at_ms).toLocaleString(),
              })}
            </span>
            {b.status !== 'done' && (
              <button
                onClick={() => resume(b)}
                disabled={busy}
                data-testid={`cb.resume.${b.batch_id}`}
              >
                {t('cb.resume')}
              </button>
            )}
          </div>
        ))}
      </div>

      {/* 2. Export + copy the prompt. */}
      {batch && batch.status !== 'done' && (
        <div className="cb-step" data-testid="cb.export-section">
          <div className="import-actions">
            <button
              className="btn-primary"
              disabled={busy}
              onClick={() => void exportBatch()}
              data-testid="cb.export"
            >
              <Download /> {batch.status === 'not_started' ? t('cb.export') : t('cb.reexport')}
            </button>
            <button
              className="btn-primary"
              disabled={busy || !prompt}
              onClick={() => void copyPrompt()}
              data-testid="cb.copy-prompt"
            >
              <ClipboardCopy /> {copied ? t('cb.copied') : t('cb.copyPrompt')}
            </button>
            {stale && <span className="tm-import-result">{t('cb.staleNote')}</span>}
          </div>
          {prompt && (
            <textarea
              readOnly
              value={prompt}
              onFocus={(e) => e.currentTarget.select()}
              rows={Math.min(16, prompt.split('\n').length + 1)}
              aria-label={t('cb.promptAria')}
              data-testid="cb.prompt"
            />
          )}
        </div>
      )}

      {/* 3. Paste or file-import the response → strict import → preview. */}
      {batch && (batch.status === 'exported' || batch.status === 'imported') && !stale && (
        <div className="cb-step" data-testid="cb.import-section">
          <textarea
            placeholder={t('cb.pastePlaceholder')}
            value={responseText}
            onChange={(e) => setResponseText(e.target.value)}
            aria-label={t('cb.pasteAria')}
            disabled={busy}
            data-testid="cb.paste"
          />
          <div className="import-actions">
            <button
              className="btn-primary"
              disabled={busy || !responseText.trim()}
              onClick={() => void importResponse()}
              data-testid="cb.import"
            >
              <ClipboardPaste /> {t('cb.import')}
            </button>
            <label className="btn-primary" data-testid="cb.import-file-label">
              <FileUp /> {t('cb.importFile')}
              <input
                type="file"
                accept=".txt,.md,.json,text/plain"
                style={{ display: 'none' }}
                disabled={busy}
                onChange={(e) => {
                  const f = e.target.files?.[0]
                  if (f) importFile(f)
                  e.target.value = ''
                }}
                data-testid="cb.import-file"
                aria-label={t('cb.importFileAria')}
              />
            </label>
            <button
              className="btn-primary"
              disabled={busy || batch.status !== 'imported' || preview.length === 0}
              onClick={() => void applyPreview()}
              data-testid="cb.apply"
            >
              <Play /> {t('cb.apply', { count: safeRows.length + overrides.size })}
            </button>
            <button
              className="icon-btn"
              aria-label={t('cb.reset')}
              disabled={busy}
              onClick={() => {
                setBatch(null)
                setPrompt('')
                setResponseText('')
                setStale(false)
                setOverrides(new Set())
              }}
              data-testid="cb.reset"
            >
              <RefreshCw />
            </button>
          </div>
          {safeRows.length > 0 && (
            <div className="glossary-table cb-preview-table" data-testid="cb.preview">
              <div className="glossary-head">
                <span>{t('cb.headKey')}</span>
                <span>{t('cb.headTranslation')}</span>
              </div>
              {safeRows.map((p) => (
                <div key={p.key} data-testid="cb.preview-row">
                  <code>{p.key}</code>
                  <strong>{p.text}</strong>
                </div>
              ))}
            </div>
          )}

          {/* Problems: never applied silently — a row lands only with the
              explicit per-row override (or stays for manual review). */}
          {problemRows.length > 0 && (
            <div className="glossary-table cb-preview-table" data-testid="cb.problem-list">
              <div className="glossary-head">
                <span>{t('cb.headKey')}</span>
                <span>{t('cb.problems')}</span>
              </div>
              {problemRows.map((p: ChatBatchPreviewItemDto) => (
                <div key={p.key} data-testid={`cb.problem-row.${p.key}`}>
                  <code>{p.key}</code>
                  <div>
                    {p.conflict ? (
                      <p className="page-note">
                        {t(`cb.conflict.${p.conflict.reason}`)}
                        {': '}
                        {t('cb.conflictCurrent', { text: p.conflict.current_text })}
                      </p>
                    ) : null}
                    {p.needs_review && <p className="page-note">{t('cb.needsReview')}</p>}
                    {p.structural_problem && (
                      <p className="page-note">
                        {t('cb.structuralProblem', { detail: p.structural_problem })}
                      </p>
                    )}
                    {p.note && <p className="page-note">{t('cb.modelNote', { note: p.note })}</p>}
                    <p>
                      <strong>{p.text}</strong>
                    </p>
                    <label>
                      <input
                        type="checkbox"
                        checked={overrides.has(p.key)}
                        onChange={() =>
                          setOverrides((prev) => {
                            const next = new Set(prev)
                            if (next.has(p.key)) next.delete(p.key)
                            else next.add(p.key)
                            return next
                          })
                        }
                        disabled={busy}
                        data-testid={`cb.override.${p.key}`}
                      />
                      {t('cb.override')}
                    </label>
                  </div>
                </div>
              ))}
            </div>
          )}
          <p className="page-note">
            <Check size={14} /> {t('cb.safetyNote')}
          </p>
        </div>
      )}

      {/* Done: the durable result. */}
      {batch?.status === 'done' && (
        <div className="cb-step" data-testid="cb.done-section">
          <p className="page-note">
            <Check size={14} />{' '}
            {t('cb.done', {
              count: batch.items.length,
              revision: batch.applied_revision ?? 0,
            })}
          </p>
          <button
            className="btn-primary"
            onClick={() => {
              setBatch(null)
              setResponseText('')
            }}
            data-testid="cb.new-batch"
          >
            {t('cb.newBatch')}
          </button>
        </div>
      )}
    </div>
  )
}
