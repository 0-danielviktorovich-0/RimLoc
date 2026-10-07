// RimLoc React lane — application root (UI R1).
// Hash router + the R1 visual shell (mandate §9/§47: Lovable R1 is the
// visual contract; the shell composition is ADOPT, data is LIVE-only).
import { useEffect, useMemo, useState } from 'react'
import { Languages, Settings2, FolderOpen, ShieldCheck, GitCompareArrows, Package, Wrench, Sun, Moon, Plus, ChevronDown, ChevronRight, X, Check, PanelLeftOpen, PanelLeftClose, ArrowUpRight, Globe } from 'lucide-react'
import { clientInstance } from './lib/client/instance'
import { projectStore } from './lib/state/project'
import { BUILTIN_LANGUAGES as LANGUAGES, SOURCE_LOCALE } from './lib/languages/registry'
import { folderForm } from './lib/languages/folderForm'
import { loadUserLanguages, type UserLanguage } from './lib/languages/manager'
import { useProjectState } from './lib/state/useProjectState'
import { Home } from './components/Home'
import { Workspace } from './components/Workspace'
import { Checks } from './components/Checks'
import { Glossary } from './components/Glossary'
import { Tm } from './components/Tm'
import { BuildExport } from './components/BuildExport'
import { Existing } from './components/Existing'
import { Compare } from './components/Compare'
import { Selfloc } from './components/Selfloc'
import { Diagnostics } from './components/Diagnostics'
import { Settings } from './components/Settings'
import { ProvidersScreen } from './components/ProvidersScreen'
import { LanguageManager } from './components/LanguageManager'
import { t, getLocale, setLocale, type Locale } from './lib/i18n'
import { useCommandPalette, type PaletteCommand } from './lib/palette'

// Palette commands are pure hash navigations. Labels resolve through t()
// (audit: RU-литералы не менялись с UI-языком); массив строится фабрикой и
// держит СТАБИЛЬНУЮ идентичность на смену UI-локали — фильтр-мемо хука и
// эффект курсора зависят от идентичности списка, пересоздание на каждый
// рендер сбрасывало бы activeIndex стрелками.
// Acceptance §3 MUST-FIX #5: маршруты existing/compare/selfloc/diagnostics/
// providers/lm не были покрыты командами — теперь полный набор.
// tools УДАЛЁН (W0-решение: нет продуктового определения — LIVE или удалён
// из навигации; маршрут в Route type остаётся, hash #/tools рендерит
// fallback как раньше).
function buildPaletteCommands(): PaletteCommand[] {
  return [
    { id: 'entries', label: t('palette.cmd.entries'), action: () => { window.location.hash = '#/home' } },
    { id: 'projects', label: t('palette.cmd.projects'), action: () => { window.location.hash = '#/projects' } },
    { id: 'checks', label: t('palette.cmd.checks'), action: () => { window.location.hash = '#/checks' } },
    { id: 'existing', label: t('palette.cmd.existing'), action: () => { window.location.hash = '#/existing' } },
    { id: 'compare', label: t('palette.cmd.compare'), action: () => { window.location.hash = '#/compare' } },
    { id: 'glossary', label: t('palette.cmd.glossary'), action: () => { window.location.hash = '#/glossary' } },
    { id: 'tm', label: t('palette.cmd.tm'), action: () => { window.location.hash = '#/tm' } },
    { id: 'export', label: t('palette.cmd.export'), action: () => { window.location.hash = '#/export' } },
    { id: 'selfloc', label: t('palette.cmd.selfloc'), action: () => { window.location.hash = '#/selfloc' } },
    { id: 'diagnostics', label: t('palette.cmd.diagnostics'), action: () => { window.location.hash = '#/diagnostics' } },
    { id: 'providers', label: t('palette.cmd.providers'), action: () => { window.location.hash = '#/providers' } },
    { id: 'lm', label: t('palette.cmd.lm'), action: () => { window.location.hash = '#/lm' } },
    { id: 'settings', label: t('palette.cmd.settings'), action: () => { window.location.hash = '#/settings' } },
  ]
}

