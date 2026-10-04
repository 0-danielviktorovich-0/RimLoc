// Self-localization (R1, journey J8): «Help translate RimLoc» — the app's
// own UI catalog opens as an ORDINARY project (mandate §38: same editor,
// same review, same build surface — adapter-specific only in contribution).
// Implementation: resolve the bundled catalog dir → createContractProject →
// workspace. The contribution builder lives on the Build/Export surface.
import { useState } from 'react'
import { useProjectState } from '../lib/state/useProjectState'
import { ArrowRight, BookOpen, Check } from 'lucide-react'
import { clientInstance } from '../lib/client/instance'
import { projectStore } from '../lib/state/project'
import { contractErrorText as contractErrorTextStatic } from '../lib/client/messagesError'
import { t } from '../lib/i18n'

export function Selfloc({ onOpen }: { onOpen: () => void }) {
  const [state, setState] = useState<'idle' | 'resolving' | 'error'>('idle')
  const [error, setError] = useState<string | null>(null)
  const st = useProjectState()

  const alreadyOpen =
    st.snapshot !== null &&
    (st.summaries.find((x) => x.project_id === st.snapshot!.project_id)?.name ?? '').includes('RimLoc UI')

  const open = async (): Promise<void> => {
    setState('resolving')
    setError(null)
    try {
      const client = clientInstance.getClient()
      const dir = await client.selflocCatalogDir()
      const ok = await projectStore.createContractProject(dir)
      if (!ok) throw new Error(projectStore.lastErrorText ?? 'selfloc create failed')
      // Stamp the display name the way the Svelte lane did (mandate D:
      // the project IS "RimLoc UI (en)").
      onOpen()
    } catch (e) {
      setError(
        e && typeof e === 'object' && 'code' in e
          ? contractErrorTextStatic((e as { code?: string }).code ?? 'internal', e instanceof Error ? e.message : String(e))
          : e instanceof Error
            ? e.message
            : String(e),
      )
      setState('error')
    }
  }

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('selfloc.eyebrow')}</span>
          <h2>{t('selfloc.title')}</h2>
          <p>{t('selfloc.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert" data-testid="selfloc.error">
          <span>{error}</span>
        </div>
      )}

      <div className="project-storage">
        <BookOpen />
        <span>{t('selfloc.explain')}</span>
      </div>

      {!alreadyOpen ? (
        <button className="btn-primary" disabled={state === 'resolving'} onClick={() => void open()} data-testid="selfloc.open">
          {state === 'resolving' ? t('selfloc.opening') : t('selfloc.open')}
          <ArrowRight size={15} />
        </button>
      ) : (
        <div className="inline-success" role="status">
          <Check /> {t('selfloc.opened')}
        </div>
      )}
      <p className="page-note">{t('selfloc.note')}</p>
    </div>
  )
}

