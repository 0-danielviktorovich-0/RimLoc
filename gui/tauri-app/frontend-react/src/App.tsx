// RimLoc React lane — application root (UI R1).
// Hash router + the R1 visual shell (mandate §9/§47: Lovable R1 is the
// visual contract; the shell composition is ADOPT, data is LIVE-only).
// Route metadata (sidebar, palette, breadcrumb, currentRoute whitelist)
// lives in lib/routes.ts (§18) — this file owns no route literals.
import { useEffect, useMemo, useState } from 'react'
import { Languages, FolderOpen, Sun, Moon, Plus, ChevronRight, X, PanelLeftOpen, PanelLeftClose } from 'lucide-react'
import { clientInstance } from './lib/client/instance'
import { projectStore } from './lib/state/project'
import { BUILTIN_LANGUAGES as LANGUAGES, SOURCE_LOCALE } from './lib/languages/registry'
import { folderForm } from './lib/languages/folderForm'
import { loadUserLanguages, type UserLanguage } from './lib/languages/manager'
import { useProjectState } from './lib/state/useProjectState'
import { ROUTES, ROUTE_IDS, routeMeta, type Route } from './lib/routes'
import { Home } from './components/Home'
import { Workspace } from './components/Workspace'
import { Checks } from './components/Checks'
import { Glossary } from './components/Glossary'
import { Tm } from './components/Tm'
import { ChatBatch } from './components/ChatBatch'
import { BuildExport } from './components/BuildExport'
import { Existing } from './components/Existing'
import { Compare } from './components/Compare'
import { Selfloc } from './components/Selfloc'
import { Diagnostics } from './components/Diagnostics'
import { Settings } from './components/Settings'
import { ProvidersScreen } from './components/ProvidersScreen'
import { LanguageManager } from './components/LanguageManager'
import { ProjectSwitcher } from './components/ProjectSwitcher'
import { t, getLocale, setLocale, type Locale } from './lib/i18n'
import { useCommandPalette, type PaletteCommand } from './lib/palette'

// §18 projections from ROUTES (order preserved):
//  - primary-nav: every sidebar route except settings (settings renders in
//    the sidebar-bottom slot, as in the Lovable canon), sorted by the
//    historical nav rank (sidebarOrder) — «Проекты» first, «Строки
//    перевода» second;
//  - palette: the 14 nav commands in ROUTES array order — composition,
//    order and labels are pinned by palette-acceptance (WDIO); labels
//    resolve through t() per render so the UI-language switch reaches the
//    shell too (audit v2 #4/#5).
const SIDEBAR_NAV = ROUTES.filter((m) => m.visibleInSidebar && m.route !== 'settings' && m.sidebarOrder !== undefined).sort((a, b) => a.sidebarOrder! - b.sidebarOrder!)
const SETTINGS_META = routeMeta('settings')!

// Palette commands (§18) plus ACTION commands that appear ONLY with an open
// project (audit v2 #7, mandate §9): one target-switch command per language
// from targetLangs and «Open project» (jumps to the workspace of the open
// project). Labels resolve through t(); массив строится фабрикой и держит
// СТАБИЛЬНУЮ идентичность между рендерами при тех же зависимостях —
// фильтр-мемо хука и эффект курсора зависят от идентичности списка,
// пересоздание на каждый рендер сбрасывало бы activeIndex стрелками.
function buildPaletteCommands(extra: {
  projectOpen: boolean
  targetLangs: { localeId: string; nativeName: string }[]
  targetLocale: string
}): PaletteCommand[] {
  const nav: PaletteCommand[] = ROUTES.filter((m) => m.visibleInPalette).map((m) => ({
    id: m.route,
    label: t(m.paletteKey ?? m.labelKey),
    action:
      m.route === 'home'
        ? // §6: «Строки перевода» ведёт в открытый workspace (лейбл
          // зафиксирован palette-acceptance — меняется только цель).
          () => {
            window.location.hash = extra.projectOpen ? '#/workspace' : '#/home'
          }
        : () => {
            window.location.hash = `#/${m.route}`
          },
  }))
  if (!extra.projectOpen) return nav
  return [
    ...nav,
    { id: 'open-project', label: t('palette.cmd.openProject'), action: () => { window.location.hash = '#/workspace' } },
    ...extra.targetLangs.map((l) => ({
      id: `target-${l.localeId}`,
      label: `${t('palette.cmd.target')} ${l.nativeName}`,
      action: () => projectStore.setTargetLocale(l.localeId),
    })),
  ]
}

