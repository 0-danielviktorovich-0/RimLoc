// New Translation wizard (R1, journey J1): real source folder via the
// native picker → target version → create → editor. The Rust scan IS the
// preview (the returned snapshot carries the live inventory); no fake
// scanning timers. Users never see DefInjected/Keyed unless Advanced (§22).
import { useState } from 'react'
import { ArrowRight, Check, FolderOpen, LoaderCircle, Plus } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { projectStore } from '../lib/state/project'
import { BUILTIN_LANGUAGES } from '../lib/languages/registry'
import { t } from '../lib/i18n'

export function NewProjectWizard({ onCreated }: { onCreated: () => void }) {
  const [open, setOpen] = useState(false)
  const [step, setStep] = useState(0)
  const [modRoot, setModRoot] = useState('')
  const [version, setVersion] = useState('1.6')
  // §22: the target language is chosen in the wizard, not silently 'ru'.
  // The project_create contract carries no target-locale param (translations
  // start empty); the chosen locale designates the editing target through
  // the store's §32 multi-target state right after the live scan snapshot
  // is adopted — the same switcher the workspace uses.
  const [target, setTarget] = useState('ru')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const targetName = BUILTIN_LANGUAGES.find((l) => l.localeId === target)?.nativeName ?? target

  const pick = async (): Promise<void> => {
    setError(null)
    try {
      const dir = await clientInstance.getClient().pickDirectory()
      if (dir) {
        setModRoot(dir)
        setStep(1)
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const create = async (): Promise<void> => {
    setBusy(true)
    setError(null)
    try {
      const ok = await projectStore.createContractProject(modRoot, version)
      if (!ok) throw new Error(projectStore.lastErrorText ?? 'create failed')
      // §22: the wizard's target choice becomes the workspace editing target
      // (no-op when it equals the current one; the snapshot already exists).
      projectStore.setTargetLocale(target)
      setBusy(false)
      setOpen(false)
      onCreated()
    } catch (e) {
      setBusy(false)
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  if (!open) {
    return (
      <button className="new-project-card" onClick={() => setOpen(true)} data-testid="wizard.open">
        <Plus size={28} /> <strong>{t('wizard.open')}</strong> <span>{t('wizard.openHint')}</span>
      </button>
    )
  }

  return (
    <div className="wizard-overlay" role="dialog" aria-modal="true" aria-label={t('wizard.title')} data-testid="wizard.overlay">
      <div className="rim-dialog">
        <div className="dialog-header">
          <span className="eyebrow">
            {t('wizard.eyebrow')} · {step + 1} / 3
          </span>
          <h2>{[t('wizard.step1'), t('wizard.step2'), t('wizard.step3')][step]}</h2>
          <p>{[t('wizard.step1Body'), t('wizard.step2Body'), t('wizard.step3Body')][step]}</p>
        </div>
        <div className="wizard-dots">
          {[0, 1, 2].map((i) => (
            <div key={i} className={i <= step ? 'done' : ''} />
          ))}
        </div>

        {step === 0 && (
          <>
            <label className="file-drop">
              <FolderOpen size={28} />
              <strong>{modRoot || t('wizard.pickFolder')}</strong>
              <small>{t('wizard.pickFolderHint')}</small>
              <button type="button" onClick={() => void pick()} data-testid="wizard.pick">
                <FolderOpen /> {t('be.pick')}
              </button>
            </label>
            <label className="field">
              <span>{t('wizard.pathLabel')}</span>
              <input
                value={modRoot}
                onChange={(e) => setModRoot(e.target.value)}
                placeholder={t('wizard.pathPlaceholder')}
                disabled={busy}
                data-testid="wizard.path-input"
              />
            </label>
            {error && (
              <div className="inline-warning" role="alert">
                <span>{error}</span>
              </div>
            )}
          </>
        )}
        {step === 1 && (
          <div className="form-grid">
            <label className="field">
              <span>{t('wizard.sourceLang')}</span>
              <select defaultValue="en" disabled>
                <option value="en">English</option>
              </select>
            </label>
            <label className="field">
              <span>{t('wizard.targetLang')}</span>
              <select value={target} onChange={(e) => setTarget(e.target.value)} data-testid="wizard.target-locale">
                {BUILTIN_LANGUAGES.map((l) => (
                  <option key={l.localeId} value={l.localeId}>
                    {l.nativeName}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>{t('ws.gameVersion')}</span>
              <select value={version} onChange={(e) => setVersion(e.target.value)} data-testid="wizard.version">
                <option>1.6</option>
                <option>1.5</option>
                <option>1.4</option>
              </select>
            </label>
          </div>
        )}
        {step === 2 && (
          <div className="wizard-summary">
            <div className="success-emblem">
              <Check />
            </div>
            <h3>{modRoot.split('/').filter(Boolean).at(-1) ?? t('wizard.newProject')}</h3>
            <p>
              English → {targetName} · RimWorld {version}
            </p>
            <code>{modRoot}</code>
            <small>{t('wizard.summaryNote')}</small>
          </div>
        )}

        <div className="dialog-actions">
          {step > 0 ? (
            <button onClick={() => setStep(step - 1)} disabled={busy}>
              {t('wizard.back')}
            </button>
          ) : (
            <button onClick={() => setOpen(false)} disabled={busy}>
              {t('wizard.cancel')}
            </button>
          )}
          <button
            className="btn-primary"
            disabled={busy || (step === 0 && !modRoot)}
            onClick={() => (step === 2 ? void create() : setStep(step + 1))}
            data-testid="wizard.next"
          >
            {busy ? <LoaderCircle className="spin" size={14} /> : step === 2 ? <Check /> : <ArrowRight />}
            {step === 2 ? t('wizard.create') : t('wizard.next')}
          </button>
        </div>
      </div>
    </div>
  )
}

