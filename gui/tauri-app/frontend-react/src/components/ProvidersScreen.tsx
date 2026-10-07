// Providers screen (R1 → provider/settings parity, §7 F7.1): provider
// INSTANCES over the live contract (provider_instance_list/upsert/delete/
// validate/test) — the Glossary/TM pattern. The API key is typed once into
// the form and goes straight to the OS keychain; this screen never displays
// it and the list only ever reports has_key.
//
// TWO distinct checks, honestly named:
// - «Проверить настройки» — the OFFLINE form check (typed invalid_config
//   problems), no network, no paid API;
// - «Проверить подключение» — the REAL bounded probe (one tiny prompt
//   through contract_provider_instance_test); its typed outcome is the ONLY
//   thing that can claim green + Wifi (connected).
import { useEffect, useState, useSyncExternalStore } from 'react'
import {
  Check,
  CircleAlert,
  CircleDashed,
  CircleHelp,
  Clock,
  KeyRound,
  Play,
  Wifi,
  WifiOff,
  ServerCrash,
  LoaderCircle,
  Trash2,
} from 'lucide-react'
import {
  providers,
  PROVIDER_TEMPLATES,
  connectionOf,
  errorText,
  FORM_PROBE_TARGET,
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

// §7 F7.1: green + Wifi is RESERVED for connected. Everything else either
// carries an explicit failure colour or stays neutral (unverified).
const CONNECTION_STYLE: Record<ProviderConnection, { cls: string; icon: typeof Check; spinning?: boolean }> = {
  configured: { cls: '', icon: CircleDashed },
  connection_unknown: { cls: 'text-warning', icon: CircleHelp },
  testing: { cls: '', icon: LoaderCircle, spinning: true },
  connected: { cls: 'text-success', icon: Wifi },
  auth_failed: { cls: 'text-destructive', icon: KeyRound },
  network_failed: { cls: 'text-destructive', icon: WifiOff },
  model_not_found: { cls: 'text-destructive', icon: CircleAlert },
  rate_limited: { cls: 'text-destructive', icon: Clock },
  server_error: { cls: 'text-destructive', icon: ServerCrash },
  local_offline: { cls: 'text-destructive', icon: WifiOff },
}

const FAILED_CONNECTIONS: ProviderConnection[] = [
  'auth_failed',
  'network_failed',
  'model_not_found',
  'rate_limited',
  'server_error',
  'local_offline',
]

function connectionLabel(conn: ProviderConnection): string {
  return t(`prov.conn.${conn}`)
}

// New-instance form defaults per template (frontend knowledge only; the
// backend validates the real configuration).
const TEMPLATE_BY_PRESET = new Map(PROVIDER_TEMPLATES.map((tpl) => [tpl.preset, tpl]))

function StatusIcon({ conn, size = 13 }: { conn: ProviderConnection; size?: number }) {
  const S = CONNECTION_STYLE[conn]
  const Icon = S.icon
  return <Icon size={size} className={S.spinning ? 'spin' : undefined} />
}

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
  const formTarget = FORM_PROBE_TARGET(selectedId)
  // §7 F7.1 header counter: connected is counted SEPARATELY from
  // configured; failures get their own segment (never folded into either).
  const connections = new Map(st.instances.map((p) => [p.id, connectionOf(p, st.probes, st.probing)]))
  const counts = {
    connected: [...connections.values()].filter((c) => c === 'connected').length,
    configured: [...connections.values()].filter((c) => c === 'configured' || c === 'connection_unknown' || c === 'testing').length,
    failed: [...connections.values()].filter((c) => FAILED_CONNECTIONS.includes(c)).length,
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

  // «Проверить настройки» — the OFFLINE schema validation (renamed from
  // the old Test; same providerInstanceValidate contract call).
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

  // «Проверить подключение» — the REAL bounded probe. A typed probe-only
  // secret (just typed, not yet saved) crosses once; otherwise the backend
  // resolves the stored keychain key by instance id. Probe failures are
  // RESULTS (typed statuses) — they land in the store, not the error lane.
  const testConnection = (): Promise<void> =>
    run(async () => {
      await providers.testConnection(formTarget, {
        preset,
        model: model.trim(),
        ...(baseUrl.trim() ? { base_url: baseUrl.trim() } : {}),
        ...(secret.trim() ? { secret: secret.trim() } : {}),
        ...(selected ? { instance_id: selected.id } : {}),
      })
    })

  const formProbe = st.probes[formTarget] ?? null
  const formConnection = st.probing[formTarget]
    ? 'testing'
    : formProbe
      ? (foldProbeForForm(formProbe.status, local))
      : null

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('prov.eyebrow')}</span>
          <h2>{t('prov.title')}</h2>
          <p>{t('prov.subtitle')}</p>
        </div>
        <span
          className="safe-label"
          style={{ color: 'var(--muted-foreground)' }}
          data-testid="prov.counter"
        >
          <span className="text-success" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }} title={t('prov.counter.connected')}>
            {counts.connected} <Wifi size={14} />
          </span>
          {' · '}
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }} title={t('prov.counter.configured')}>
            {counts.configured} <CircleDashed size={14} />
          </span>
          {counts.failed > 0 && (
            <>
              {' · '}
              <span className="text-destructive" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }} title={t('prov.counter.failed')}>
                {counts.failed} <WifiOff size={14} />
              </span>
            </>
          )}
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
            const conn = connections.get(p.id) ?? 'configured'
            const S = CONNECTION_STYLE[conn]
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
                <span
                  className={`status-label ${S.cls}`}
                  title={connectionLabel(conn)}
                  data-testid={`prov.status-${p.id}`}
                >
                  <StatusIcon conn={conn} />
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
            <button
              disabled={busy || st.probing[formTarget] === true}
              onClick={() => void testConnection()}
              data-testid={`prov.connect.${selected?.id ?? 'new'}`}
            >
              {st.probing[formTarget] === true ? (
                <LoaderCircle className="spin" size={14} />
              ) : (
                <Wifi size={14} />
              )}
              {t('prov.testConnection')}
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

          {selected && (() => {
            const conn = connectionOf(selected, st.probes, st.probing)
            const S = CONNECTION_STYLE[conn]
            return (
              <div className="capability-row">
                <span>{t('prov.status')}</span>
                <strong
                  className={S.cls}
                  style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}
                >
                  <StatusIcon conn={conn} />
                  {connectionLabel(conn)}
                </strong>
              </div>
            )
          })()}

          {formConnection !== null && (
            <div
              className={formConnection === 'connected' ? 'capability-row text-success' : 'capability-row'}
              role="status"
              data-testid="prov.probe-result"
            >
              <span>{t('prov.status')}</span>
              <strong
                className={CONNECTION_STYLE[formConnection].cls}
                style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}
              >
                <StatusIcon conn={formConnection} />
                {connectionLabel(formConnection)}
              </strong>
            </div>
          )}

          {formProbe?.detail && formConnection !== 'connected' && (
            <p className="page-note" data-testid="prov.probe-detail">
              {t('prov.probeDetail')}: {formProbe.detail}
            </p>
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

// The form-side display fold mirrors the store rule: local && network_failed
// shows as local_offline (the local server on this machine is simply down).
function foldProbeForForm(status: ProviderConnection, local: boolean): ProviderConnection {
  return local && status === 'network_failed' ? 'local_offline' : status
}
