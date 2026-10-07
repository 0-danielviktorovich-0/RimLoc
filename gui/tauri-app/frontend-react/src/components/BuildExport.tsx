// Build & Export (R1, journey J6): real build_mod and export into a
// caller-chosen directory via the native picker; validation gate before
// build; honest results (files written / reparsed), never demo paths.
// §8 F8.1: the Preview button runs the LIVE validator and shows a result
// panel (status + counts + first findings + destination + PLANNED output
// structure) — never a promise-text rendered as an error. §8 F8.3:
// skipped_unknown_type from a build/export ack is shown, not dropped.
// §8 F8.4: after a successful build/export the output folder opens through
// the guarded reveal_path shell command (session allow-list on the backend).
import { useState } from 'react'
import {
  FolderOpen,
  Package,
  Play,
  ShieldCheck,
  ArrowDownToLine,
  CircleAlert,
  CheckCircle2,
} from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { contractErrorText } from '../lib/client/messagesError'
import { folderForm } from '../lib/languages/folderForm'
import type { ValidateProjectResponseDto } from '../lib/client/types'
import { t } from '../lib/i18n'

type Result =
  | {
      kind: 'build'
      files: number
      reparsed: number
      outDir: string
      skipped: string[]
    }
  | {
      kind: 'export'
      files: number
      reparsed: number
      outDir: string
      skipped: string[]
    }

/** Snapshot of one live validation run + the state it was valid FOR
 *  (revision / locale / epoch) — anything else is a stale check. */
interface PreviewState {
  report: ValidateProjectResponseDto
  revision: number
  locale: string
  epoch: number
}

const PREVIEW_FINDINGS_LIMIT = 5

/** ApiError-shaped rejections ({message}) render their message — the reveal
 *  shell refusal is a plain ApiError, not a contract DTO. */
function errText(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) {
    const m = (e as { message?: unknown }).message
    if (typeof m === 'string') return m
  }
  return e instanceof Error ? e.message : String(e)
}

