// Projects library (nav split, «Проекты» section): managed project cards
// from project_list, client-side search, open, create via the GLOBAL wizard
// (the card only raises onCreate — the wizard instance lives in App), and
// the currently-open project marked honestly on its card. The list IS
// project_list fetched fresh on mount — the same source the sidebar
// switcher labels «Недавние проекты»; no synthetic recency data.
// No delete: the client contract carries no project-delete operation.
// Supersedes the old Home screen as the library surface (#/projects).
import { useEffect, useMemo, useState } from 'react'
import { ArrowRight, BookOpen, FolderOpen, HardDrive, Plus, Search } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { registry, SOURCE_LOCALE } from '../lib/languages/registry'
import { t } from '../lib/i18n'
import { shortId } from './Home'

export function Projects({
  onCreate,
  onOpen,
}: {
  onCreate: () => void
  onOpen: (projectId: string) => void
}) {
  const st = useProjectState()
  const [query, setQuery] = useState('')

  useEffect(() => {
    void projectStore.listProjects()
  }, [])

  // Client-side search: name + raw id, case-insensitive. Pure projection —
  // the underlying list state is untouched.
  const q = query.trim().toLowerCase()
  const visible = useMemo(
    () =>
      st.summaries.filter((p) => !q || `${p.name ?? ''} ${p.project_id}`.toLowerCase().includes(q)),
    [st.summaries, q],
  )
  const currentId = st.snapshot?.project_id ?? null

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('home.eyebrow')}</span>
          <h2>{t('home.title')}</h2>
          <p>{t('home.subtitle')}</p>
        </div>
        <label className="search-field">
          <Search size={16} />
          <input
            aria-label={t('projects.search')}
            placeholder={t('projects.searchPlaceholder')}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            data-testid="projects.search"
          />
        </label>
      </div>

      {st.load.kind === 'error' && (
        <div className="inline-warning">
          <span>{st.load.message}</span>
        </div>
      )}

      {q !== '' && (
        <p className="page-note" data-testid="projects.found">
          {t('projects.found', { count: visible.length })}
        </p>
      )}

      <div className="project-grid">
        {visible.map((p) => {
          const isOpen = p.project_id === currentId
          return (
            <button
              key={p.project_id}
              className="project-card"
              onClick={() => onOpen(p.project_id)}
              data-testid="projects.project-card"
            >
              <div>
                <span className="project-meta">
                  {p.target_version ? `RIMWORLD ${p.target_version}` : 'PROJECT'} ·{' '}
                  {SOURCE_LOCALE.toUpperCase()} → {registry.resolve(st.targetLocale).nativeName}
                </span>
                <h3>
                  {p.name?.trim() ? p.name : shortId(p.project_id)}
                  {isOpen && (
                    <span
                      className="project-meta"
                      style={{ marginLeft: 8, color: 'var(--primary)', fontSize: 9 }}
                      data-testid="projects.open-badge"
                    >
                      {t('projects.openBadge')}
                    </span>
                  )}
                </h3>
                <p>
                  {t('home.revision')} {p.revision}
                </p>
                <span>
                  {t('home.open')} <ArrowRight size={16} />
                </span>
              </div>
            </button>
          )
        })}
        <button className="new-project-card" onClick={onCreate} data-testid="wizard.open">
          <Plus size={28} /> <strong>{t('wizard.open')}</strong> <span>{t('wizard.openHint')}</span>
        </button>
      </div>

      {q !== '' && visible.length === 0 && st.load.kind !== 'loading' && (
        <div className="project-storage" data-testid="projects.no-match">
          <Search />
          <span>{t('projects.noMatch')}</span>
        </div>
      )}

      {st.summaries.length === 0 && st.load.kind !== 'loading' && (
        <div className="project-storage">
          <HardDrive />
          <span>{t('home.emptyNote')}</span>
        </div>
      )}

      <div className="project-storage">
        <FolderOpen />
        <span>{t('home.openExistingNote')}</span>
      </div>

      {/* The «Help translate RimLoc» card leads to the live selfloc journey
          (the UI catalog opens as an ordinary managed project on #/selfloc). */}
      <a className="project-storage" href="#/selfloc" data-testid="projects.selfloc-card">
        <BookOpen />
        <span>
          {t('home.selflocTitle')} — {t('home.selflocHint')}
        </span>
      </a>
    </div>
  )
}
