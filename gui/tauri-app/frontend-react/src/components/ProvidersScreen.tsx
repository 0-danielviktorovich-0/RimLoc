// Providers screen (R1 → provider/settings parity): provider INSTANCES over
// the live contract (provider_instance_list/upsert/delete/validate) — the
// Glossary/TM pattern. The API key is typed once into the form and goes
// straight to the OS keychain; this screen never displays it and the list
// only ever reports has_key. "Validate configuration" is an honest OFFLINE
// form check (typed invalid_config problems) — no network call, no paid API.
import { useEffect, useState, useSyncExternalStore } from 'react'
import { Check, CircleAlert, Play, Wifi, WifiOff, LoaderCircle, Trash2 } from 'lucide-react'
import {
  providers,
  PROVIDER_TEMPLATES,
  errorText,
  type ProvidersState,
  type ProviderConnection,
} from '../lib/state/providers'
import type { ProviderInstanceSummaryDto } from '../lib/client/types'
import { t } from '../lib/i18n'

function useProvidersState(): ProvidersState {
  return useSyncExternalStore(
    (l) => providers.subscribe(l),
    () => providers.getSnapshot(),
    () => providers.getSnapshot(),
  )
}

const CONNECTION_STYLE: Record<ProviderConnection, { cls: string; icon: typeof Check }> = {
  ready: { cls: 'text-success', icon: Check },
  not_configured: { cls: 'text-warning', icon: CircleAlert },
}

const CONNECTION_LABEL: Record<ProviderConnection, string> = {
  ready: t('prov.ready'),
  not_configured: t('prov.notConfigured'),
}

function connectionOf(p: ProviderInstanceSummaryDto): ProviderConnection {
  return p.has_key || p.local ? 'ready' : 'not_configured'
}

// New-instance form defaults per template (frontend knowledge only; the
// backend validates the real configuration).
const TEMPLATE_BY_PRESET = new Map(PROVIDER_TEMPLATES.map((tpl) => [tpl.preset, tpl]))

