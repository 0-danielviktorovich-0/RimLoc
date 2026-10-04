// Home (R1): New translation · Open/update existing · Recent projects.
// Data lives or states honestly — no fake cards (mandate §21/§42).
import { useEffect } from 'react'
import { Plus, FolderOpen, ArrowRight, HardDrive } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { t } from '../lib/i18n'

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
              <span className="project-meta">{p.target_version ? `RIMWORLD ${p.target_version}` : 'PROJECT'} · EN → RU</span>
              <h3>{p.name || p.project_id}</h3>
              <p>
                {t('home.revision')} {p.revision}
              </p>
              <span>
                {t('home.open')} <ArrowRight size={16} />
              </span>
            </div>
          </button>
        ))}
        <button className="new-project-card" data-testid="home.new-project">
          <Plus size={28} />
          <strong>{t('home.newTranslation')}</strong>
          <span>{t('home.newTranslationHint')}</span>
        </button>
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
    </div>
  )
}
