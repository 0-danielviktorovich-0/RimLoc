// About (Settings zone): the product hero, the four verified actions
// (Documentation / GitHub / Report an issue / Support RimLoc), the
// creator-support panel and the HONEST technical identity.
//
// Data-safety and honesty contract (owner addendum Oct-15):
//  - every external link rides `open_external_url` — the backend opens
//    ONLY https destinations from the verified allowlist, in the OS
//    browser; no in-app checkout, no webview forms, no clipboard actions;
//  - the build identity (version / source commit / profile / features)
//    comes from the RUNNING binary via build_identity and is preserved
//    verbatim, including the dirty-tree marker;
//  - the updater section states the AUTO_UPDATE_DESIGN fact of THIS build:
//    the Tauri 2 updater is designed but NOT configured here — no fake
//    check button, only the manual releases route;
//  - support copy never promises perks that do not exist and never
//    represents a donation as legally segregated project funding.
import { useEffect, useState } from 'react'
import { BookOpen, Code2, CircleAlert, Heart, ExternalLink } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import type { BuildIdentityDto } from '../lib/client/types'
import { t } from '../lib/i18n'

/** Canonical project home (mkdocs repo_url / README badges). Docs live
 *  in-repo (mkdocs.yml site_url is environment-gated, empty by default). */
export const REPO_URL = 'https://github.com/0-danielviktorovich-0/RimLoc'
const DOCS_URL = 'https://0-danielviktorovich-0.github.io/RimLoc/'
const ISSUES_URL = `${REPO_URL}/issues`
const RELEASES_URL = `${REPO_URL}/releases`
const LICENSE_URL = `${REPO_URL}/blob/main/LICENSE`
/** Verified support destinations — the SAME list the backend allowlist
 *  enforces (open_external_url). One creator account per platform,
 *  contextual RimLoc presentation. */
export const SPONSORS_URL = 'https://github.com/sponsors/0-danielviktorovich-0'
export const BMC_URL = 'https://buymeacoffee.com/danielviktorovich'
export const KOFI_URL = 'https://ko-fi.com/danielviktorovich'

