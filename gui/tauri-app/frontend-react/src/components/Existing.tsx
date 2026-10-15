// Existing translation (R1, journey J2): dry-run import_existing against
// the OPEN project → honest classification counts → guarded apply of the
// reusable set (persist-before-ack, conflicts never overwritten).
// UI-10: the same screen hosts the project-exchange PO — export the
// durable translations to a .po file for external CAT tools and analyze/
// apply a .po pack through the SAME review/apply flow as a folder pack.
import { useState } from 'react'
import { ArrowRight, Check, Download, FileText, FolderOpen, Upload } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { usePersistedInput } from '../lib/state/usePersistedInput'
import { projectStore } from '../lib/state/project'
import { contractErrorText } from '../lib/client/messagesError'
import { folderForm } from '../lib/languages/folderForm'
import { t } from '../lib/i18n'
import type { ImportExistingResponseDto } from '../lib/client/types'

export function Existing() {
  const st = useProjectState()
  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch
  const acked = st.snapshot?.acked_revision ?? st.snapshot?.revision ?? 0

  // D5: каталог переживает ремоунт экрана — sessionStorage по проекту.
  const [dir, setDir] = usePersistedInput(projectId ?? '', 'existing-dir')
  const [poPath, setPoPath] = usePersistedInput(projectId ?? '', 'existing-po-path')
  const [poOut, setPoOut] = usePersistedInput(projectId ?? '', 'existing-po-out')
  const [xliffPath, setXliffPath] = usePersistedInput(projectId ?? '', 'existing-xliff-path')
  const [xliffOut, setXliffOut] = usePersistedInput(projectId ?? '', 'existing-xliff-out')
  const [xliffStats, setXliffStats] = useState<{
    total: number
    translated: number
    out: string
  } | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [report, setReport] = useState<ImportExistingResponseDto | null>(null)
  const [applied, setApplied] = useState<number | null>(null)
  // Which pack source the CURRENT report came from — the apply call must
  // re-read the SAME source the user reviewed (folder dir or PO file).
  const [reportSource, setReportSource] = useState<'folder' | 'po' | 'xliff'>('folder')
  const [poStats, setPoStats] = useState<{ total: number; translated: number; out: string } | null>(
    null,
  )

  const pick = async (): Promise<void> => {
    setError(null)
    try {
      const d = await clientInstance.getClient().pickDirectory()
      if (d) setDir(d)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const pickPo = async (): Promise<void> => {
    setError(null)
    try {
      const f = await clientInstance.getClient().pickPoFile()
      if (f) setPoPath(f)
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
          // Пак кормит АКТИВНУЮ цель (strict folder form), не захардкоженный
          // 'Russian' — при uk-цели импорт Ukrainian-пака больше не лжёт.
          locale: folderForm(st.targetLocale),
        }),
      )
      setReportSource('folder')
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  const analyzePo = async (): Promise<void> => {
    if (!poPath.trim()) {
      setError(t('ex.needDir'))
      return
    }
    setBusy(true)
    setError(null)
    setApplied(null)
    try {
      setReport(
        await clientInstance.getClient().importExistingPo({
          project_id: projectId,
          session_epoch: epoch,
          po_path: { path: poPath.trim() },
          locale: folderForm(st.targetLocale),
        }),
      )
      setReportSource('po')
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  const analyzeXliff = async (): Promise<void> => {
    if (!xliffPath.trim()) {
      setError(t('ex.needDir'))
      return
    }
    setBusy(true)
    setError(null)
    setApplied(null)
    try {
      setReport(
        await clientInstance.getClient().importExistingXliff({
          project_id: projectId,
          session_epoch: epoch,
          xliff_path: { path: xliffPath.trim() },
          locale: folderForm(st.targetLocale),
        }),
      )
      setReportSource('xliff')
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  const exportXliff = async (): Promise<void> => {
    if (!xliffOut.trim()) {
      setError(t('ex.needDir'))
      return
    }
    setBusy(true)
    setError(null)
    try {
      const r = await clientInstance.getClient().exportProjectXliff({
        project_id: projectId,
        session_epoch: epoch,
        out_xliff: { path: xliffOut.trim() },
        locale: folderForm(st.targetLocale),
      })
      setXliffStats({ total: r.keys_total, translated: r.keys_translated, out: r.out_path })
    } catch (e) {
      setError(contractErrorText((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  const exportPo = async (): Promise<void> => {
    if (!poOut.trim()) {
      setError(t('ex.needDir'))
      return
    }
    setBusy(true)
    setError(null)
    try {
      const r = await clientInstance.getClient().exportProjectPo({
        project_id: projectId,
        session_epoch: epoch,
        out_po: { path: poOut.trim() },
        locale: folderForm(st.targetLocale),
      })
      setPoStats({ total: r.keys_total, translated: r.keys_translated, out: r.out_po })
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
      const r =
        reportSource === 'po'
          ? await clientInstance.getClient().applyExistingPo({
              project_id: projectId,
              expected_revision: acked,
              session_epoch: epoch,
              po_path: { path: poPath.trim() },
              locale: folderForm(st.targetLocale),
            })
          : reportSource === 'xliff'
          ? await clientInstance.getClient().applyExistingXliff({
              project_id: projectId,
              expected_revision: acked,
              session_epoch: epoch,
              xliff_path: { path: xliffPath.trim() },
              locale: folderForm(st.targetLocale),
            })
          : await clientInstance.getClient().applyExisting({
              project_id: projectId,
              expected_revision: acked,
              session_epoch: epoch,
              existing_dir: { path: dir.trim() },
              locale: folderForm(st.targetLocale),
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

      {/* UI-10: the project-exchange PO — export the durable translations
          for external CAT tools; import/apply a .po pack through the SAME
          review/apply flow as a folder pack. */}
      <div className="import-form" data-testid="ex.po-block">
        <label className="field">
          <span>{t('ex.poOut')}</span>
          <div className="out-dir-row">
            <input
              value={poOut}
              onChange={(e) => setPoOut(e.target.value)}
              placeholder={t('ex.poOutPlaceholder')}
              disabled={busy}
              data-testid="ex.po-out"
            />
          </div>
        </label>
        <div className="build-actions">
          <button disabled={busy || !poOut.trim()} onClick={() => void exportPo()} data-testid="ex.po-export">
            <Download /> {t('ex.poExport')}
          </button>
        </div>
        {poStats && (
          <div className="inline-success" role="status" data-testid="ex.po-exported">
            <Check /> {t('ex.poExported', { total: poStats.total, translated: poStats.translated })}{' '}
            <code>{poStats.out}</code>
          </div>
        )}
        <label className="field">
          <span>{t('ex.poIn')}</span>
          <div className="out-dir-row">
            <input
              value={poPath}
              onChange={(e) => setPoPath(e.target.value)}
              placeholder={t('ex.poInPlaceholder')}
              disabled={busy}
              data-testid="ex.po-path"
            />
            <button type="button" onClick={() => void pickPo()} disabled={busy} data-testid="ex.po-pick">
              <FileText /> {t('be.pick')}
            </button>
          </div>
        </label>
        <div className="build-actions">
          <button disabled={busy || !poPath.trim()} onClick={() => void analyzePo()} data-testid="ex.po-analyze">
            <Upload /> {t('ex.poAnalyze')}
          </button>
        </div>

        {/* XLIFF 1.2 mirror of the same exchange flow. */}
        <label className="field">
          <span>{t('ex.xliffOut')}</span>
          <div className="out-dir-row">
            <input
              value={xliffOut}
              onChange={(e) => setXliffOut(e.target.value)}
              placeholder={t('ex.poOutPlaceholder')}
              disabled={busy}
              data-testid="ex.xliff-out"
            />
          </div>
        </label>
        <div className="build-actions">
          <button disabled={busy || !xliffOut.trim()} onClick={() => void exportXliff()} data-testid="ex.xliff-export">
            <Download /> {t('ex.xliffExport')}
          </button>
        </div>
        {xliffStats && (
          <div className="inline-success" role="status" data-testid="ex.xliff-exported">
            <Check />{' '}
            {t('ex.poExported', { total: xliffStats.total, translated: xliffStats.translated })}{' '}
            <code>{xliffStats.out}</code>
          </div>
        )}
        <label className="field">
          <span>{t('ex.xliffIn')}</span>
          <div className="out-dir-row">
            <input
              value={xliffPath}
              onChange={(e) => setXliffPath(e.target.value)}
              placeholder={t('ex.poInPlaceholder')}
              disabled={busy}
              data-testid="ex.xliff-path"
            />
          </div>
        </label>
        <div className="build-actions">
          <button disabled={busy || !xliffPath.trim()} onClick={() => void analyzeXliff()} data-testid="ex.xliff-analyze">
            <Upload /> {t('ex.xliffAnalyze')}
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
