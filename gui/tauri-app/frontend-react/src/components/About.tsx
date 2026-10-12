// About (Settings zone): the HONEST build identity. Version and source
// commit come from the RUNNING binary via build_identity (build.rs
// RIMLOC_SOURCE_COMMIT etc. — the app reports its own source truth, a dirty
// tree is marked explicitly, never silently claimed clean). The updates
// section states the AUTO_UPDATE_DESIGN fact of THIS build: the Tauri 2
// updater is designed but NOT configured here (tauri.conf plugins: {},
// no tauri-plugin-updater dependency) — so there is no check button and no
// install flow to fake; only the manual releases route.
import { useEffect, useState } from 'react'
import { clientInstance } from '../lib/client/instance'
import type { BuildIdentityDto } from '../lib/client/types'
import { t } from '../lib/i18n'

/** Canonical project home (mkdocs repo_url / README badges). Docs live
 *  in-repo (mkdocs.yml site_url is environment-gated, empty by default). */
export const REPO_URL = 'https://github.com/0-danielviktorovich-0/RimLoc'
const DOCS_URL = `${REPO_URL}/tree/main/docs`
const RELEASES_URL = `${REPO_URL}/releases`

export function About() {
  const [identity, setIdentity] = useState<BuildIdentityDto | null>(null)
  const [failed, setFailed] = useState(false)

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

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('about.eyebrow')}</span>
          <h2>{t('about.title')}</h2>
          <p>{t('about.subtitle')}</p>
        </div>
      </div>

      <section className="settings-section">
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
      </section>

      <section className="settings-section">
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
            href={`${REPO_URL}/blob/main/LICENSE`}
            target="_blank"
            rel="noreferrer"
            data-testid="about.license"
          >
            GPL-3.0 ↗
          </a>
        </div>
      </section>

      <section className="settings-section">
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
      </section>
    </div>
  )
}
