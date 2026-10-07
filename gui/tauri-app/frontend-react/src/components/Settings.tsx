// Settings (R1): appearance (theme + UI language arrive from the app root as
// props — ONE source of truth, audit v2 #4; a locale change re-renders the
// whole tree immediately, audit v2 #5) and an HONEST capability table
// straight from the handshake report (LIVE vs unsupported — §42).
// The «project defaults» section is REMOVED (audit v2 #3): its local
// version/target state had no persistence and no consumers — a project is
// created with its settings in the wizard and its target changes live in
// the workspace switcher.
import { useEffect, useState } from 'react'
import { clientInstance } from '../lib/client/instance'
import { t, type Locale } from '../lib/i18n'

export function Settings(props: {
  dark: boolean
  onDarkChange: (v: boolean) => void
  locale: Locale
  onLocaleChange: (v: Locale) => void
}) {
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
          <select value={props.dark ? 'dark' : 'light'} onChange={(e) => props.onDarkChange(e.target.value === 'dark')} data-testid="settings.theme">
            <option value="light">{t('settings.light')}</option>
            <option value="dark">{t('settings.dark')}</option>
          </select>
        </label>
        <label className="field">
          <span>{t('settings.uiLanguage')}</span>
          <select value={props.locale} onChange={(e) => props.onLocaleChange(e.target.value as Locale)} data-testid="settings.ui-locale">
            <option value="ru">Русский</option>
            <option value="en">English</option>
          </select>
        </label>
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
