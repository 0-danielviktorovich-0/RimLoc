// Checks (R1): real validator over the open project (project_validate).
// Counts/findings come from the backend only — no fake clean state (§35).
import { useEffect, useState } from 'react'
import { ShieldCheck, CircleAlert, ArrowRight, Download } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { contractErrorText } from '../lib/client/messagesError'
import { t } from '../lib/i18n'
import type { ValidateProjectResponseDto, ValidationFindingDto } from '../lib/client/types'

type Filter = 'all' | 'error' | 'warning' | 'info'

export function Checks() {
  const st = useProjectState()
  const [report, setReport] = useState<ValidateProjectResponseDto | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [filter, setFilter] = useState<Filter>('all')

  const projectId = st.snapshot?.project_id
  const epoch = st.snapshot?.session_epoch

  useEffect(() => {
    if (!projectId || epoch === undefined) return
    let cancelled = false
    setBusy(true)
    setError(null)
    clientInstance
      .getClient()
      .validateProject(projectId, epoch)
      .then((r) => {
        if (!cancelled) setReport(r)
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setError(
            contractErrorText(
              (e as { code?: string }).code ?? 'internal',
              e instanceof Error ? e.message : String(e),
            ),
          )
        }
      })
      .finally(() => {
        if (!cancelled) setBusy(false)
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
            <span className="eyebrow">{t('checks.eyebrow')}</span>
            <h2>{t('checks.title')}</h2>
            <p>{t('checks.needProject')}</p>
          </div>
        </div>
        <a className="btn-primary" href="#/home">
          {t('ws.backHome')}
        </a>
      </div>
    )
  }

  const findings = report?.findings ?? []
  const shown = findings.filter((f) => filter === 'all' || f.severity === filter)
  const errors = findings.filter((f) => f.severity === 'error').length
  const warnings = findings.filter((f) => f.severity === 'warning').length
  const total = st.entries.length || 1

  // §35: a finding opens in the editor — Workspace + select by the finding's
  // key (§9 mapping: the full identity key when present, the raw key
  // otherwise). Only findings whose key resolves to a workspace entry are
  // actionable; project-level findings stay non-clickable — a navigation to
  // nothing would be a dead affordance of the kind this wave removes.
  const resolveFinding = (f: ValidationFindingDto): string | null => {
    const key = f.id?.key ?? f.key
    if (!key) return null
    return st.entries.some((e) => e.key === key) ? key : null
  }
  const openFinding = (key: string): void => {
    projectStore.select(key)
    window.location.hash = '#/workspace'
  }

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('checks.eyebrow')}</span>
          <h2>{t('checks.title')}</h2>
          <p>{t('checks.subtitle')}</p>
        </div>
        <button
          disabled={busy}
          data-testid="checks.rerun"
          onClick={() => {
            setReport(null)
            // Re-trigger by bumping a nonce through the same effect inputs.
            setFilter((f) => f)
            void (async () => {
              setBusy(true)
              try {
                setReport(await clientInstance.getClient().validateProject(projectId, epoch))
              } catch (e) {
                setError(e instanceof Error ? e.message : String(e))
              } finally {
                setBusy(false)
              }
            })()
          }}
        >
          <ShieldCheck /> {t('checks.rerun')}
        </button>
      </div>

      {error && (
        <div className="inline-warning" role="alert">
          <span>{error}</span>
        </div>
      )}

      <div className="metrics-band">
        <div>
          <strong className="text-destructive">{errors}</strong>
          <span>{t('checks.errors')}</span>
        </div>
        <div>
          <strong className="text-warning">{warnings}</strong>
          <span>{t('checks.warnings')}</span>
        </div>
        <div>
          <strong className="text-success">{report?.info_count ?? 0}</strong>
          <span>{t('checks.infos')}</span>
        </div>
        <div>
          <strong>
            {Math.round(((total - errors) / total) * 100)}%
          </strong>
          <span>{t('checks.coverage')}</span>
        </div>
      </div>

      <div className="section-tabs">
        {(['all', 'error', 'warning', 'info'] as const).map((f) => (
          <button key={f} className={filter === f ? 'active' : ''} onClick={() => setFilter(f)}>
            {t(`checks.filter.${f}`)}
          </button>
        ))}
        <button
          className="ml-auto"
          disabled={!report}
          data-testid="checks.report"
          onClick={() => {
            if (!report) return
            const blob = JSON.stringify(report, null, 2)
            const a = document.createElement('a')
            a.href = URL.createObjectURL(new Blob([blob], { type: 'application/json' }))
            a.download = 'rimloc-validation.json'
            a.click()
          }}
        >
          <Download size={14} /> {t('checks.report')}
        </button>
      </div>

      <div className="finding-list" data-testid="checks.findings">
        {shown.map((f, i) => {
          const fixKey = resolveFinding(f)
          const row = (
            <>
              <CircleAlert className={f.severity === 'error' ? 'text-destructive' : 'text-warning'} size={19} />
              <div>
                <strong>{f.kind}</strong>
                <code>{f.id?.key ?? f.key}</code>
              </div>
              <p>{f.message}</p>
              {fixKey && (
                <span
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 5,
                    fontSize: 9,
                    color: 'var(--primary)',
                    whiteSpace: 'nowrap',
                  }}
                >
                  <ArrowRight size={12} /> {t('checks.fix')}
                </span>
              )}
            </>
          )
          return fixKey ? (
            <button
              key={`${f.id?.key ?? 'root'}-${i}`}
              type="button"
              className="finding-row"
              data-testid="checks.finding-fix"
              onClick={() => openFinding(fixKey)}
              style={{ width: '100%', textAlign: 'left', cursor: 'pointer', font: 'inherit', color: 'inherit', background: 'transparent', border: 0, borderBottom: '1px solid var(--border)' }}
            >
              {row}
            </button>
          ) : (
            <div key={`${f.id?.key ?? 'root'}-${i}`} className="finding-row">
              {row}
            </div>
          )
        })}
        {shown.length === 0 && report && (
          <div className="passed-state">
            <ShieldCheck size={45} />
            <h3>{t('checks.allClean')}</h3>
            <p>{t('checks.allCleanBody')}</p>
          </div>
        )}
        {!report && !error && <p className="page-note">{t('checks.running')}</p>}
      </div>
      <div className="project-storage">
        <ArrowRight />
        <span>{t('checks.fixHint')}</span>
      </div>
    </div>
  )
}
