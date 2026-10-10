// Glossary (R1): the project glossary over the live contract
// (project_glossary list/upsert/delete, wave 13) — persist-before-ack.
// D-V3: the canon .glossary-toolbar is back — client-side term/translation
// filter, JSON export (download blob) and JSON import (file → parse →
// glossaryUpsert per record; a non-JSON file is an inline error, never a
// silent no-op).
import { useEffect, useMemo, useRef, useState } from 'react'
import { Download, Plus, Search, Trash2, Upload } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { contractErrorText } from '../lib/client/messagesError'
import { t } from '../lib/i18n'
import type { GlossaryTermDto } from '../lib/client/types'

export function Glossary() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch

  const [terms, setTerms] = useState<GlossaryTermDto[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [term, setTerm] = useState('')
  const [translation, setTranslation] = useState('')
  const [confirmKey, setConfirmKey] = useState<string | null>(null)
  // D-V3 toolbar state: client-side filter + JSON import/export.
  const [query, setQuery] = useState('')
  const [importError, setImportError] = useState<string | null>(null)
  const [importCount, setImportCount] = useState<number | null>(null)
  const fileInput = useRef<HTMLInputElement>(null)

  const visibleTerms = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return terms
    return terms.filter(
      (g) => g.term.toLowerCase().includes(q) || g.translation.toLowerCase().includes(q),
    )
  }, [terms, query])

  useEffect(() => {
    if (!projectId || epoch === undefined) return
    let cancelled = false
    clientInstance
      .getClient()
      .glossaryList(projectId, epoch)
      .then((list) => {
        if (!cancelled) setTerms(list)
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
            <span className="eyebrow">{t('gl.eyebrow')}</span>
            <h2>{t('gl.title')}</h2>
            <p>{t('gl.needProject')}</p>
          </div>
        </div>
        <a className="btn-primary" href="#/home">
          {t('ws.backHome')}
        </a>
      </div>
    )
  }

  const reload = async (): Promise<void> => {
    setTerms(await clientInstance.getClient().glossaryList(projectId, epoch))
    // A mutation moved the durable revision — adopt it into the store
    // (Existing.tsx pattern: a raw snapshot() call here was discarded and
    // the workspace kept a stale revision until the next full open).
    await projectStore.adoptExternal()
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

  const add = (): Promise<void> => {
    const termTrim = term.trim()
    const tr = translation.trim()
    if (!termTrim || !tr) return Promise.resolve()
    return run(async () => {
      await clientInstance.getClient().glossaryUpsert({
        project_id: projectId,
        session_epoch: epoch,
        term: termTrim,
        translation: tr,
      })
      setTerm('')
      setTranslation('')
    })
  }

  const remove = (g: GlossaryTermDto): Promise<void> =>
    run(async () => {
      await clientInstance.getClient().glossaryDelete({
        project_id: projectId,
        session_epoch: epoch,
        term: g.term,
      })
      setConfirmKey(null)
    })

  // D-V3: JSON export — download blob of {term, translation} records
  // (round-trips with the import below).
  const exportJson = (): void => {
    const blob = new Blob(
      [JSON.stringify(terms.map((g) => ({ term: g.term, translation: g.translation })), null, 2)],
      { type: 'application/json' },
    )
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `glossary-${projectId}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  // D-V3: JSON import — STRICT: only a JSON array of {term, translation}
  // records; anything else (broken JSON, wrong shape, no valid records) is
  // an inline error and nothing is written.
  const importJson = async (file: File): Promise<void> => {
    setImportError(null)
    setImportCount(null)
    let parsed: unknown
    try {
      parsed = JSON.parse(await file.text())
    } catch {
      setImportError(t('gl.importError'))
      return
    }
    if (!Array.isArray(parsed)) {
      setImportError(t('gl.importError'))
      return
    }
    const records: { term: string; translation: string }[] = []
    for (const item of parsed) {
      const rec = item as Record<string, unknown>
      if (
        !rec ||
        typeof rec !== 'object' ||
        typeof rec.term !== 'string' ||
        !rec.term.trim() ||
        typeof rec.translation !== 'string' ||
        !rec.translation.trim()
      ) {
        setImportError(t('gl.importError'))
        return
      }
      records.push({ term: rec.term.trim(), translation: rec.translation.trim() })
    }
    if (records.length === 0) {
      setImportError(t('gl.importError'))
      return
    }
    await run(async () => {
      for (const r of records) {
        await clientInstance.getClient().glossaryUpsert({
          project_id: projectId,
          session_epoch: epoch,
          term: r.term,
          translation: r.translation,
        })
      }
      setImportCount(records.length)
    })
  }

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('gl.eyebrow')}</span>
          <h2>{t('gl.title')}</h2>
          <p>{t('gl.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="gl.error">
          <span>{error}</span>
        </div>
      )}

      {/* D-V3: canon toolbar — filter + JSON round-trip. */}
      <div className="glossary-toolbar" data-testid="gl.toolbar">
        <div className="search-field">
          <Search size={16} />
          <input
            aria-label={t('gl.search')}
            placeholder={t('gl.searchPlaceholder')}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            data-testid="gl.filter"
          />
        </div>
        <button type="button" onClick={exportJson} disabled={busy || terms.length === 0} data-testid="gl.export">
          <Download /> {t('gl.export')}
        </button>
        <button type="button" onClick={() => fileInput.current?.click()} disabled={busy} data-testid="gl.import">
          <Upload /> {t('gl.import')}
        </button>
        <input
          hidden
          ref={fileInput}
          type="file"
          accept=".json,application/json"
          onChange={(e) => {
            const f = e.target.files?.[0]
            e.target.value = ''
            if (f) void importJson(f)
          }}
        />
      </div>
      {importError && (
        <div className="inline-warning" role="alert" data-testid="gl.import-error">
          <span>{importError}</span>
        </div>
      )}
      {importCount !== null && !importError && (
        <p className="page-note" data-testid="gl.import-done">
          {t('gl.importDone', { count: importCount })}
        </p>
      )}

      <div className="glossary-table" data-testid="gl.table">
        <div className="glossary-head">
          <span>{t('gl.term')}</span>
          <span>{t('gl.translation')}</span>
          <span />
        </div>
        {visibleTerms.map((g) => (
          <div key={g.id} data-testid={`gl.row.${g.term}`}>
            <span>{g.term}</span>
            <strong>{g.translation}</strong>
            {confirmKey === g.id ? (
              <span className="confirm-row">
                {t('gl.confirmDelete')}
                <button className="danger" disabled={busy} onClick={() => void remove(g)}>
                  {t('gl.delete')}
                </button>
                <button disabled={busy} onClick={() => setConfirmKey(null)}>
                  ×
                </button>
              </span>
            ) : (
              <button
                className="icon-btn"
                aria-label={`${t('gl.delete')} ${g.term}`}
                disabled={busy}
                onClick={() => setConfirmKey(g.id)}
              >
                <Trash2 size={14} />
              </button>
            )}
          </div>
        ))}
        {terms.length === 0 && <p className="page-note">{t('gl.empty')}</p>}
        {terms.length > 0 && visibleTerms.length === 0 && <p className="page-note">{t('gl.noMatch')}</p>}
      </div>

      <form
        className="add-term"
        onSubmit={(e) => {
          e.preventDefault()
          void add()
        }}
      >
        <input
          required
          placeholder={t('gl.placeholderTerm')}
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          aria-label={t('gl.placeholderTerm')}
          disabled={busy}
          data-testid="gl.add-term"
        />
        <input
          required
          placeholder={t('gl.placeholderTranslation')}
          value={translation}
          onChange={(e) => setTranslation(e.target.value)}
          aria-label={t('gl.placeholderTranslation')}
          disabled={busy}
          data-testid="gl.add-translation"
        />
        <button type="submit" className="btn-primary" disabled={busy} data-testid="gl.add-submit">
          <Plus /> {t('gl.add')}
        </button>
      </form>
    </div>
  )
}