export function BuildExport() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch
  const [outDir, setOutDir] = useState('')
  const [busy, setBusy] = useState(false)
  const [validating, setValidating] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<Result | null>(null)
  const [preview, setPreview] = useState<PreviewState | null>(null)

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

  /** §8 F8.1: run the live validator over the ACTIVE target locale and keep
   *  the report with the state it addressed. Returns null on a typed
   *  failure (the error is rendered, never a fabricated clean state). */
  const validateNow = async (): Promise<ValidateProjectResponseDto | null> => {
    setValidating(true)
    try {
      const locale = folderForm(st.targetLocale)
      const report = await clientInstance.getClient().validateProject(projectId, epoch, locale)
      setPreview({ report, revision: st.snapshot?.revision ?? -1, locale, epoch })
      return report
    } catch (e) {
      setError(errText(e))
      return null
    } finally {
      setValidating(false)
    }
  }

  /** §8 pre-build gate: a preview is reusable ONLY when nothing it addressed
   *  changed (revision / locale / epoch); otherwise the validator re-runs
   *  right here — the panel refreshes with the fresh report. */
  const ensureFreshValidation = async (current: PreviewState | null): Promise<ValidateProjectResponseDto | null> => {
    const fresh =
      current &&
      current.epoch === epoch &&
      current.revision === (st.snapshot?.revision ?? -1) &&
      current.locale === folderForm(st.targetLocale)
    if (fresh) return current.report
    return validateNow()
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
      const report = await ensureFreshValidation(preview)
      if (!report) {
        setBusy(false)
        return
      }
      if (report.error_count > 0) {
        setError(t('be.validationGate', { count: report.error_count }))
        setBusy(false)
        return
      }
      // АКТИВНАЯ цель проекта, а не захардкоженная 'ru' (тот же класс бага,
      // что закрыт в commit(); контракт хочет strict folder form).
      const r = await clientInstance.getClient().buildModProject(projectId, epoch, outDir.trim(), folderForm(st.targetLocale))
      setResult({
        kind: 'build',
        files: r.files_written,
        reparsed: r.reparsed_keys,
        outDir: r.out_dir.path,
        skipped: r.skipped_unknown_type ?? [],
      })
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
      setResult({
        kind: 'export',
        files: r.files_written,
        reparsed: r.reparsed_keys,
        outDir: r.out_dir.path,
        skipped: r.skipped_unknown_type ?? [],
      })
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

  /** §8 F8.4: open the last successful output folder. The backend refuses
   *  anything that is not a session output — the refusal renders as-is. */
  const revealOutput = async (): Promise<void> => {
    if (!result) return
    setError(null)
    try {
      await clientInstance.getClient().revealPath(result.outDir)
    } catch (e) {
      setError(errText(e))
    }
  }

  // §8 F8.1: a preview is stale the moment the state it addressed moved.
  const previewStale =
    preview !== null &&
    (preview.epoch !== epoch ||
      preview.revision !== (st.snapshot?.revision ?? -1) ||
      preview.locale !== folderForm(st.targetLocale))
  const previewFindings = preview ? preview.report.findings.slice(0, PREVIEW_FINDINGS_LIMIT) : []
  const previewMore = preview ? Math.max(0, preview.report.findings.length - PREVIEW_FINDINGS_LIMIT) : 0
  const previewOk = preview ? preview.report.status === 'succeeded' && preview.report.error_count === 0 : false

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
              disabled={busy || validating || !outDir.trim()}
              onClick={() => void validateNow()}
              data-testid="be.preview"
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

          {/* §8 F8.1: live preview — validation status + counts + first
              findings + destination + PLANNED output structure. Rendered
              only while a validation runs or a report exists (never during
              a plain build/export — that would promise a check that is not
              running). */}
          {(preview || validating) && (
            <div data-testid="be.previewPanel" style={{ marginBottom: 12 }}>
              <div className="preview-heading">
                <Play size={14} />
                <span>{t('be.previewTitle')}</span>
              </div>
              {!preview ? (
                <p className="page-note">{t('be.previewRunning')}</p>
              ) : (
                <div className="context-content">
                  <div>
                    {previewOk ? (
                      <CheckCircle2 className="text-success" size={16} />
                    ) : (
                      <CircleAlert className="text-destructive" size={16} />
                    )}
                    <strong data-testid="be.previewStatus">
                      {t(previewOk ? 'be.previewStatusOk' : 'be.previewStatusFailed')}
                    </strong>
                  </div>
                  <div>
                    <span>{t('be.prevErrors')}</span>
                    <strong>{preview.report.error_count}</strong>
                  </div>
                  <div>
                    <span>{t('be.prevWarnings')}</span>
                    <strong>{preview.report.warning_count}</strong>
                  </div>
                  <div>
                    <span>{t('be.prevInfos')}</span>
                    <strong>{preview.report.info_count}</strong>
                  </div>
                  {previewStale && (
                    <p className="page-note" data-testid="be.previewStale">
                      {t('be.previewStale')}
                    </p>
                  )}
                  {previewFindings.length > 0 && (
                    <div className="finding-list" data-testid="be.previewFindings">
                      {previewFindings.map((f, i) => (
                        <div key={`${f.id?.key ?? 'root'}-${i}`} className="finding-row">
                          <CircleAlert
                            className={f.severity === 'error' ? 'text-destructive' : 'text-warning'}
                            size={15}
                          />
                          <div>
                            <strong>{f.kind}</strong>
                            <code>{f.id?.key ?? f.key}</code>
                          </div>
                          <p>
                            {f.message}
                            {f.path ? ` — ${f.path}${f.line !== undefined ? `:${f.line}` : ''}` : ''}
                          </p>
                        </div>
                      ))}
                      {previewMore > 0 && (
                        <p className="page-note">{t('be.prevFindingsMore', { count: previewMore })}</p>
                      )}
                    </div>
                  )}
                  <div>
                    <span>{t('be.prevDest')}</span>
                    <code>{outDir.trim()}</code>
                  </div>
                  <div>
                    <span>
                      {t('be.prevPlan')} · {t('be.prevPlanBadge')}
                    </span>
                    <code>About/About.xml</code>
                    <code>{`Languages/${folderForm(st.targetLocale)}/…`}</code>
                    <p className="page-note">{t('be.prevPlanNote')}</p>
                  </div>
                </div>
              )}
            </div>
          )}

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
            !preview && <p className="page-note">{t('be.resultEmpty')}</p>
          )}

          {/* §8 F8.3: skipped_unknown_type is NEVER dropped — the types the
              schema did not recognize stay visible with the rescan/Checks
              hint, or a silent data loss reads as a clean run. */}
          {result && result.skipped.length > 0 && (
            <div className="inline-warning" data-testid="be.skipped">
              <span>
                {t('be.resSkipped', {
                  count: result.skipped.length,
                  list:
                    result.skipped.slice(0, PREVIEW_FINDINGS_LIMIT).join(', ') +
                    (result.skipped.length > PREVIEW_FINDINGS_LIMIT ? ', …' : ''),
                })}
              </span>
            </div>
          )}

          {result && (
            <button
              type="button"
              onClick={() => void revealOutput()}
              disabled={busy}
              data-testid="be.reveal"
            >
              <FolderOpen size={14} /> {t('be.reveal')}
            </button>
          )}
        </aside>
      </div>
    </div>
  )
}