type Route =
  | 'home'
  | 'projects'
  | 'checks'
  | 'compare'
  | 'glossary'
  | 'tm'
  | 'export'
  | 'tools'
  | 'settings'
  | 'workspace'
  | 'existing'
  | 'selfloc'
  | 'diagnostics'
  | 'providers'
  | 'lm'

const NAV: { to: Route; label: string; icon: typeof FolderOpen }[] = [
  { to: 'projects', label: t('nav.projects'), icon: FolderOpen },
  { to: 'home', label: t('nav.entries'), icon: Languages },
  { to: 'checks', label: t('nav.checks'), icon: ShieldCheck },
  { to: 'existing', label: t('nav.existing'), icon: GitCompareArrows },
  { to: 'compare', label: t('nav.compare'), icon: GitCompareArrows },
  { to: 'glossary', label: t('nav.glossary'), icon: Package },
  { to: 'tm', label: t('nav.tm'), icon: Package },
  { to: 'export', label: t('nav.export'), icon: Wrench },
  // Acceptance MUST-FIX #1: LM был недостижим из UI (только ручной #/lm).
  { to: 'lm', label: t('nav.lm'), icon: Globe },
  // tools удалён из навигации (W0): не было продуктового определения.
  // Route 'tools' жив — внешний hash #/tools честно падает в fallback.
]

