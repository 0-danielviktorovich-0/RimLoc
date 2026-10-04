// RimLoc React lane — application root (UI R1).
// Hash router + the R1 visual shell (mandate §9/§47: Lovable R1 is the
// visual contract; the shell composition is ADOPT, data is LIVE-only).
import { useEffect, useState } from 'react'
import { Languages, Settings2, FolderOpen, ShieldCheck, GitCompareArrows, Package, Wrench, Sun, Moon, Plus, ChevronDown, ChevronRight, X, Check, PanelLeftOpen, PanelLeftClose, ArrowUpRight } from 'lucide-react'
import { clientInstance } from './lib/client/instance'
import { projectStore } from './lib/state/project'
import { BUILTIN_LANGUAGES as LANGUAGES } from './lib/languages/registry'
import { useProjectState } from './lib/state/useProjectState'
import { Home } from './components/Home'
import { Workspace } from './components/Workspace'
import { Checks } from './components/Checks'
import { Glossary } from './components/Glossary'
import { BuildExport } from './components/BuildExport'
import { Existing } from './components/Existing'
import { Selfloc } from './components/Selfloc'
import { Diagnostics } from './components/Diagnostics'
import { t } from './lib/i18n'

type Route =
  | 'home'
  | 'projects'
  | 'checks'
  | 'compare'
  | 'glossary'
  | 'export'
  | 'tools'
  | 'settings'
  | 'workspace'
  | 'existing'
  | 'selfloc'
  | 'diagnostics'

const NAV: { to: Route; label: string; icon: typeof FolderOpen }[] = [
  { to: 'projects', label: t('nav.projects'), icon: FolderOpen },
  { to: 'home', label: t('nav.entries'), icon: Languages },
  { to: 'checks', label: t('nav.checks'), icon: ShieldCheck },
  { to: 'existing', label: t('nav.existing'), icon: GitCompareArrows },
  { to: 'compare', label: t('nav.compare'), icon: GitCompareArrows },
  { to: 'glossary', label: t('nav.glossary'), icon: Package },
  { to: 'export', label: t('nav.export'), icon: Wrench },
]