export function ProvidersScreen() {
  const st = useProvidersState()

  // Form state: null = "new provider" mode.
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [preset, setPreset] = useState('zai')
  const [label, setLabel] = useState('')
  const [model, setModel] = useState(TEMPLATE_BY_PRESET.get('zai')?.models[0] ?? '')
  const [baseUrl, setBaseUrl] = useState(TEMPLATE_BY_PRESET.get('zai')?.baseUrl ?? '')
  const [local, setLocal] = useState(false)
  const [secret, setSecret] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [validateResult, setValidateResult] = useState<string[] | null>(null)

  useEffect(() => {
    void providers.reload()
  }, [])

  const applyTemplate = (next: string): void => {
    setPreset(next)
    const tpl = TEMPLATE_BY_PRESET.get(next)
    if (tpl) {
      setModel(tpl.models[0] ?? '')
      setBaseUrl(tpl.baseUrl)
      setLocal(tpl.local)
    }
    setValidateResult(null)
  }

  const selected = st.instances.find((p) => p.id === selectedId) ?? null
  const counts = {
    ready: st.instances.filter((p) => connectionOf(p) === 'ready').length,
    offline: st.instances.filter((p) => connectionOf(p) === 'not_configured').length,
  }

  const run = async (fn: () => Promise<void>): Promise<void> => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await fn()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const startEdit = (p: ProviderInstanceSummaryDto): void => {
    setSelectedId(p.id)
    setPreset(p.preset)
    setLabel(p.label)
    setModel(p.model)
    setBaseUrl(p.base_url ?? '')
    setLocal(p.local)
    setSecret('')
    setError(null)
    setValidateResult(null)
  }

  const startNew = (): void => {
    setSelectedId(null)
    applyTemplate(preset)
    setLabel('')
    setSecret('')
    setError(null)
    setValidateResult(null)
  }

  const save = (): Promise<void> =>
    run(async () => {
      const req = {
        ...(selected ? { instance_id: selected.id } : {}),
        preset,
        ...(label.trim() ? { label: label.trim() } : {}),
        model: model.trim(),
        ...(baseUrl.trim() ? { base_url: baseUrl.trim() } : {}),
        ...(secret.trim() ? { secret: secret.trim() } : {}),
        local,
      }
      await providers.upsert(req)
      setSecret('')
      setValidateResult(null)
    })

  const remove = (p: ProviderInstanceSummaryDto): Promise<void> =>
    run(async () => {
      await providers.remove(p.id)
      if (selectedId === p.id) startNew()
    })

  const validate = (): Promise<void> =>
    run(async () => {
      const res = await providers.validate({
        preset,
        model: model.trim(),
        ...(baseUrl.trim() ? { base_url: baseUrl.trim() } : {}),
        local,
        has_key: selected?.has_key || secret.trim().length > 0,
      })
      setValidateResult(res.ok ? [] : res.problems)
    })

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('prov.eyebrow')}</span>
          <h2>{t('prov.title')}</h2>
          <p>{t('prov.subtitle')}</p>
        </div>
        <span className="safe-label">
          {counts.ready} <Wifi size={14} /> · {counts.offline} <WifiOff size={14} />
        </span>
      </div>

      {!st.loaded && !st.error && (
        <p className="page-note">{t('prov.busy')}</p>
      )}
      {(error || st.error) && (
        <div className="inline-warning" role="alert" data-testid="prov.error">
          <span>{error ?? st.error}</span>
        </div>
      )}

      <div className="tools-grid">
        <div className="tool-catalog" data-testid="prov.catalog">
          {st.instances.map((p) => {
            const conn = connectionOf(p)
            const S = CONNECTION_STYLE[conn]
            const Icon = S.icon
            return (
              <button
                key={p.id}
                className={selectedId === p.id ? 'active' : ''}
                onClick={() => startEdit(p)}
                data-testid={`prov.card.${p.id}`}
              >
                <span>
                  <strong>{p.label || p.preset}</strong>
                  <small>{p.local ? t('prov.local') : t('prov.cloud')}</small>
                </span>
                <span className={`status-label ${S.cls}`} title={CONNECTION_LABEL[conn]}>
                  <Icon size={13} />
                </span>
              </button>
            )
          })}
          <button
            className={selectedId === null ? 'active' : ''}
            onClick={startNew}
            data-testid="prov.card.new"
          >
            <span>
              <strong>{t('prov.newProvider')}</strong>
              <small>{preset}</small>
            </span>
          </button>
        </div>

        <aside className="tool-settings" data-testid="prov.detail">
          <span className="eyebrow">{selected ? selected.id : 'new'}</span>
          <h3>{selected ? t('prov.editProvider') : t('prov.newProvider')}</h3>

          <label className="field">
            <span>{t('prov.preset')}</span>
            <select
              value={preset}
              onChange={(e) => applyTemplate(e.target.value)}
              data-testid="prov.preset"
            >
              {PROVIDER_TEMPLATES.map((tpl) => (
                <option key={tpl.preset} value={tpl.preset}>
                  {tpl.preset}
                </option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>{t('prov.label')}</span>
            <input
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder={preset}
              data-testid="prov.label"
            />
          </label>

          <label className="field">
            <span>{t('prov.model')}</span>
            <input
              value={model}
              onChange={(e) => setModel(e.target.value)}
              list="prov-model-options"
              data-testid={`prov.model.${selected?.id ?? 'new'}`}
            />
            <datalist id="prov-model-options">
              {(TEMPLATE_BY_PRESET.get(preset)?.models ?? []).map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </label>

          <label className="field">
            <span>{t('prov.baseUrl')}</span>
            <input
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              data-testid={`prov.baseurl.${selected?.id ?? 'new'}`}
            />
          </label>

          <label className="field">
            <span>
              {t('prov.key')} —{' '}
              {selected?.has_key ? (
                <strong className="text-success">{t('prov.keyStored')}</strong>
              ) : (
                <strong className="text-warning">{t('prov.keyMissing')}</strong>
              )}
            </span>
            <input
              type="password"
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              placeholder={t('prov.keyPlaceholder')}
              autoComplete="off"
              data-testid={`prov.secret.${selected?.id ?? 'new'}`}
            />
          </label>

          <label className="field">
            <span>
              <input
                type="checkbox"
                checked={local}
                onChange={(e) => setLocal(e.target.checked)}
                data-testid="prov.local"
              />{' '}
              {t('prov.local')}
            </span>
          </label>

          <div className="detail-actions">
            <button disabled={busy} onClick={() => void save()} data-testid={`prov.save.${selected?.id ?? 'new'}`}>
              {busy ? <LoaderCircle className="spin" size={14} /> : <Check size={14} />}
              {t('prov.save')}
            </button>
            <button disabled={busy} onClick={() => void validate()} data-testid={`prov.test.${selected?.id ?? 'new'}`}>
              <Play size={14} />
              {t('prov.test')}
            </button>
            {selected && (
              <button
                disabled={busy}
                onClick={() => void remove(selected)}
                title={t('prov.deleteNote')}
                data-testid={`prov.delete.${selected.id}`}
              >
                <Trash2 size={14} />
                {t('prov.delete')}
              </button>
            )}
          </div>

          {selected && (
            <div className="capability-row">
              <span>{t('prov.status')}</span>
              <strong
                className={CONNECTION_STYLE[connectionOf(selected)].cls}
                style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}
              >
                {(() => {
                  const S = CONNECTION_STYLE[connectionOf(selected)]
                  const Icon = S.icon
                  return <Icon size={13} />
                })()}
                {CONNECTION_LABEL[connectionOf(selected)]}
              </strong>
            </div>
          )}

          {validateResult !== null && (
            <div
              className={validateResult.length === 0 ? 'capability-row text-success' : 'inline-warning'}
              role={validateResult.length === 0 ? 'status' : 'alert'}
              data-testid="prov.validate-result"
            >
              {validateResult.length === 0 ? (
                <span>{t('prov.validateOk')}</span>
              ) : (
                <span>
                  {t('prov.validateBad')}
                  <ul>
                    {validateResult.map((p, i) => (
                      <li key={i}>{p}</li>
                    ))}
                  </ul>
                </span>
              )}
            </div>
          )}

          <p className="page-note">{t('prov.mockNote')}</p>
        </aside>
      </div>
    </div>
  )
}
