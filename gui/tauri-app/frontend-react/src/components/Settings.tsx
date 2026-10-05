// Settings (R1): appearance (theme/locale from the app state), project
// defaults (version + target from the registry), and an HONEST capability
// table straight from the handshake report (LIVE vs unsupported — §42).
import { useEffect, useState } from 'react'
import { clientInstance } from '../lib/client/instance'
import { BUILTIN_LANGUAGES } from '../lib/languages/registry'
import { setLocale, getLocale, t, type Locale } from '../lib/i18n'

export function Settings() {
  const [dark, setDark] = useState(document.documentElement.classList.contains('dark'))
  const [locale, setUiLocale] = useState<Locale>(getLocale())
  const [version, setVersion] = useState('1.6')
  const [target, setTarget] = useState('ru')
  const [capabilities, setCapabilities] = useState<{ supported: string[]; unsupported: { capability: string; reason: string }[] } | null>(null)

  useEffect(() => {
    let cancelled = false
    clientInstance
      .getClient()
      .handshake()
      .then((hs) => {
        if (!cancelled) setCapabilities(hs.capabilities)
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [])

  const applyDark = (v: boolean): void => {
    setDark(v)
    document.documentElement.classList.toggle('dark', v)
  }
  const applyLocale = (v: Locale): void => {
    setUiLocale(v)
    setLocale(v)
    // React strings re-render via t() on next mount; a full re-render pass
    // happens on route change — recorded as a known transient (Phase E:
    // locale context provider).
  }

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('settings.eyebrow')}</span>
          <h2>{t('settings.title')}</h2>
          <p>{t('settings.subtitle')}</p>
        </div>
      </div>

      <section className="settings-section">
        <h3>{t('settings.appearance')}</h3>
        <label className="field">
          <span>{t('settings.theme')}</span>
          <select value={dark ? 'dark' : 'light'} onChange={(e) => applyDark(e.target.value === 'dark')} data-testid="settings.theme">
            <option value="light">{t('settings.light')}</option>
            <option value="dark">{t('settings.dark')}</option>
          </select>
        </label>
        <label className="field">
          <span>{t('settings.uiLanguage')}</span>
          <select value={locale} onChange={(e) => applyLocale(e.target.value as Locale)} data-testid="settings.ui-locale">
            <option value="ru">Русский</option>
            <option value="en">English</option>
          </select>
        </label>
      </section>

      <section className="settings-section">
        <h3>{t('settings.projectDefaults')}</h3>
        <div className="form-grid">
          <label className="field">
            <span>{t('ws.gameVersion')}</span>
            <select value={version} onChange={(e) => setVersion(e.target.value)}>
              <option>1.6</option>
              <option>1.5</option>
              <option>1.4</option>
            </select>
          </label>
          <label className="field">
            <span>{t('settings.defaultTarget')}</span>
            <select value={target} onChange={(e) => setTarget(e.target.value)}>
              {BUILTIN_LANGUAGES.filter((l) => l.localeId !== 'en' && l.capabilities.translation).map((l) => (
                <option key={l.localeId} value={l.localeId}>
                  {l.nativeName}
                </option>
              ))}
            </select>
          </label>
        </div>
      </section>

      <section className="settings-section">
        <h3>{t('settings.capabilities')}</h3>
        {!capabilities ? (
          <p className="page-note">{t('settings.capabilitiesLoading')}</p>
        ) : (
          <>
            <div className="capability-row">
              <span>{t('settings.capabilitiesCount')}</span>
              <strong className="text-success">{capabilities.supported.length}</strong>
            </div>
            {capabilities.unsupported.map((u) => (
              <div key={u.capability} className="capability-row">
                <span>{u.capability}</span>
                <strong className="text-warning">{t('settings.unsupported')}</strong>
              </div>
            ))}
          </>
        )}
      </section>
    </div>
  )
}
