// Translation memory (TM live, owner decision A+B+C): the project TM over
// the live contract (project_tm_list/upsert/delete/import/lookup) — the
// Glossary pattern, persist-before-ack.
//   A = accepted translations land automatically (backend, on the ack);
//   B = import (JSON/CSV) — DRAFT by default, never trusted blindly;
//   C = manual CRUD here: provenance=MANUAL, the user picks the status.
// Lookup tiers: exact → normalized → bounded fuzzy; locales are ISOLATED.
import { useEffect, useMemo, useState } from 'react'
import { Plus, Trash2, Search, Check } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { contractErrorText } from '../lib/client/messagesError'
import { t, tEnum } from '../lib/i18n'
import type {
  TmImportResponseDto,
  TmMatchDto,
  TmStatusDto,
  TmUpsertRequestDto,
  TranslationMemoryEntryDto,
} from '../lib/client/types'

const STATUSES: TmStatusDto[] = ['draft', 'accepted', 'reviewed']

export function Tm() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch

  const [entries, setEntries] = useState<TranslationMemoryEntryDto[]>([])
  const [total, setTotal] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Filters (server-side, honest "shown of total").
  const [fLocale, setFLocale] = useState('')
  const [fStatus, setFStatus] = useState<TmStatusDto | ''>('')
  const [fQuery, setFQuery] = useState('')

  // Manual create/edit form (C). editingSource/locale identify the record
  // being edited; null = create mode.
  const [formSource, setFormSource] = useState('')
  const [formTarget, setFormTarget] = useState('')
  const [formLocale, setFormLocale] = useState('')
  const [formStatus, setFormStatus] = useState<TmStatusDto | ''>('')
  const [editing, setEditing] = useState<TranslationMemoryEntryDto | null>(null)

  // Lookup panel (exact → normalized → fuzzy within ONE locale).
  const [lookupQuery, setLookupQuery] = useState('')
  const [lookupLocale, setLookupLocale] = useState('')
  const [lookupResults, setLookupResults] = useState<TmMatchDto[] | null>(null)

  // Import (B): raw JSON/CSV paste; the backend auto-detects the format.
  const [importPayload, setImportPayload] = useState('')
  const [importResult, setImportResult] = useState<TmImportResponseDto | null>(null)

  const [confirmKey, setConfirmKey] = useState<string | null>(null)

  const locales = useMemo(
    () => Array.from(new Set(entries.map((e) => e.target_locale))).sort(),
    [entries],
  )

  useEffect(() => {
    if (!projectId || epoch === undefined) return
    let cancelled = false
    clientInstance
      .getClient()
      .tmList({ project_id: projectId, session_epoch: epoch })
      .then((res) => {
        if (cancelled) return
        setEntries(res.entries)
        setTotal(res.total)
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e))
      })
    return () => {
      cancelled = true
    }
  }, [projectId, epoch])

  if (!projectId || epoch === undefined) {
    return (
      <div className="page-content narrow-page">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{t('tm.eyebrow')}</span>
            <h2>{t('tm.title')}</h2>
            <p>{t('tm.needProject')}</p>
          </div>
        </div>
        <a className="btn-primary" href="#/home">
          {t('ws.backHome')}
        </a>
      </div>
    )
  }

  const filtered = (): Promise<{ entries: TranslationMemoryEntryDto[]; total: number }> =>
    clientInstance.getClient().tmList({
      project_id: projectId,
      session_epoch: epoch,
      ...(fLocale ? { locale: fLocale } : {}),
      ...(fStatus ? { status: fStatus } : {}),
      ...(fQuery.trim() ? { query: fQuery.trim() } : {}),
    })

  const reload = async (): Promise<void> => {
    const res = await filtered()
    setEntries(res.entries)
    setTotal(res.total)
    // A mutation moved the durable revision — adopt it.
    await clientInstance.getClient().snapshot(projectId).catch(() => undefined)
  }

  const run = async (fn: () => Promise<void>): Promise<void> => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await fn()
      await reload()
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

  const resetForm = (): void => {
    setFormSource('')
    setFormTarget('')
    setFormLocale('')
    setFormStatus('')
    setEditing(null)
  }

  // C: create or update by (source_text, target_locale). Provenance is
  // ALWAYS set by the service (MANUAL); omitted status → ACCEPTED.
  const upsert = (): Promise<void> => {
    const src = formSource.trim()
    const tgt = formTarget.trim()
    const loc = formLocale.trim()
    if (!src || !tgt || !loc) return Promise.resolve()
    const req: TmUpsertRequestDto = {
      project_id: projectId,
      session_epoch: epoch,
      source_text: src,
      target_text: tgt,
      target_locale: loc,
      ...(formStatus ? { status: formStatus } : {}),
    }
    return run(async () => {
      await clientInstance.getClient().tmUpsert(req)
      resetForm()
    })
  }

  const startEdit = (e: TranslationMemoryEntryDto): void => {
    setEditing(e)
    setFormSource(e.source_text)
    setFormTarget(e.target_text)
    setFormLocale(e.target_locale)
    setFormStatus(e.status)
  }

  // Status change = a manual edit of the record content it already has:
  // the same key upserts in place (the service stamps provenance=MANUAL —
  // the human, not the import, now owns the record).
  const changeStatus = (e: TranslationMemoryEntryDto, status: TmStatusDto): Promise<void> =>
    run(async () => {
      await clientInstance.getClient().tmUpsert({
        project_id: projectId,
        session_epoch: epoch,
        source_text: e.source_text,
        target_text: e.target_text,
        target_locale: e.target_locale,
        status,
      })
    })

  const remove = (e: TranslationMemoryEntryDto): Promise<void> =>
    run(async () => {
      await clientInstance.getClient().tmDelete({
        project_id: projectId,
        session_epoch: epoch,
        id: e.id,
      })
      setConfirmKey(null)
    })

  // Lookup within ONE locale (isolation is mandatory on the contract side).
  const lookup = (): Promise<void> => {
    const q = lookupQuery.trim()
    const loc = (lookupLocale || fLocale).trim()
    if (!q || !loc) return Promise.resolve()
    return run(async () => {
      const res = await clientInstance.getClient().tmLookup({
        project_id: projectId,
        session_epoch: epoch,
        source_text: q,
        target_locale: loc,
      })
      setLookupResults(res.matches)
    })
  }

  const importRun = (): Promise<void> => {
    const payload = importPayload
    if (!payload.trim()) return Promise.resolve()
    return run(async () => {
      const res = await clientInstance.getClient().tmImport({
        project_id: projectId,
        session_epoch: epoch,
        payload,
      })
      setImportResult(res)
      setImportPayload('')
    })
  }

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('tm.eyebrow')}</span>
          <h2>{t('tm.title')}</h2>
          <p>{t('tm.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="tm.error">
          <span>{error}</span>
        </div>
      )}

      <div className="tm-filters">
        <select
          aria-label={t('tm.locale')}
          value={fLocale}
          onChange={(e) => {
            setFLocale(e.target.value)
            setLookupResults(null)
          }}
          disabled={busy}
          data-testid="tm.filter-locale"
        >
          <option value="">{t('tm.filterAllLocales')}</option>
          {locales.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>
        <select
          aria-label={t('tm.status')}
          value={fStatus}
          onChange={(e) => setFStatus(e.target.value as TmStatusDto | '')}
          disabled={busy}
          data-testid="tm.filter-status"
        >
          <option value="">{t('tm.filterAllStatuses')}</option>
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {tEnum('tm.status', s)}
            </option>
          ))}
        </select>
        <input
          aria-label={t('tm.source')}
          placeholder={`${t('tm.source')}/${t('tm.target')}…`}
          value={fQuery}
          onChange={(e) => setFQuery(e.target.value)}
          disabled={busy}
          data-testid="tm.filter-query"
        />
      </div>
      <p className="tm-count" data-testid="tm.count">
        {t('tm.ofTotal', { shown: entries.length, total })}
      </p>

      <div className="glossary-table tm-table" data-testid="tm.table">
        <div className="glossary-head">
          <span>{t('tm.source')}</span>
          <span>{t('tm.target')}</span>
          <span>{t('tm.locale')}</span>
          <span>{t('tm.status')}</span>
          <span>{t('tm.provenance')}</span>
          <span />
        </div>
        {entries.map((e) => (
          <div key={e.id} data-testid={`tm.row`}>
            <span>{e.source_text}</span>
            <strong>{e.target_text}</strong>
            <span>{e.target_locale}</span>
            <select
              aria-label={`${t('tm.status')} ${e.source_text}`}
              value={e.status}
              disabled={busy}
              onChange={(ev) => void changeStatus(e, ev.target.value as TmStatusDto)}
              data-testid="tm.status-select"
            >
              {STATUSES.map((s) => (
                <option key={s} value={s}>
                  {tEnum('tm.status', s)}
                </option>
              ))}
            </select>
            <span>{tEnum('tm.prov', e.provenance)}</span>
            {confirmKey === e.id ? (
              <span className="confirm-row">
                {t('tm.confirmDelete')}
                <button className="danger" disabled={busy} onClick={() => void remove(e)}>
                  {t('tm.delete')}
                </button>
                <button disabled={busy} onClick={() => setConfirmKey(null)}>
                  ×
                </button>
              </span>
            ) : (
              <span className="confirm-row">
                <button
                  className="icon-btn"
                  aria-label={`${t('tm.edit')} ${e.source_text}`}
                  disabled={busy}
                  onClick={() => startEdit(e)}
                  data-testid="tm.edit"
                >
                  ✎
                </button>
                <button
                  className="icon-btn"
                  aria-label={`${t('tm.delete')} ${e.source_text}`}
                  disabled={busy}
                  onClick={() => setConfirmKey(e.id)}
                  data-testid="tm.delete"
                >
                  <Trash2 size={14} />
                </button>
              </span>
            )}
          </div>
        ))}
        {entries.length === 0 && <p className="page-note">{t('tm.empty')}</p>}
      </div>

      {/* C: create / edit form. */}
      <form
        className="add-term"
        onSubmit={(e) => {
          e.preventDefault()
          void upsert()
        }}
      >
        <input
          required
          placeholder={t('tm.placeholderSource')}
          value={formSource}
          onChange={(e) => setFormSource(e.target.value)}
          aria-label={t('tm.placeholderSource')}
          disabled={busy}
          data-testid="tm.add-source"
        />
        <input
          required
          placeholder={t('tm.placeholderTarget')}
          value={formTarget}
          onChange={(e) => setFormTarget(e.target.value)}
          aria-label={t('tm.placeholderTarget')}
          disabled={busy}
          data-testid="tm.add-target"
        />
        <input
          required
          placeholder={t('tm.placeholderLocale')}
          value={formLocale}
          onChange={(e) => setFormLocale(e.target.value)}
          aria-label={t('tm.placeholderLocale')}
          disabled={busy}
          data-testid="tm.add-locale"
        />
        <select
          aria-label={t('tm.status')}
          value={formStatus}
          onChange={(e) => setFormStatus(e.target.value as TmStatusDto | '')}
          disabled={busy}
          data-testid="tm.add-status"
        >
          <option value="">{tEnum('tm.status', 'accepted')}</option>
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {tEnum('tm.status', s)}
            </option>
          ))}
        </select>
        <button type="submit" className="btn-primary" disabled={busy} data-testid="tm.add-submit">
          {editing ? <Check /> : <Plus />} {editing ? t('tm.save') : t('tm.add')}
        </button>
        {editing && (
          <button type="button" disabled={busy} onClick={resetForm} data-testid="tm.cancel-edit">
            {t('tm.cancel')}
          </button>
        )}
      </form>

      {/* Lookup panel: ranked candidates within ONE locale. */}
      <form
        className="tm-filters"
        style={{ marginTop: 24 }}
        onSubmit={(e) => {
          e.preventDefault()
          void lookup()
        }}
      >
        <select
          aria-label={t('tm.locale')}
          value={lookupLocale || fLocale}
          onChange={(e) => {
            setLookupLocale(e.target.value)
            setLookupResults(null)
          }}
          disabled={busy}
          data-testid="tm.lookup-locale"
        >
          <option value="">{t('tm.filterAllLocales')}</option>
          {locales.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>
        <input
          className="lookup-field"
          placeholder={t('tm.lookupPlaceholder')}
          value={lookupQuery}
          onChange={(e) => setLookupQuery(e.target.value)}
          aria-label={t('tm.lookup')}
          disabled={busy}
          data-testid="tm.lookup-query"
        />
        <button
          type="submit"
          className="btn-primary"
          disabled={busy || !(lookupLocale || fLocale) || !lookupQuery.trim()}
          data-testid="tm.lookup-submit"
        >
          <Search /> {t('tm.lookup')}
        </button>
      </form>
      {lookupResults && (
        <div className="glossary-table tm-table" data-testid="tm.lookup-results">
          {lookupResults.length === 0 ? (
            <p className="page-note">{t('tm.noMatches')}</p>
          ) : (
            lookupResults.map((m) => (
              <div key={m.entry.id}>
                <span>{m.entry.source_text}</span>
                <strong>{m.entry.target_text}</strong>
                <span>{tEnum('tm.matchTier', m.match_kind)}</span>
                <span>{m.distance !== undefined ? `d=${m.distance}` : ''}</span>
                <span>{tEnum('tm.status', m.entry.status)}</span>
                <span>{tEnum('tm.prov', m.entry.provenance)}</span>
              </div>
            ))
          )}
        </div>
      )}

      {/* B: import (JSON array or CSV) — DRAFT by default on the backend. */}
      <div className="tm-import-area">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{t('tm.eyebrow')}</span>
            <h3>{t('tm.importTitle')}</h3>
          </div>
        </div>
        <textarea
          placeholder={t('tm.importPlaceholder')}
          value={importPayload}
          onChange={(e) => setImportPayload(e.target.value)}
          aria-label={t('tm.importTitle')}
          disabled={busy}
          data-testid="tm.import-payload"
        />
        <div className="import-actions">
          <button
            type="button"
            className="btn-primary"
            disabled={busy || !importPayload.trim()}
            onClick={() => void importRun()}
            data-testid="tm.import-submit"
          >
            {t('tm.importRun')}
          </button>
          {importResult && (
            <span className="tm-import-result" data-testid="tm.import-result">
              {t('tm.importResult', {
                imported: importResult.imported,
                updated: importResult.updated,
                skipped: importResult.skipped,
                rejected: importResult.rejected,
              })}
            </span>
          )}
        </div>
      </div>
    </div>
  )
}
