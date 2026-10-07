// Sidebar project switcher (§6/§19): the ChevronDown on the project card was
// a dead affordance — the card is now a real toggle revealing the recent
// managed projects (project_list) and opening the chosen one
// (projectStore.open → workspace). No synthetic recency data: the list IS
// project_list, fetched fresh on every expand.
import { useEffect, useState } from 'react'
import { ChevronDown } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { projectStore } from '../lib/state/project'
import { t } from '../lib/i18n'
import { shortId } from './Home'

const listStyle: React.CSSProperties = {
  position: 'absolute',
  top: 'calc(100% + 6px)',
  left: 0,
  right: 0,
  zIndex: 40,
  background: 'var(--popover, var(--card))',
  border: '1px solid var(--border)',
  borderRadius: 6,
  boxShadow: '0 12px 32px rgba(0,0,0,.28)',
  padding: 6,
  maxHeight: 260,
  overflow: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: 2,
}

const optionStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'flex-start',
  gap: 2,
  width: '100%',
  textAlign: 'left',
  padding: '8px 9px',
  borderRadius: 5,
  font: 'inherit',
  color: 'inherit',
  cursor: 'pointer',
  background: 'transparent',
  border: 0,
}

export function ProjectSwitcher() {
  const st = useProjectState()
  const [open, setOpen] = useState(false)

  // Fresh list on every expand — project_list is cheap and the honest source
  // (no cached "recent" copy that could drift from the library).
  useEffect(() => {
    if (open) void projectStore.listProjects()
  }, [open])

  const currentId = st.snapshot?.project_id ?? null
  const current = currentId ? st.summaries.find((p) => p.project_id === currentId) : undefined
  const others = st.summaries.filter((p) => p.project_id !== currentId)
  const cardLabel = current
    ? current.name?.trim() || shortId(current.project_id)
    : currentId
      ? shortId(currentId)
      : t('shell.noProject')

  const openProject = (projectId: string): void => {
    setOpen(false)
    void projectStore.open(projectId).then((ok) => {
      if (ok) window.location.hash = '#/workspace'
    })
  }

  return (
    <div style={{ position: 'relative' }}>
      <button
        type="button"
        className="sidebar-project"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={t('shell.switchProject')}
        data-testid="sidebar.project-switcher"
        onClick={() => setOpen(!open)}
        style={{
          width: 'calc(100% - 28px)',
          textAlign: 'left',
          cursor: 'pointer',
          font: 'inherit',
          color: 'inherit',
          background: 'transparent',
        }}
      >
        <div>
          <strong>{cardLabel}</strong>
          <span>
            {st.snapshot ? t('shell.projectOpen') : t('shell.localProject')} <span className="dot success" />
          </span>
        </div>
        <ChevronDown
          size={14}
          style={{ transform: open ? 'rotate(180deg)' : 'none', transition: 'transform .15s' }}
        />
      </button>

      {open && (
        <>
          {/* Click-away backdrop: closes the panel without juggling document
              listeners — same pattern as the command palette overlay. */}
          <div
            style={{ position: 'fixed', inset: 0, zIndex: 39, background: 'transparent' }}
            onClick={() => setOpen(false)}
            data-testid="sidebar.switcher-backdrop"
          />
          <div style={listStyle} data-testid="sidebar.switcher-list">
            <p style={{ fontSize: 8, color: 'var(--muted-foreground)', padding: '4px 6px', textTransform: 'uppercase' }}>
              {t('shell.switcherHeading')}
            </p>
            {st.load.kind === 'error' && (
              <p style={{ fontSize: 9, color: 'var(--warning)', padding: '4px 6px' }}>{st.load.message}</p>
            )}
            {others.length === 0 && st.load.kind !== 'loading' && (
              <p style={{ fontSize: 9, color: 'var(--muted-foreground)', padding: '4px 6px' }}>
                {t('shell.noOtherProjects')}
              </p>
            )}
            {others.map((p) => (
              <button
                key={p.project_id}
                type="button"
                style={optionStyle}
                data-testid="sidebar.project-option"
                onClick={() => openProject(p.project_id)}
              >
                <strong style={{ fontSize: 11, fontWeight: 500 }}>
                  {p.name?.trim() ? p.name : shortId(p.project_id)}
                </strong>
                <span style={{ fontSize: 9, color: 'var(--muted-foreground)' }}>
                  {t('home.revision')} {p.revision}
                </span>
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