export function About() {
  const [identity, setIdentity] = useState<BuildIdentityDto | null>(null)
  const [failed, setFailed] = useState(false)
  const [techOpen, setTechOpen] = useState(false)
  const [openError, setOpenError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    clientInstance
      .getClient()
      .buildIdentity()
      .then((id) => {
        if (!cancelled) setIdentity(id)
      })
      .catch(() => {
        if (!cancelled) setFailed(true)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const open = (url: string): void => {
    clientInstance
      .getClient()
      .openExternalUrl(url)
      .catch((e: unknown) => setOpenError(e instanceof Error ? e.message : String(e)))
  }

  const actions: { url: string; icon: typeof BookOpen; label: string; testid: string }[] = [
    { url: DOCS_URL, icon: BookOpen, label: t('about.action.docs'), testid: 'about.action.docs' },
    { url: REPO_URL, icon: Code2, label: t('about.action.github'), testid: 'about.action.github' },
    { url: ISSUES_URL, icon: CircleAlert, label: t('about.action.issues'), testid: 'about.action.issues' },
    { url: BMC_URL, icon: Heart, label: t('about.action.support'), testid: 'about.action.support' },
  ]
  const supportOptions: { url: string; label: string; note: string; testid: string }[] = [
    { url: SPONSORS_URL, label: 'GitHub Sponsors', note: t('about.support.monthly'), testid: 'about.support.sponsors' },
    { url: BMC_URL, label: 'Buy Me a Coffee', note: t('about.support.oneoff'), testid: 'about.support.bmc' },
    { url: KOFI_URL, label: 'Ko-fi', note: t('about.support.oneoff'), testid: 'about.support.kofi' },
  ]

  return (
    <div className="page-content narrow-page">
      {/* Hero */}
      <div className="about-hero" data-testid="about.hero">
        <img src="/rimloc-icon.png" alt="" className="about-hero-icon" aria-hidden />
        <div>
          <span className="eyebrow">{t('about.eyebrow')}</span>
          <h2>RimLoc</h2>
          <p>{t('about.heroTagline')}</p>
          <p className="about-hero-note">{t('about.heroPositioning')}</p>
        </div>
      </div>

      {/* Four verified actions */}
      <div className="about-actions" data-testid="about.actions">
        {actions.map(({ url, icon: Icon, label, testid }) => (
          <button
            key={testid}
            className="about-action"
            data-testid={testid}
            onClick={() => open(url)}
          >
            <Icon size={15} />
            <span>{label}</span>
            <ExternalLink size={11} className="about-action-ext" aria-hidden />
          </button>
        ))}
      </div>
      {openError && (
        <div className="inline-warning" role="alert" data-testid="about.open-error">
          {openError}
        </div>
      )}

      {/* Support panel */}
      <section className="settings-section about-support" data-testid="about.support">
        <h3>
          <Heart size={14} className="text-warning" aria-hidden /> {t('about.support.title')}
        </h3>
        <p className="page-note">{t('about.support.purpose')}</p>
        <div className="about-support-grid">
          {supportOptions.map(({ url, label, note, testid }) => (
            <button key={testid} className="about-support-card" data-testid={testid} onClick={() => open(url)}>
              <strong>{label}</strong>
              <small>{note}</small>
            </button>
          ))}
        </div>
        <p className="page-note">{t('about.support.honestNote')}</p>
      </section>

      {/* Foldable technical details — every prior truthful block preserved */}
      <section className="settings-section">
        <button
          className="about-tech-toggle"
          data-testid="about.tech-toggle"
          aria-expanded={techOpen}
          onClick={() => setTechOpen(!techOpen)}
        >
          {t('about.techTitle')} {techOpen ? '▾' : '▸'}
        </button>
        {techOpen && (
          <>
            <h3>{t('about.build')}</h3>
            {!identity && !failed && <p className="page-note">{t('about.loading')}</p>}
            {failed && <p className="page-note">{t('about.noBridge')}</p>}
            {identity && (
              <>
                <div className="capability-row" data-testid="about.version">
                  <span>{t('about.version')}</span>
                  <strong className="text-success">{identity.appVersion}</strong>
                </div>
                <div className="capability-row" data-testid="about.commit">
                  <span>{t('about.commit')}</span>
                  <strong>
                    <code>{identity.sourceCommit}</code>
                  </strong>
                </div>
                <div className="capability-row" data-testid="about.profile">
                  <span>{t('about.profile')}</span>
                  <strong>{identity.buildProfile}</strong>
                </div>
                {identity.buildFeatures.trim() !== '' && (
                  <div className="capability-row" data-testid="about.features">
                    <span>{t('about.features')}</span>
                    <strong>{identity.buildFeatures}</strong>
                  </div>
                )}
              </>
            )}

            <h3>{t('about.links')}</h3>
            <div className="capability-row">
              <span>{t('about.github')}</span>
              <a href={REPO_URL} target="_blank" rel="noreferrer" data-testid="about.link-github">
                GitHub ↗
              </a>
            </div>
            <div className="capability-row">
              <span>{t('about.docs')}</span>
              <a href={DOCS_URL} target="_blank" rel="noreferrer" data-testid="about.link-docs">
                docs ↗
              </a>
            </div>
            <div className="capability-row">
              <span>{t('about.license')}</span>
              <a
                href={LICENSE_URL}
                target="_blank"
                rel="noreferrer"
                data-testid="about.license"
              >
                GPL-3.0 ↗
              </a>
            </div>

            <h3>{t('about.updates')}</h3>
            <div className="capability-row" data-testid="about.updates-status">
              <span>{t('about.updatesStatus')}</span>
              <strong className="text-warning">{t('about.updatesOff')}</strong>
            </div>
            <p className="page-note">{t('about.updatesNote')}</p>
            <div className="capability-row">
              <span>{t('about.releases')}</span>
              <a href={RELEASES_URL} target="_blank" rel="noreferrer" data-testid="about.link-releases">
                Releases ↗
              </a>
            </div>
          </>
        )}
      </section>
    </div>
  )
}
