// Compare (version diff, journey J-version): read-only source-inventory
// diff of TWO mod roots — new / missing / changed / unchanged per key with
// the carrying file per side. Stateless: unlike Existing, no project
// session is involved — the contract_version_diff command scans both trees
// and classifies; nothing is ever written.
import { useState } from 'react'
import { Check, FolderOpen, GitCompareArrows } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { contractErrorText } from '../lib/client/messagesError'
import { t } from '../lib/i18n'
import type { VersionDiffEntryDto, VersionDiffResponseDto } from '../lib/client/types'

/** Category chip class per wire category (counts carry the same colors). */
const catChipClass: Record<VersionDiffEntryDto['category'], string> = {
  changed: 'text-warning',
  new: 'text-info',
  missing: 'text-muted-foreground',
  unchanged: 'text-success',
}

export function Compare() {
  const [oldDir, setOldDir] = useState('')
  const [newDir, setNewDir] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [report, setReport] = useState<VersionDiffResponseDto | null>(null)

  const pick = async (side: 'old' | 'new'): Promise<void> => {
    setError(null)
    try {
      const d = await clientInstance.getClient().pickDirectory()
      if (!d) return
      if (side === 'old') setOldDir(d)
      else setNewDir(d)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const run = async (): Promise<void> => {
    if (!oldDir.trim() || !newDir.trim()) {
      setError(t('cmp.needDirs'))
      return
    }
    setBusy(true)
    setError(null)
    try {
      setReport(
        await clientInstance.getClient().versionDiff({
          old_root: { path: oldDir.trim() },
          new_root: { path: newDir.trim() },
        }),
      )
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
          <span className="eyebrow">{t('cmp.eyebrow')}</span>
          <h2>{t('cmp.title')}</h2>
          <p>{t('cmp.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="cmp.error">
          <span>{error}</span>
        </div>
      )}

      <div className="import-form">
        <label className="field">
          <span>{t('cmp.oldDir')}</span>
          <div className="out-dir-row">
            <input
              value={oldDir}
              onChange={(e) => setOldDir(e.target.value)}
              placeholder={t('cmp.oldDirPlaceholder')}
              disabled={busy}
              data-testid="cmp.old-dir"
            />
            <button type="button" onClick={() => void pick('old')} disabled={busy} data-testid="cmp.old-pick">
              <FolderOpen /> {t('be.pick')}
            </button>
          </div>
        </label>
        <label className="field">
          <span>{t('cmp.newDir')}</span>
          <div className="out-dir-row">
            <input
              value={newDir}
              onChange={(e) => setNewDir(e.target.value)}
              placeholder={t('cmp.newDirPlaceholder')}
              disabled={busy}
              data-testid="cmp.new-dir"
            />
            <button type="button" onClick={() => void pick('new')} disabled={busy} data-testid="cmp.new-pick">
              <FolderOpen /> {t('be.pick')}
            </button>
          </div>
        </label>
        <div className="build-actions">
          <button
            className="btn-primary"
            disabled={busy || !oldDir.trim() || !newDir.trim()}
            onClick={() => void run()}
            data-testid="cmp.run"
          >
            <GitCompareArrows /> {t('cmp.run')}
          </button>
        </div>
      </div>

      {report && (
        <>
          <div className="metrics-band" data-testid="cmp.metrics">
            <div>
              <strong className="text-warning">{report.changed}</strong>
              <span>{t('cmp.changed')}</span>
            </div>
            <div>
              <strong className="text-info">{report.new}</strong>
              <span>{t('cmp.new')}</span>
            </div>
            <div>
              <strong className="text-muted-foreground">{report.missing}</strong>
              <span>{t('cmp.missing')}</span>
            </div>
            <div>
              <strong className="text-success">{report.unchanged}</strong>
              <span>{t('cmp.unchanged')}</span>
            </div>
          </div>

          {report.entries.length === 0 ? (
            <div className="empty-state">
              <Check />
              <p>{t('cmp.empty')}</p>
            </div>
          ) : (
            <div className="glossary-table cmp-table" data-testid="cmp.table">
              <div className="glossary-head">
                <span>{t('cmp.head.category')}</span>
                <span>{t('cmp.head.key')}</span>
                <span>{t('cmp.head.oldFile')}</span>
                <span>{t('cmp.head.newFile')}</span>
              </div>
              {report.entries.map((e) => (
                <div key={`${e.category}:${e.key}`}>
                  <span className={catChipClass[e.category]}>{t(`cmp.cat.${e.category}`)}</span>
                  <span>
                    <code>{e.key}</code>
                    {e.category === 'changed' && (e.old_source || e.new_source) && (
                      <small className="cmp-source-line">
                        {e.old_source ?? '—'} → {e.new_source ?? '—'}
                      </small>
                    )}
                  </span>
                  <code className="cmp-path">{e.old_path ?? '—'}</code>
                  <code className="cmp-path">{e.new_path ?? '—'}</code>
                </div>
              ))}
            </div>
          )}

          {report.entries_truncated && (
            <p className="page-note" data-testid="cmp.truncated">
              {t('cmp.truncated', { shown: report.entries.length, total: report.changed + report.new + report.missing + report.unchanged })}
            </p>
          )}
          <p className="page-note">
            <Check size={15} /> {t('cmp.safetyNote')}
          </p>
        </>
      )}
    </div>
  )
}
