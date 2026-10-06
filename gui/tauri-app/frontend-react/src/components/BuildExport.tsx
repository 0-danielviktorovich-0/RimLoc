// Build & Export (R1, journey J6): real build_mod and export into a
// caller-chosen directory via the native picker; validation gate before
// build; honest results (files written / reparsed), never demo paths.
import { useState } from 'react'
import { FolderOpen, Package, Play, ShieldCheck, ArrowDownToLine } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { contractErrorText } from '../lib/client/messagesError'
import { folderForm } from '../lib/languages/folderForm'
import { t } from '../lib/i18n'

type Result =
  | { kind: 'build'; files: number; reparsed: number; outDir: string }
  | { kind: 'export'; files: number; reparsed: number; outDir: string }

export function BuildExport() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch
  const [outDir, setOutDir] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<Result | null>(null)

  if (!projectId || epoch === undefined) {
    return (
      <div className="page-content narrow-page">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{t('be.eyebrow')}</span>
            <h2>{t('be.title')}</h2>
            <p>{t('be.needProject')}</p>
          </div>
        </div>
        <a className="btn-primary" href="#/home">
          {t('ws.backHome')}
        </a>
      </div>
    )
  }

  const pickDir = async (): Promise<void> => {
    try {
      const dir = await clientInstance.getClient().pickDirectory()
      if (dir) setOutDir(dir)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const validateFirst = async (): Promise<number> => {
    const report = await clientInstance.getClient().validateProject(projectId, epoch)
    return report.error_count
  }

  const runBuild = async (): Promise<void> => {
    if (!outDir.trim()) {
      setError(t('be.needOutDir'))
      return
    }
    setBusy(true)
    setError(null)
    setResult(null)
    try {
      const errors = await validateFirst()
      if (errors > 0) {
        setError(t('be.validationGate', { count: errors }))
        setBusy(false)
        return
      }
      // АКТИВНАЯ цель проекта, а не захардкоженная 'ru' (тот же класс бага,
      // что закрыт в commit(); контракт хочет strict folder form).
      const r = await clientInstance.getClient().buildModProject(projectId, epoch, outDir.trim(), folderForm(st.targetLocale))
      setResult({ kind: 'build', files: r.files_written, reparsed: r.reparsed_keys, outDir: r.out_dir.path })
    } catch (e) {
      setError(
        e && typeof e === 'object' && 'code' in e
          ? contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e))
          : e instanceof Error
            ? e.message
            : String(e),
      )
    } finally {
      setBusy(false)
    }
  }

  const runExport = async (): Promise<void> => {
    if (!outDir.trim()) {
      setError(t('be.needOutDir'))
      return
    }
    setBusy(true)
    setError(null)
    setResult(null)
    try {
      const r = await clientInstance.getClient().exportProject(projectId, epoch, outDir.trim(), folderForm(st.targetLocale))
      setResult({ kind: 'export', files: r.files_written, reparsed: r.reparsed_keys, outDir: r.out_dir.path })
    } catch (e) {
      setError(
        e && typeof e === 'object' && 'code' in e
          ? contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e))
          : e instanceof Error
            ? e.message
            : String(e),
      )
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('be.eyebrow')}</span>
          <h2>{t('be.title')}</h2>
          <p>{t('be.subtitle')}</p>
        </div>
        <span className="safe-label">
          <ShieldCheck size={16} /> {t('be.safety')}
        </span>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="be.error">
          <span>{error}</span>
        </div>
      )}

      <div className="build-grid">
        <div>
          <h3>{t('be.outputSection')}</h3>
          <label className="field">
            <span>{t('be.outDir')}</span>
            <div className="out-dir-row">
              <input
                value={outDir}
                onChange={(e) => setOutDir(e.target.value)}
                placeholder={t('be.outDirPlaceholder')}
                disabled={busy}
                data-testid="be.outdir"
              />
              <button type="button" onClick={() => void pickDir()} disabled={busy} data-testid="be.pick">
                <FolderOpen /> {t('be.pick')}
              </button>
            </div>
          </label>
          <div className="build-actions">
            <button disabled={busy || !outDir.trim()} onClick={() => void runExport()} data-testid="be.export">
              <ArrowDownToLine /> {t('be.exportFiles')}
            </button>
            <button
              className="btn-primary"
              disabled={busy || !outDir.trim()}
              onClick={() => void runBuild()}
              data-testid="be.build"
            >
              <Package /> {t('be.buildMod')}
            </button>
            <button
              disabled={busy || !outDir.trim()}
              onClick={() => {
                setError(t('be.previewNote'))
              }}
            >
              <Play /> {t('be.preview')}
            </button>
          </div>
          <p className="page-note">{t('be.gateNote')}</p>
        </div>

        <aside className="build-preview" data-testid="be.result">
          <div className="preview-heading">
            <Package size={18} />
            <span>{t('be.resultTitle')}</span>
          </div>
          {result ? (
            <div className="context-content">
              <div>
                <span>{t('be.resFiles')}</span>
                <strong>{result.files}</strong>
              </div>
              <div>
                <span>{t('be.resReparsed')}</span>
                <strong>{result.reparsed}</strong>
              </div>
              <div>
                <span>{t('be.resDir')}</span>
                <code>{result.outDir}</code>
              </div>
            </div>
          ) : (
            <p className="page-note">{t('be.resultEmpty')}</p>
          )}
        </aside>
      </div>
    </div>
  )
}
