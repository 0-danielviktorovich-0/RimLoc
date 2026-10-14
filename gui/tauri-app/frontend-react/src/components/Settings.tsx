// Settings (R1): appearance (theme + UI language arrive from the app root as
// props — ONE source of truth, audit v2 #4; a locale change re-renders the
// whole tree immediately, audit v2 #5) and an HONEST capability table
// straight from the handshake report (LIVE vs unsupported — §42).
// The «project defaults» section is REMOVED (audit v2 #3): its local
// version/target state had no persistence and no consumers — a project is
// created with its settings in the wizard and its target changes live in
// the workspace switcher.
// Theme is a THREE-way preference ('light'|'dark'|'system'): 'system'
// follows the OS prefers-color-scheme live; the effective dark flag is
// derived in the app root — this screen only edits the preference.
import { useEffect, useState } from 'react'
import { clientInstance } from '../lib/client/instance'
import { t, type Locale } from '../lib/i18n'

export type ThemePref = 'system' | 'light' | 'dark'

/** UI zoom (rel27): 0.8–1.4, persisted; applied on the app shell root. */
export function readUiZoom(): number {
  const raw = window.localStorage.getItem('rimloc.ui.zoom')
  const n = raw ? Number(raw) : 1
  return Number.isFinite(n) && n >= 0.8 && n <= 1.4 ? n : 1
}

export function writeUiZoom(z: number): void {
  const clamped = Math.min(1.4, Math.max(0.8, z))
  window.localStorage.setItem('rimloc.ui.zoom', String(clamped))
  window.dispatchEvent(new CustomEvent('rimloc.ui.zoom'))
}

export function Settings(props: {
  theme: ThemePref
  onThemeChange: (v: ThemePref) => void
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
          <select value={props.theme} onChange={(e) => props.onThemeChange(e.target.value as ThemePref)} data-testid="settings.theme">
            <option value="light">{t('settings.light')}</option>
            <option value="dark">{t('settings.dark')}</option>
            <option value="system">{t('settings.system')}</option>
          </select>
        </label>
        <label className="field">
          <span>{t('settings.uiLanguage')}</span>
          <select value={props.locale} onChange={(e) => props.onLocaleChange(e.target.value as Locale)} data-testid="settings.ui-locale">
            <option value="ru">Русский</option>
            <option value="en">English</option>
          </select>
        </label>
        <label className="field">
          <span>{t('settings.uiZoom')}</span>
          <select
            value={String(readUiZoom())}
            onChange={(e) => writeUiZoom(Number(e.target.value))}
            data-testid="settings.ui-zoom"
          >
            <option value="0.8">80%</option>
            <option value="0.9">90%</option>
            <option value="1">100%</option>
            <option value="1.1">110%</option>
            <option value="1.2">120%</option>
            <option value="1.4">140%</option>
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

      <section className="settings-section">
        <h3>{t('nav.about')}</h3>
        <div className="capability-row">
          <span>{t('settings.aboutLink')}</span>
          <a href="#/about" data-testid="settings.about-link">
            {t('nav.about')} →
          </a>
        </div>
      </section>
    </div>
  )
}
