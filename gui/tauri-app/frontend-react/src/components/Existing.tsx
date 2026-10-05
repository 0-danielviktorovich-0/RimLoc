// Existing translation (R1, journey J2): dry-run import_existing against
// the OPEN project → honest classification counts → guarded apply of the
// reusable set (persist-before-ack, conflicts never overwritten).
import { useState } from 'react'
import { ArrowRight, Check, FolderOpen, Upload } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { contractErrorText } from '../lib/client/messagesError'
import { t } from '../lib/i18n'
import type { ImportExistingResponseDto } from '../lib/client/types'

export function Existing() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch
  const acked = st.snapshot?.acked_revision ?? st.snapshot?.revision ?? 0

  const [dir, setDir] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [report, setReport] = useState<ImportExistingResponseDto | null>(null)
  const [applied, setApplied] = useState<number | null>(null)

  const pick = async (): Promise<void> => {
    setError(null)
    try {
      const d = await clientInstance.getClient().pickDirectory()
      if (d) setDir(d)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  if (!projectId || epoch === undefined) {
    return (
      <div className="page-content narrow-page">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{t('ex.eyebrow')}</span>
            <h2>{t('ex.title')}</h2>
            <p>{t('ex.needProject')}</p>
          </div>
        </div>
        <a className="btn-primary" href="#/home">
          {t('ws.backHome')}
        </a>
      </div>
    )
  }

  const analyze = async (): Promise<void> => {
    if (!dir.trim()) {
      setError(t('ex.needDir'))
      return
    }
    setBusy(true)
    setError(null)
    setApplied(null)
    try {
      setReport(
        await clientInstance.getClient().importExisting({
          project_id: projectId,
          session_epoch: epoch,
          existing_dir: { path: dir.trim() },
          locale: 'Russian',
        }),
      )
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  const apply = async (): Promise<void> => {
    setBusy(true)
    setError(null)
    try {
      const r = await clientInstance.getClient().applyExisting({
        project_id: projectId,
        expected_revision: acked,
        session_epoch: epoch,
        existing_dir: { path: dir.trim() },
        locale: 'Russian',
      })
      setApplied(r.applied)
      setReport(null)
      await projectStore.adoptExternal()
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('ex.eyebrow')}</span>
          <h2>{t('ex.title')}</h2>
          <p>{t('ex.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="ex.error">
          <span>{error}</span>
        </div>
      )}

      <div className="import-form">
        <label className="field">
          <span>{t('ex.dir')}</span>
          <div className="out-dir-row">
            <input
              value={dir}
              onChange={(e) => setDir(e.target.value)}
              placeholder={t('ex.dirPlaceholder')}
              disabled={busy}
              data-testid="ex.dir"
            />
            <button type="button" onClick={() => void pick()} disabled={busy} data-testid="ex.pick">
              <FolderOpen /> {t('be.pick')}
            </button>
          </div>
        </label>
        <div className="build-actions">
          <button disabled={busy || !dir.trim()} onClick={() => void analyze()} data-testid="ex.analyze">
            <Upload /> {t('ex.analyze')}
          </button>
          <button
            className="btn-primary"
            disabled={busy || !report || report.reusable_count === 0}
            onClick={() => void apply()}
            data-testid="ex.apply"
          >
            <Check /> {t('ex.apply', { count: report?.reusable_count ?? 0 })}
          </button>
        </div>
      </div>

      {report && (
        <>
          <div className="metrics-band" data-testid="ex.metrics">
            <div>
              <strong className="text-success">{report.reusable_count}</strong>
              <span>{t('ex.reusable')}</span>
            </div>
            <div>
              <strong className="text-warning">{report.conflict_count}</strong>
              <span>{t('ex.conflicts')}</span>
            </div>
            <div>
              <strong className="text-info">{report.new_count}</strong>
              <span>{t('ex.newStrings')}</span>
            </div>
            <div>
              <strong className="text-muted-foreground">{report.obsolete_count}</strong>
              <span>{t('ex.obsolete')}</span>
            </div>
            <div>
              <strong>{report.ambiguous_count}</strong>
              <span>{t('ex.ambiguous')}</span>
            </div>
          </div>
          <div className="diff-list" data-testid="ex.reusable">
            {report.reusable.slice(0, 20).map((r) => (
              <div className="diff-row" key={r.key}>
                <code>{r.key}</code>
                <ArrowRight size={13} />
              </div>
            ))}
          </div>
        </>
      )}

      {applied !== null && (
        <div className="inline-success" role="status">
          <Check /> {t('ex.applied', { count: applied })}
        </div>
      )}
      <p className="page-note">{t('ex.safetyNote')}</p>
    </div>
  )
}
