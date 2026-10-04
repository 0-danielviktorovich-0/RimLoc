// Providers screen (R1): AI provider templates + statuses. Honest mock —
// purely client-side statuses, no real backend connection (§42: demo states
// are labeled, never pretending to be live).
import { useSyncExternalStore, useState } from 'react'
import { Check, CircleAlert, Play, Wifi, WifiOff, LoaderCircle } from 'lucide-react'
import { providers, type ProviderData, type ProviderStatus } from '../lib/state/providers'
import { t } from '../lib/i18n'

function useProviders(): ProviderData[] {
  return useSyncExternalStore(
    (l) => providers.subscribe(l),
    () => providers.getSnapshot(),
    () => providers.getSnapshot(),
  )
}

const STATUS_STYLE: Record<ProviderStatus, { cls: string; icon: typeof Check }> = {
  connected: { cls: 'text-success', icon: Check },
  not_configured: { cls: 'text-warning', icon: CircleAlert },
  offline: { cls: 'text-destructive', icon: WifiOff },
  testing: { cls: 'text-info', icon: LoaderCircle },
}

const STATUS_LABEL: Record<ProviderStatus, string> = {
  connected: 'Подключён',
  not_configured: 'Не настроен',
  offline: 'Недоступен',
  testing: 'Проверяю…',
}

export function ProvidersScreen() {
  const list = useProviders()
  const [selected, setSelected] = useState<ProviderData['id']>('zai')
  const active = list.find((p) => p.id === selected) ?? list[0]

  const counts = {
    connected: list.filter((p) => p.status === 'connected').length,
    offline: list.filter((p) => p.status === 'offline').length,
  }

  return (
    <div className="page-content">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('prov.eyebrow')}</span>
          <h2>{t('prov.title')}</h2>
          <p>{t('prov.subtitle')}</p>
        </div>
        <span className="safe-label">
          {counts.connected} <Wifi size={14} /> · {counts.offline} <WifiOff size={14} />
        </span>
      </div>

      <div className="tools-grid">
        <div className="tool-catalog" data-testid="prov.catalog">
          {list.map((p) => (
            <button
              key={p.id}
              className={selected === p.id ? 'active' : ''}
              onClick={() => setSelected(p.id)}
              data-testid={`prov.card.${p.id}`}
            >
              <span>
                <strong>{t(p.nameKey)}</strong>
                <small>{p.privacy === 'local' ? t('prov.local') : t('prov.cloud')}</small>
              </span>
              <StatusDot status={p.status} />
            </button>
          ))}
        </div>

        {active && (
          <aside className="tool-settings" data-testid="prov.detail">
            <span className="eyebrow">{active.id}</span>
            <h3>{t(active.nameKey)}</h3>
            <p>{t(active.descKey)}</p>

            <label className="field">
              <span>{t('prov.model')}</span>
              <select
                value={active.model}
                onChange={(e) => providers.setModel(active.id, e.target.value)}
                data-testid={`prov.model.${active.id}`}
              >
                {active.models.map((m) => (
                  <option key={m}>{m}</option>
                ))}
              </select>
            </label>

            <label className="field">
              <span>{t('prov.baseUrl')}</span>
              <input value={active.baseUrl} onChange={(e) => providers.setBaseUrl(active.id, e.target.value)} data-testid={`prov.baseurl.${active.id}`} />
            </label>

            <div className="detail-actions">
              <button
                disabled={active.status === 'testing'}
                onClick={() => providers.test(active.id)}
                data-testid={`prov.test.${active.id}`}
              >
                {active.status === 'testing' ? <LoaderCircle className="spin" size={14} /> : <Play size={14} />}
                {t('prov.test')}
              </button>
            </div>

            <StatusRow status={active.status} />
            <p className="page-note">{t('prov.mockNote')}</p>
          </aside>
        )}
      </div>
    </div>
  )
}

function StatusDot({ status }: { status: ProviderStatus }) {
  const S = STATUS_STYLE[status] ?? STATUS_STYLE.not_configured
  const Icon = S.icon
  return (
    <span className={`status-label ${S.cls}`} title={STATUS_LABEL[status]}>
      <Icon size={13} />
    </span>
  )
}

function StatusRow({ status }: { status: ProviderStatus }) {
  const S = STATUS_STYLE[status] ?? STATUS_STYLE.not_configured
  const Icon = S.icon
  return (
    <div className="capability-row">
      <span>{t('prov.status')}</span>
      <strong className={S.cls} style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
        <Icon size={13} /> {STATUS_LABEL[status]}
      </strong>
    </div>
  )
}
