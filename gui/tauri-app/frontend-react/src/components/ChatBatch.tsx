// Chat batch (external-AI workflow WITHOUT an API): the mandate's canonical
// path as ONE screen —
//   workspace checkboxes → create batch → export prompt → Copy prompt →
//   the user pastes the LLM-chat response → Import (strict parse + gates)
//   → preview → Apply (origin=import, persist-before-ack via session.apply).
// The backend owns the parser, the identity/revision/source-hash gates and
// the durable batch state; this screen is the human transport (clipboard).
import { useEffect, useMemo, useState } from 'react'
import { Check, ClipboardCopy, ClipboardPaste, Download, Play, RefreshCw } from 'lucide-react'
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
  SourceEntryIdDto,
} from '../lib/client/types'

/** The exact serialization key the backend pins
 *  (`SourceEntryId::display_identity()`): kind token · defType · key. */
function displayIdentity(id: SourceEntryIdDto): string {
  return id.def_type ? `${id.kind}·${id.def_type}·${id.key}` : `${id.kind}·${id.key}`
}

export function ChatBatch() {
  const st = useProjectState()
  const snap = st.snapshot

  // Selection (workspace checkboxes over the live inventory).
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [untranslatedOnly, setUntranslatedOnly] = useState(true)

  // Batch lifecycle.
  const [batch, setBatch] = useState<ChatBatchDto | null>(null)
  const [stale, setStale] = useState(false)
  const [prompt, setPrompt] = useState('')
  const [responseText, setResponseText] = useState('')
  const [copied, setCopied] = useState(false)

  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Selection candidates follow the open project + its target locale.
  const candidates = useMemo(() => {
    if (untranslatedOnly) return st.entries.filter((e) => !e.target.trim())
    return st.entries
  }, [st.entries, untranslatedOnly])

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

  // 1. canonical selection → deterministic batch (backend pins identities).
  const create = (): Promise<void> => {
    const snapNow = st.snapshot
    if (!snapNow || selected.size === 0) return Promise.resolve()
    return run(async () => {
      const batchDto = await clientInstance.getClient().chatBatchCreate({
        project_id: snapNow.project_id,
        session_epoch: snapNow.session_epoch,
        locale: folderForm(st.targetLocale),
        entry_keys: [...selected],
      })
      setBatch(batchDto)
      setStale(false)
      setPrompt('')
      setResponseText('')
      setSelected(new Set())
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
    })
  }

  const copyPrompt = async (): Promise<void> => {
    if (!prompt) return
    await navigator.clipboard.writeText(prompt)
    setCopied(true)
  }

  // 3. paste response → strict parse → preview (nothing durable yet).
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
    })
  }

  // 4. preview → apply (origin=import through session.apply).
  const applyPreview = (): Promise<void> => {
    const snapNow = st.snapshot
    if (!snapNow || !batch) return Promise.resolve()
    return run(async () => {
      const res = await clientInstance.getClient().chatBatchApply({
        batch_id: batch.batch_id,
        expected_revision: snapNow.acked_revision ?? snapNow.revision,
        session_epoch: snapNow.session_epoch,
      })
      setBatch(res.batch)
      setPrompt('')
      setResponseText('')
      // The ack moved the durable revision — re-adopt the snapshot.
      await projectStore.adoptExternal()
    })
  }

  const badgeStatus: ChatBatchStatusDto | null = batch ? (stale ? 'stale' : batch.status) : null

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

      {/* 1. Selection. */}
      <div className="cb-selection">
        <div className="entry-filters">
          <button
            className={untranslatedOnly ? 'active' : ''}
            onClick={() => setUntranslatedOnly(!untranslatedOnly)}
            data-testid="cb.filter-untranslated"
          >
            {t('cb.untranslatedOnly')}
          </button>
          <span data-testid="cb.selection-count">
            {t('cb.selected', { count: selected.size })}
          </span>
          <button
            className="btn-primary"
            disabled={busy || selected.size === 0 || !!batch}
            onClick={() => void create()}
            data-testid="cb.create"
          >
            {t('cb.createBatch')}
          </button>
        </div>
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

      {/* 3. Paste the response → strict import → preview. */}
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
            <button
              className="btn-primary"
              disabled={busy || batch.status !== 'imported' || !batch.preview?.items.length}
              onClick={() => void applyPreview()}
              data-testid="cb.apply"
            >
              <Play /> {t('cb.apply', { count: batch.preview?.items.length ?? 0 })}
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
              }}
              data-testid="cb.reset"
            >
              <RefreshCw />
            </button>
          </div>
          {batch.preview && batch.preview.items.length > 0 && (
            <div className="glossary-table cb-preview-table" data-testid="cb.preview">
              <div className="glossary-head">
                <span>{t('cb.headKey')}</span>
                <span>{t('cb.headTranslation')}</span>
              </div>
              {batch.preview.items.map((p) => (
                <div key={p.key} data-testid="cb.preview-row">
                  <code>{p.key}</code>
                  <strong>{p.text}</strong>
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