function currentRoute(): Route {
  const h = window.location.hash.replace(/^#\/?/, '')
  const known: Route[] = ['home', 'projects', 'checks', 'compare', 'glossary', 'tm', 'export', 'tools', 'settings', 'workspace', 'existing', 'selfloc', 'diagnostics', 'providers', 'lm']
  return (known.find((r) => r === h) ?? 'home') as Route
}

export function App() {
  const st = useProjectState()
  const [route, setRoute] = useState<Route>(currentRoute)
  const [dark, setDark] = useState(false)
  const [navOpen, setNavOpen] = useState(false)
  // UI-локаль — ЕДИНЫЙ источник в состоянии App (audit v2 #4/#5): смена
  // языка в Settings вызывает setLocale + сеттинг стейта → немедленный
  // ререндер всего дерева (t() читает модульную локаль на рендере), а не
  // «на следующий роут». Мемо-ключ пересобирает команды палитры с метками
  // активного языка, идентичность между рендерами сохраняется.
  const [uiLocale, setUiLocaleState] = useState<Locale>(getLocale())
  const paletteCommands = useMemo(buildPaletteCommands, [uiLocale])
  // Palette state (open/query/activeIndex) lives in the hook — one source of
  // truth for the window keydown contract (Cmd+K, Escape, arrows, Enter).
  const {
    open: paletteOpen,
    setOpen: setPaletteOpen,
    query: paletteQuery,
    setQuery: setPaletteQuery,
    filtered: paletteFiltered,
    activeIndex,
    setActiveIndex,
    runCommand,
  } = useCommandPalette(paletteCommands)
  const [clientError, setClientError] = useState<string | null>(null)

  // Keep the active option visible while navigating with arrows/End/Home
  // (Svelte canon: scrollIntoView block:'nearest' on the active option id).
  useEffect(() => {
    if (!paletteOpen) return
    document.getElementById(`palette-opt-${activeIndex}`)?.scrollIntoView({ block: 'nearest' })
  }, [paletteOpen, activeIndex])

  // Acceptance MUST-FIX #2 (LM↔target): пользовательские языки жили только в
  // localStorage LM-экрана и не попадали в переключатель цели воркспейса.
  // Перечитываем при входе на маршрут — LM на своём маршруте мог их изменить.
  const [userLangs, setUserLangs] = useState<UserLanguage[]>(() => loadUserLanguages())
  useEffect(() => {
    setUserLangs(loadUserLanguages())
  }, [route])
  const targetLangs = [
    ...LANGUAGES.map((l) => ({ localeId: l.localeId, nativeName: l.nativeName })),
    ...userLangs.map((l) => ({ localeId: l.localeId, nativeName: l.nativeName })),
  ]

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

  // Cmd+K / Escape / arrows / Enter are handled by useCommandPalette (window
  // keydown) — App no longer adds its own listener (a second one would
  // double-toggle Cmd+K).
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
                    {/* Источник/цель из состояния (audit v2 #3): folderForm
                      даёт строгую папочную форму реестра; en — канонический
                      источник (SOURCE_LOCALE). */}
                    <span className="dot primary" /> {folderForm(SOURCE_LOCALE)} <span>→</span> {folderForm(st.targetLocale)}
                  </div>
                  <h1>{t('ws.headingTitle')}</h1>
                  <p>{t('ws.headingSubtitle')}</p>
                </div>
                <div className="heading-controls">
                  {/* Мёртвый version-select УБРАН (аудит): селект с
                    defaultValue без onChange ничего не управлял, а
                    projectStore.gameVersion не существует — возвращаем,
                    когда версия появится в состоянии проекта. */}
                  <label className="version-select">
                    <span>→</span>
                    <select
                      aria-label={t('ws.targetLang')}
                      data-testid="ws.target-locale"
                      value={st.targetLocale}
                      onChange={(e) => projectStore.setTargetLocale(e.target.value)}
                    >
                      {targetLangs.map((l) => (
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
          ) : route === 'compare' ? (
            <Compare />
          ) : route === 'export' ? (
            <BuildExport />
          ) : route === 'selfloc' ? (
            <Selfloc onOpen={() => { window.location.hash = '#/workspace' }} />
          ) : route === 'diagnostics' ? (
            <Diagnostics />
          ) : route === 'settings' ? (
            <Settings
              dark={dark}
              onDarkChange={setDark}
              locale={uiLocale}
              onLocaleChange={(v) => {
                setLocale(v)
                setUiLocaleState(v)
              }}
            />
          ) : route === 'providers' ? (
            <ProvidersScreen />
          ) : route === 'lm' ? (
            <LanguageManager />
          ) : route === 'checks' ? (
            <Checks />
          ) : route === 'glossary' ? (
            <Glossary />
          ) : route === 'tm' ? (
            <Tm />
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

      {paletteOpen && (
        <div className="palette-overlay" onClick={() => setPaletteOpen(false)}>
          <div className="palette-box" onClick={(e) => e.stopPropagation()}>
            <input
              className="palette-input"
              role="combobox"
              aria-expanded={paletteFiltered.length > 0}
              aria-controls="palette-listbox"
              aria-autocomplete="list"
              aria-activedescendant={paletteFiltered[activeIndex] ? `palette-opt-${activeIndex}` : undefined}
              placeholder={t('palette.search')}
              value={paletteQuery}
              onChange={(e) => setPaletteQuery(e.target.value)}
              autoFocus
              data-testid="palette.input"
            />
            <div className="palette-list" id="palette-listbox" role="listbox" aria-label={t('palette.title')}>
              {paletteFiltered.map((c, i) => (
                <button
                  key={c.id}
                  type="button"
                  id={`palette-opt-${i}`}
                  role="option"
                  aria-selected={i === activeIndex}
                  className={i === activeIndex ? 'palette-item palette-item-active' : 'palette-item'}
                  tabIndex={-1}
                  onMouseEnter={() => setActiveIndex(i)}
                  onClick={() => runCommand(c)}
                >
                  {c.label}
                </button>
              ))}
              {paletteQuery.trim() !== '' && paletteFiltered.length === 0 && (
                <p className="palette-empty">{t('palette.empty')}</p>
              )}
            </div>
          </div>
        </div>
      )}
      </div>
    </div>
  )
}