function currentRoute(): Route {
  const h = window.location.hash.replace(/^#\/?/, '')
  return ROUTE_IDS.find((r) => r === h) ?? 'home'
}

// §43: onboarding-strip живёт до первого явного закрытия (localStorage-флаг).
// Wizard-тур открывает настоящий клик по карточке wizard.open: состояние
// мастера принадлежит Home, а лейн-владение запрещает править Home/Wizard.
const ONBOARDING_KEY = 'rimloc.onboarding.dismissed'

export function App() {
  const st = useProjectState()
  const [route, setRoute] = useState<Route>(currentRoute)
  // §6/§17 Theme persistence: 'rimloc.theme' ('dark'|'light') read once at
  // first mount; every write goes through changeDark so the topbar toggle and
  // the Settings select share ONE storage key.
  const [dark, setDark] = useState<boolean>(() => {
    try {
      return localStorage.getItem('rimloc.theme') === 'dark'
    } catch {
      return false
    }
  })
  const changeDark = (v: boolean): void => {
    setDark(v)
    try {
      localStorage.setItem('rimloc.theme', v ? 'dark' : 'light')
    } catch {
      /* storage unavailable — session-only theme */
    }
  }
  const [navOpen, setNavOpen] = useState(false)
  // UI-локаль — ЕДИНЫЙ источник в состоянии App (audit v2 #4/#5): смена
  // языка в Settings вызывает setLocale + сеттинг стейта → немедленный
  // ререндер всего дерева (t() читает модульную локаль на рендере), а не
  // «на следующий роут». Мемо-ключ пересобирает команды палитры с метками
  // активного языка, идентичность между рендерами сохраняется.
  const [uiLocale, setUiLocaleState] = useState<Locale>(getLocale())
  // Acceptance MUST-FIX #2 (LM↔target): пользовательские языки жили только в
  // localStorage LM-экрана и не попадали в переключатель цели воркспейса.
  // Перечитываем при входе на маршрут — LM на своём маршруте мог их изменить.
  const [userLangs, setUserLangs] = useState<UserLanguage[]>(() => loadUserLanguages())
  useEffect(() => {
    setUserLangs(loadUserLanguages())
  }, [route])
  // §43: полоса показывается на Home, пока пользователь не закрыл её (флаг
  // в localStorage) и у него нет открытого проекта; перечитываем флаг при
  // входе на Home — другой фон окна мог его выставить.
  const [onboardingDismissed, setOnboardingDismissed] = useState(
    () => localStorage.getItem(ONBOARDING_KEY) !== null,
  )
  useEffect(() => {
    if (route === 'home') setOnboardingDismissed(localStorage.getItem(ONBOARDING_KEY) !== null)
  }, [route])
  const dismissOnboarding = (): void => {
    localStorage.setItem(ONBOARDING_KEY, '1')
    setOnboardingDismissed(true)
  }
  const startWizardTour = (): void => {
    document.querySelector<HTMLButtonElement>('[data-testid="wizard.open"]')?.click()
  }
  // §11: dirty-индикатор — производная от store (drafts + snapshot.dirty +
  // busy), без таймеров. Приоритет: активная запись → ошибка → грязно → чисто.
  const saveState = !st.snapshot
    ? null
    : st.busy
      ? { label: t('shell.saveState.saving'), dot: 'dot primary' }
      : st.lastError !== null
        ? { label: t('shell.saveState.error'), dot: 'dot destructive' }
        // Пилюля считает только НЕПУСТЫЕ черновики: чистый черновик —
        // отсутствие записи (revert удаляет ключ, commit тоже).
        : Object.values(st.drafts).some((v) => v !== '') || st.snapshot.dirty === true
          ? { label: t('shell.saveState.unsaved'), dot: 'dot warning' }
          : { label: t('shell.saveState.saved'), dot: 'dot success' }
  // §10: красная пилюля на «Проверки» — только РЕАЛЬНЫЙ error_count из
  // живой валидации; null-отчёт (stale/unknown) честно скрывает бейдж.
  const checksErrorCount =
    st.validation.report && st.validation.report.error_count > 0 ? st.validation.report.error_count : 0
  const SettingsIcon = SETTINGS_META.icon
  const targetLangs = [
    ...LANGUAGES.map((l) => ({ localeId: l.localeId, nativeName: l.nativeName })),
    ...userLangs.map((l) => ({ localeId: l.localeId, nativeName: l.nativeName })),
  ]
  // Зависимости мемо команд палитры — стабильные между рендерами величины:
  // локаль UI, факт открытого проекта, активная цель и реестр
  // пользовательских языков (targetLangs выводится из LANGUAGES + userLangs,
  // сам массив каждый рендер новый — в депсы не попадает).
  const paletteCommands = useMemo(
    () => buildPaletteCommands({ projectOpen: st.snapshot !== null, targetLangs, targetLocale: st.targetLocale }),
    [uiLocale, st.snapshot !== null, st.targetLocale, userLangs],
  )
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
  // (The resolved client is consumed by screens/store — the shell only
  // surfaces the resolution failure.)
  try {
    clientInstance.getClient()
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
        {/* §6/§19: the project card is a real switcher now (recent projects →
            projectStore.open); the dead ChevronDown affordance is gone. */}
        <ProjectSwitcher />
        <nav className="primary-nav">
          {SIDEBAR_NAV.map(({ route: to, labelKey, icon: Icon }) => (
            <a key={to} href={`#/${to}`} className={(route === to || (to === 'home' && route === 'workspace')) ? 'active' : ''} onClick={() => setNavOpen(false)}>
              <Icon />
              <span>{t(labelKey)}</span>
              {to === 'checks' && checksErrorCount > 0 && (
                <span className="nav-count" data-testid="nav.checks-count">
                  {checksErrorCount}
                </span>
              )}
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
          <a href={`#/${SETTINGS_META.route}`} className={route === SETTINGS_META.route ? 'active' : ''}>
            <SettingsIcon /> {t(SETTINGS_META.labelKey)}
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
            {/* §18: честный лейбл маршрута из routes.ts — providers/selfloc/
              diagnostics несут свои имена, никакого fallback на «Настройки».
              workspace несёт nav.entries (мета-лейбл маршрута). */}
            <strong>{t(routeMeta(route)?.labelKey ?? 'nav.entries')}</strong>
          </div>
          <div className="topbar-actions">
            {/* §11: Saved / Unsaved / Saving… / Error — только с открытым
              проектом: без него сохранять нечего, честно скрыто. */}
            {saveState && (
              <span className="status-label" data-testid="shell.save-state">
                <span className={saveState.dot} />
                {saveState.label}
              </span>
            )}
            <button className="icon-btn" data-testid="theme-toggle" aria-label={dark ? t('a11y.lightTheme') : t('a11y.darkTheme')} onClick={() => changeDark(!dark)}>
              {dark ? <Sun /> : <Moon />}
            </button>
            <div className="topbar-divider" />
            <a className="btn-primary" href="#/projects">
              <Plus /> {t('shell.newProject')}
            </a>
          </div>
        </header>

        <main className="route-content">
          {/* §43: тонкая полоса первого запуска — только на Home, до закрытия
            (localStorage-флаг) и до открытого проекта. */}
          {route === 'home' && !clientError && !onboardingDismissed && !st.snapshot && (
            <div className="onboarding-strip" data-testid="onboarding.strip">
              <span className="onboarding-label">{t('onboarding.label')}</span>
              <div className="onboarding-steps">
                <button onClick={startWizardTour} data-testid="onboarding.start">
                  <span className="step-number current">1</span>
                  {t('onboarding.step1')}
                </button>
              </div>
              <button className="icon-btn" aria-label={t('onboarding.dismiss')} data-testid="onboarding.dismiss" onClick={dismissOnboarding}>
                <X size={14} />
              </button>
            </div>
          )}
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
              onDarkChange={changeDark}
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
          ) : route === 'chatbatch' ? (
            <ChatBatch />
          ) : (
            // §16: placeholder-маршрут удалён — Route больше не содержит
            // 'tools' (INTENTIONALLY_REJECTED: продуктового определения не
            // было), финальная ветка честно покрывает home/projects.
            // Home falls through to the shared layout below.
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
