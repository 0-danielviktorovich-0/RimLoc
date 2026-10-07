// Home (R1): New translation · Open/update existing · Recent projects.
// Data lives or states honestly — no fake cards (mandate §21/§42).
import { useEffect } from 'react'
import { FolderOpen, ArrowRight, HardDrive, BookOpen } from 'lucide-react'
import { NewProjectWizard } from './NewProjectWizard'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { registry, SOURCE_LOCALE } from '../lib/languages/registry'
import { t } from '../lib/i18n'

/** Raw managed-project ids are opaque (`proj-<hash>-<n>`) — the card label
 *  is a trimmed human hint, never the raw id (mandate §21). Exported for the
 *  sidebar project switcher, which labels the same cards the same way. */
export function shortId(id: string): string {
  const m = /-(\d+)\.rimloc\.json$/.exec(id) ?? /^proj-([0-9a-f]{6})/.exec(id)
  return m ? `Проект ${m[1]}` : id.slice(0, 12)
}

export function Home({ onOpen }: { onOpen: (projectId: string) => void }) {
  const st = useProjectState()

  useEffect(() => {
    void projectStore.listProjects()
  }, [])

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('home.eyebrow')}</span>
          <h2>{t('home.title')}</h2>
          <p>{t('home.subtitle')}</p>
        </div>
      </div>

      {st.load.kind === 'error' && (
        <div className="inline-warning">
          <span>{st.load.message}</span>
        </div>
      )}

      <div className="project-grid">
        {st.summaries.map((p) => (
          <button
            key={p.project_id}
            className="project-card"
            onClick={() => onOpen(p.project_id)}
            data-testid="home.project-card"
          >
            <div>
              {/* §21: the target comes from the store's active target locale
                  (never-reject registry resolve covers user languages); the
                  summary DTO carries no per-project target. */}
              <span className="project-meta">
                {p.target_version ? `RIMWORLD ${p.target_version}` : 'PROJECT'} · {SOURCE_LOCALE.toUpperCase()} → {registry.resolve(st.targetLocale).nativeName}
              </span>
              <h3>{p.name?.trim() ? p.name : shortId(p.project_id)}</h3>
              <p>
                {t('home.revision')} {p.revision}
              </p>
              <span>
                {t('home.open')} <ArrowRight size={16} />
              </span>
            </div>
          </button>
        ))}
        <NewProjectWizard
          onCreated={() => {
            onOpen('__created__')
          }}
        />
      </div>

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

      {/* §21: the «Help translate RimLoc» card leads to the live selfloc
          journey (the UI catalog opens as an ordinary managed project on
          #/selfloc — the screen owns the real open flow). */}
      <a className="project-storage" href="#/selfloc" data-testid="home.selfloc-card">
        <BookOpen />
        <span>
          {t('home.selflocTitle')} — {t('home.selflocHint')}
        </span>
      </a>
    </div>
  )
}
