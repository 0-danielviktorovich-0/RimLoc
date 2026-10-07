// Glossary (R1): the project glossary over the live contract
// (project_glossary list/upsert/delete, wave 13) — persist-before-ack.
import { useEffect, useState } from 'react'
import { Plus, Trash2 } from 'lucide-react'
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

      <div className="glossary-table" data-testid="gl.table">
        <div className="glossary-head">
          <span>{t('gl.term')}</span>
          <span>{t('gl.translation')}</span>
          <span />
        </div>
        {terms.map((g) => (
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
