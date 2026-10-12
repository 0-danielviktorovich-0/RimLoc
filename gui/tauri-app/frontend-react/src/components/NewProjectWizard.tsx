// New Translation wizard (R1, journey J1): real source folder via the
// native picker → target version → create → editor. The Rust scan IS the
// preview (the returned snapshot carries the live inventory); no fake
// scanning timers. Users never see DefInjected/Keyed unless Advanced (§22).
//
// Nav split: the wizard is invoked GLOBALLY — the topbar «Новый проект»
// button (any section), the library card and the onboarding strip all open
// THE SAME instance mounted in App. Pass `open` + `onOpenChange` for that
// controlled mode: the open state lives in App, closing (cancel/Escape)
// simply hides the overlay — the user stays in the section they invoked the
// wizard from (no route change ever happens). Without the props the wizard
// keeps the legacy self-contained mode (own trigger card + state).
import { useEffect, useState } from 'react'
import { ArrowRight, Check, FolderOpen, LoaderCircle, Plus } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { projectStore } from '../lib/state/project'
import { BUILTIN_LANGUAGES, SOURCE_LOCALE } from '../lib/languages/registry'
import { folderForm } from '../lib/languages/folderForm'
import { useLanguageManager } from '../lib/languages/manager'
import { t } from '../lib/i18n'

export function NewProjectWizard({
  onCreated,
  open: openProp,
  onOpenChange,
}: {
  onCreated: () => void
  /** Controlled mode (global invocation): App owns the open state. */
  open?: boolean
  onOpenChange?: (open: boolean) => void
}) {
  const [selfOpen, setSelfOpen] = useState(false)
  const controlled = openProp !== undefined && onOpenChange !== undefined
  const open = openProp ?? selfOpen
  const setOpen = (v: boolean): void => {
    if (openProp !== undefined && onOpenChange !== undefined) onOpenChange(v)
    else setSelfOpen(v)
  }
  const [step, setStep] = useState(0)
  const [modRoot, setModRoot] = useState('')
  const [version, setVersion] = useState('1.6')
  // §22: the target language is chosen in the wizard, not silently 'ru'.
  // The project_create contract carries no target-locale param (translations
  // start empty); the chosen locale designates the editing target through
  // the store's §32 multi-target state right after the live scan snapshot
  // is adopted — the same switcher the workspace uses.
  const [target, setTarget] = useState('ru')
  // Source language (source-language wave): selectable from the SAME option
  // table the Language Manager renders (builtins + user languages), not a
  // disabled English-only stub. The choice rides the project_create contract
  // (source_locale + strict folder form) so the backend scans
  // Languages/<source_dir> as the source inventory — the UI and the backend
  // can never disagree about the source side.
  const [source, setSource] = useState(SOURCE_LOCALE)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // Та же опция-таблица, что у Language Manager: builtins + user languages
  // (manager store). nativeName labels match the manager rows 1:1.
  const { allLanguages } = useLanguageManager()
  const targetName = BUILTIN_LANGUAGES.find((l) => l.localeId === target)?.nativeName ?? target
  const sourceName = allLanguages.find((l) => l.localeId === source)?.nativeName ?? source

  // D4: отмена (и успешное создание) сбрасывают ВСЁ локальное состояние —
  // иначе следующий запуск визарда открывается на старом пути/шаге, будто
  // отмена ничего не означала. В controlled-режиме закрытие возвращает
  // пользователя в исходный раздел: оверлей просто скрывается.
  const resetWizard = (): void => {
    setOpen(false)
    setStep(0)
    setModRoot('')
    setVersion('1.6')
    setTarget('ru')
    setSource(SOURCE_LOCALE)
    setBusy(false)
    setError(null)
  }

  // D6: Escape закрывает визард той же логикой отмены, что и кнопка
  // (полный reset). Во время busy — как кнопка Cancel, заблокирован.
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === 'Escape' && !busy) resetWizard()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
    // resetWizard замыкает только stable-сеттеры состояния.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, busy])

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
      // Source rides the CONTRACT explicitly (locale id + strict folder
      // form): no situation where the UI shows a Japanese source while the
      // backend scans English.
      const ok = await projectStore.createContractProject(modRoot, version, {
        locale: source,
        dir: folderForm(source),
      })
      if (!ok) throw new Error(projectStore.lastErrorText ?? 'create failed')
      // §22: the wizard's target choice becomes the workspace editing target
      // (no-op when it equals the current one; the snapshot already exists).
      projectStore.setTargetLocale(target)
      setBusy(false)
      resetWizard()
      onCreated()
    } catch (e) {
      setBusy(false)
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  if (!open) {
    // Controlled (global) mode: the trigger cards live in the topbar button
    // and the library screen — a closed wizard renders nothing.
    if (controlled) return null
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
              <select
                value={source}
                onChange={(e) => setSource(e.target.value)}
                data-testid="wizard.source-locale"
              >
                {allLanguages.map((l) => (
                  <option key={l.localeId} value={l.localeId}>
                    {l.nativeName}
                  </option>
                ))}
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
              {sourceName} → {targetName} · RimWorld {version}
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
            <button onClick={resetWizard} disabled={busy}>
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