function currentRoute(): Route {
  const h = window.location.hash.replace(/^#\/?/, '')
  const known: Route[] = ['home', 'projects', 'checks', 'compare', 'glossary', 'export', 'tools', 'settings', 'workspace', 'existing', 'selfloc', 'diagnostics']
  return (known.find((r) => r === h) ?? 'home') as Route
}

export function App() {
  const st = useProjectState()
  const [route, setRoute] = useState<Route>(currentRoute)
  const [dark, setDark] = useState(false)
  const [navOpen, setNavOpen] = useState(false)
  const [clientError, setClientError] = useState<string | null>(null)

  /** Human project label: display name → trimmed id hint. Raw managed ids
   *  never render as user-facing labels (visual critique round 1/2). */
  const progressTranslated = (): number =>
    st.entries.filter((e) => e.target.trim() !== '' && e.completeness !== 'todo').length
  const progressTotal = (): number => st.entries.filter((e) => e.lifecycle !== 'orphan').length
  const progressPercent = (): number => {
    const total = progressTotal()
    return total === 0 ? 0 : Math.round((progressTranslated() / total) * 100)
  }
  const projectLabel = (): string => {
    if (!st.snapshot) return t('shell.noProject')
    const named = st.summaries.find((x) => x.project_id === st.snapshot!.project_id)?.name?.trim()
    if (named) return named
    const m = /(?:^|-)(\d{4,})$/.exec(st.snapshot.project_id)
    return m ? `${t('shell.project')} #${m[1]}` : st.snapshot.project_id.slice(0, 14)
  }
  useEffect(() => {
    document.documentElement.classList.toggle('dark', dark)
  }, [dark])

  useEffect(() => {
    const onHash = () => setRoute(currentRoute())
    window.addEventListener('hashchange', onHash)
    // Background WebKit windows may DEFER hashchange dispatch (same macOS 27
    // event-deferral class as browser.tauri.execute) — a light poll keeps
    // route state deterministic without focus (live lesson of the smoke).
    const poll = window.setInterval(() => setRoute(currentRoute()), 400)
    return () => {
      window.removeEventListener('hashchange', onHash)
      window.clearInterval(poll)
    }
  }, [])

  // Honest client resolution (mirrors the frozen Svelte instance store):
  // tauri bridge → live; otherwise a configuration error, never a mock.
  let client: ReturnType<typeof clientInstance.getClient> | null = null
  try {
    client = clientInstance.getClient()
  } catch (e) {
    if (!clientError) setClientError(e instanceof Error ? e.message : String(e))
  }

  return (
    <div className={`rim-app ${navOpen ? 'mobile-nav-open' : ''}`}>
      <aside className="app-sidebar">
        <a href="#/home" className="brand">
          <div className="brand-mark">
            <Languages size={21} />
          </div>
          <span>
            RimLoc<span className="brand-period">.</span>
          </span>
          <small>R1</small>
        </a>
        <div className="sidebar-project">
          <div>
            <strong>{projectLabel()}</strong>
            <span>
              {st.snapshot ? t('shell.projectOpen') : t('shell.localProject')} <span className="dot success" />
            </span>
          </div>
          <ChevronDown size={14} />
        </div>
        <nav className="primary-nav">
          {NAV.map(({ to, label, icon: Icon }) => (
            <a key={to} href={`#/${to}`} className={(route === to || (to === 'home' && route === 'workspace')) ? 'active' : ''} onClick={() => setNavOpen(false)}>
              <Icon />
              <span>{label}</span>
            </a>
          ))}
        </nav>
        {st.snapshot && (
          <div className="sidebar-progress">
            <div>
              <span>{t('shell.yourTranslation')}</span>
              <strong>{progressPercent()}%</strong>
            </div>
            <progress value={progressTranslated()} max={progressTotal() || 1} />
            <p>
              {progressTranslated()} {t('ws.of')} {progressTotal() || 0} {t('ws.rows')}
            </p>
          </div>
        )}
        <div className="sidebar-bottom">
          <a href="#/settings" className={route === 'settings' ? 'active' : ''}>
            <Settings2 /> {t('nav.settings')}
          </a>
          <div className="sidebar-foot">
            <span>RimLoc · React R1</span>
            <span className="dot success" />
          </div>
        </div>
      </aside>

      <div className="app-main">
        <header className="app-topbar">
          <button
            className="icon-btn mobile-menu"
            aria-label={navOpen ? t('a11y.closeNav') : t('a11y.openNav')}
            onClick={() => setNavOpen(!navOpen)}
          >
            {navOpen ? <PanelLeftClose /> : <PanelLeftOpen />}
          </button>
          <div className="breadcrumb">
            <FolderOpen size={15} />
            <span>{projectLabel()}</span>
            <ChevronRight size={13} />
            <strong>{route === 'workspace' ? t('nav.entries') : NAV.find((n) => n.to === route)?.label ?? t('nav.settings')}</strong>
          </div>
          <div className="topbar-actions">
            <button className="icon-btn" data-testid="theme-toggle" aria-label={dark ? t('a11y.lightTheme') : t('a11y.darkTheme')} onClick={() => setDark(!dark)}>
              {dark ? <Sun /> : <Moon />}
            </button>
            <div className="topbar-divider" />
            <a className="btn-primary" href="#/projects">
              <Plus /> {t('shell.newProject')}
            </a>
          </div>
        </header>

        <main className="route-content">
          {clientError ? (
            <div className="page-content narrow-page">
              <div className="inline-warning">
                <X size={15} />
                <span>{clientError}</span>
              </div>
            </div>
          ) : route === 'workspace' ? (
            <div className="ws-page">
              <div className="workspace-heading">
                <div>
                  <div className="heading-eyebrow">
                    <span className="dot primary" /> ENGLISH <span>→</span> РУССКИЙ
                  </div>
                  <h1>{t('ws.headingTitle')}</h1>
                  <p>{t('ws.headingSubtitle')}</p>
                </div>
                <div className="heading-controls">
                  <label className="version-select">
                    <span>RimWorld</span>
                    <select aria-label={t('ws.gameVersion')} defaultValue="1.6">
                      <option>1.6</option>
                      <option>1.5</option>
                      <option>1.4</option>
                    </select>
                  </label>
                  <label className="version-select">
                    <span>→</span>
                    <select
                      aria-label={t('ws.targetLang')}
                      data-testid="ws.target-locale"
                      value={st.targetLocale}
                      onChange={(e) => projectStore.setTargetLocale(e.target.value)}
                    >
                      {LANGUAGES.map((l) => (
                        <option key={l.localeId} value={l.localeId}>
                          {l.nativeName}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
              </div>
              <Workspace onBack={() => { window.location.hash = '#/home' }} />
            </div>
          ) : route === 'existing' ? (
            <Existing />
          ) : route === 'export' ? (
            <BuildExport />
          ) : route === 'selfloc' ? (
            <Selfloc onOpen={() => { window.location.hash = '#/workspace' }} />
          ) : route === 'diagnostics' ? (
            <Diagnostics />
          ) : route === 'checks' ? (
            <Checks />
          ) : route === 'glossary' ? (
            <Glossary />
          ) : route === 'home' || route === 'projects' ? (
          // Home falls through to the shared layout below
            <Home
              onOpen={(projectId) => {
                const after =
                  projectId === '__created__'
                    ? Promise.resolve(true)
                    : projectStore.open(projectId)
                void after.then((ok) => {
                  if (ok) window.location.hash = '#/workspace'
                })
              }}
            />
          ) : (
            <div className="page-content narrow-page">
              <div className="section-heading">
                <div>
                  <span className="eyebrow">
                    <span className="dot primary" /> REACT R1 · REPRESENTATIVE LANE
                  </span>
                  <h2>{t('shell.placeholderTitle')}</h2>
                  <p>{t('shell.placeholderBody')}</p>
                </div>
              </div>
              <p className="page-note">
                <Check size={15} /> client: {client ? 'resolved' : 'pending'} · contract surface ready
                <ArrowUpRight size={12} />
              </p>
            </div>
          )}
        </main>

        <footer className="app-footer">
          <span>
            <span className="dot success" /> {t('shell.safetyNote')}
          </span>
        </footer>
      </div>
    </div>
  )
}
