// Diagnostics (R1, journey J9): a REAL failure → human-readable result →
// safe support bundle (project_diagnose). The bundle path comes from the
// contract; secrets are sanitized by the services layer.
import { useState } from 'react'
import { FolderOpen, Package } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { contractErrorText } from '../lib/client/messagesError'
import { t } from '../lib/i18n'
import type { DiagnoseResponseDto } from '../lib/client/types'

export function Diagnostics() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const [outDir, setOutDir] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [report, setReport] = useState<DiagnoseResponseDto | null>(null)

  const pick = async (): Promise<void> => {
    try {
      const dir = await clientInstance.getClient().pickDirectory()
      if (dir) setOutDir(dir)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const run = async (): Promise<void> => {
    if (!projectId || !outDir.trim()) {
      setError(t('diag.needProjectAndDir'))
      return
    }
    setBusy(true)
    setError(null)
    setReport(null)
    try {
      setReport(await clientInstance.getClient().diagnoseProject(projectId, outDir.trim()))
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('diag.eyebrow')}</span>
          <h2>{t('diag.title')}</h2>
          <p>{t('diag.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="diag.error">
          <span>{error}</span>
        </div>
      )}

      <label className="field">
        <span>{t('be.outDir')}</span>
        <div className="out-dir-row">
          <input
            value={outDir}
            onChange={(e) => setOutDir(e.target.value)}
            placeholder={t('be.outDirPlaceholder')}
            disabled={busy}
            data-testid="diag.outdir"
          />
          <button type="button" onClick={() => void pick()} disabled={busy} data-testid="diag.pick">
            <FolderOpen /> {t('be.pick')}
          </button>
        </div>
      </label>
      <div className="build-actions">
        <button className="btn-primary" disabled={busy || !projectId} onClick={() => void run()} data-testid="diag.run">
          <Package /> {t('diag.run')}
        </button>
      </div>
      {!projectId && <p className="page-note">{t('diag.needProject')}</p>}

      {report && (
        <div className="build-preview" data-testid="diag.result">
          <div className="preview-heading">
            <Package size={18} />
            <span>{t('diag.resultTitle')}</span>
          </div>
          <div className="context-content">
            <div>
              <span>{t('diag.operationId')}</span>
              <code>{report.operation_id}</code>
            </div>
            <div>
              <span>{t('be.resDir')}</span>
              <code>{report.bundle_dir.path}</code>
            </div>
            <div>
              <span>{t('diag.files')}</span>
              <strong>{report.files.length}</strong>
            </div>
            <div>
              <span>{t('diag.redacted')}</span>
              <strong>{report.redacted_count}</strong>
            </div>
            <div>
              <span>{t('diag.excluded')}</span>
              <strong>{report.excluded_count}</strong>
            </div>
          </div>
          <p className="page-note">{t('diag.copyNote')}</p>
        </div>
      )}
    </div>
  )
}
