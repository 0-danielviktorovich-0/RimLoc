// AI Translation (desktop-first mandate) — ONE screen, beginner-oriented
// modes: automatic translation (BYO key/local/mock through the shared
// engine), ChatGPT plan (OAuth, owner-consented), local models, the
// existing no-API chat batch, and history/review links. All real behavior
// rides the shared services ai_job + canonical apply; model output stays a
// DRAFT preview until explicitly accepted.
import { useEffect, useState } from 'react'
import { Sparkles, KeyRound, Monitor, MessagesSquare, CircleAlert, Play, Check } from 'lucide-react'
import { useProjectState } from '../lib/state/useProjectState'
import { clientInstance } from '../lib/client/instance'
import { t } from '../lib/i18n'
import type { AiJobStatusDto, ProviderInstanceSummaryDto } from '../lib/client/types'

type Mode = 'auto' | 'chatgpt' | 'byo' | 'local' | 'noapi'

/** Demo echo provider: only reachable in dev builds (Vite dev flag) — a
 *  production build never offers the fake echo as a "real" AI provider
 *  (review follow-up P1). The backend independently refuses mock without
 *  RIMLOC_ALLOW_MOCK_JOBS=1. */
const MOCK_ALLOWED: boolean = import.meta.env?.DEV === true

const MODES: { id: Mode; label: string; labelRu: string }[] = [
  { id: 'auto', label: 'Translate automatically', labelRu: 'Перевести автоматически' },
  { id: 'chatgpt', label: 'ChatGPT plan (no API key)', labelRu: 'Подписка ChatGPT (без ключа)' },
  { id: 'byo', label: 'Bring your API key', labelRu: 'Свой API-ключ' },
  { id: 'local', label: 'Local models', labelRu: 'Локальные модели' },
  { id: 'noapi', label: 'AI chat without API', labelRu: 'Чат без API' },
]

function providersForMode(mode: Mode): string[] {
  switch (mode) {
    case 'auto':
      return MOCK_ALLOWED ? ['mock', 'openai', 'anthropic', 'zai'] : ['openai', 'anthropic', 'zai']
    case 'byo':
      return ['openai', 'anthropic', 'zai']
    case 'local':
      return ['ollama']
    default:
      return []
  }
}

export function AiTranslate({ onBack }: { onBack: () => void }) {
  const st = useProjectState()
  const [mode, setMode] = useState<Mode>('auto')
  const [provider, setProvider] = useState(() => providersForMode('auto')[0] ?? '')
  const [instances, setInstances] = useState<ProviderInstanceSummaryDto[]>([])
  const [instanceId, setInstanceId] = useState('')
  const [model, setModel] = useState('')
  const [locale, setLocale] = useState('Russian')
  const [job, setJob] = useState<AiJobStatusDto | null>(null)
  // Per-row review: identities chosen for apply (starts empty — an
  // explicit pick, never a silent all-on).
  const [selected, setSelected] = useState<Set<string>>(new Set())
  // Full-scope acknowledgement for truncated previews: two-step by design.
  const [acknowledgeFull, setAcknowledgeFull] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const snap = st.snapshot
  const epoch = snap?.session_epoch
  // BYO runs configured ProvidersScreen instances (prov-*): the screen
  // lists them and the launch carries provider_instance_id — the backend
  // resolves preset/model/base_url and the stored key in-process.
  useEffect(() => {
    if (mode !== 'byo') return
    let alive = true
    clientInstance
      .getClient()
      .providerInstanceList()
      .then((res) => {
        if (!alive) return
        setInstances(res.instances)
        setInstanceId((cur) =>
          res.instances.some((i) => i.id === cur) ? cur : (res.instances[0]?.id ?? ''),
        )
      })
      .catch(() => {
        if (alive) setInstances([])
      })
    return () => {
      alive = false
    }
  }, [mode])

  const providerValidForMode =
    mode === 'byo' ? instanceId !== '' : providersForMode(mode).includes(provider)
  const canRun = Boolean(snap && epoch !== undefined && !busy && providerValidForMode)

  // Poll while running.
  useEffect(() => {
    if (!job || job.state !== 'running') return
    const timer = setInterval(() => {
      clientInstance.getClient().aiJobStatus(job.job_id)
        .then(setJob)
        .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
    }, 500)
    return () => clearInterval(timer)
  }, [job])

  const run = (): void => {
    if (!snap || epoch === undefined) return
    if (!providerValidForMode) return
    setBusy(true)
    setError(null)
    clientInstance.getClient().aiJobStart({
      project_id: snap.project_id,
      session_epoch: epoch,
      locale,
      provider_id: mode === 'byo' ? '' : provider,
      provider_instance_id: mode === 'byo' ? instanceId : undefined,
      model: mode === 'byo' || model.trim() === '' ? undefined : model.trim(),
      batch_char_budget: 6000
    })
      .then(setJob)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false))
  }

  const accept = (): void => {
    if (!job) return
    setBusy(true)
    clientInstance.getClient().aiJobAccept(job.job_id, epoch ?? 0, acknowledgeFull)
      .then((res) => { setJob(res); setBusy(false); setAcknowledgeFull(false) })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e))
        setBusy(false)
      })
  }

  const acceptSelected = (): void => {
    if (!job) return
    const chosen = [...selected]
    const rejected = job.rows
      .filter((r) => r.status === 'translated' && !selected.has(r.identity))
      .map((r) => r.identity)
    setBusy(true)
    clientInstance.getClient().aiJobAcceptSelected(job.job_id, epoch ?? 0, chosen, rejected)
      .then((res) => { setJob(res); setBusy(false) })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e))
        setBusy(false)
      })
  }

  const toggleSelected = (identity: string): void => {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(identity)) next.delete(identity)
      else next.add(identity)
      return next
    })
  }

  const cancel = (): void => {
    if (!job) return
    clientInstance.getClient().aiJobCancel(job.job_id)
      .then(setJob)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
  }

  return (
    <div className="page-content" data-testid="ai.root">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('ai.eyebrow')}</span>
          <h2>{t('ai.title')}</h2>
          <p>{t('ai.subtitle')}</p>
        </div>
        <button className="icon-btn" onClick={onBack} aria-label={t('ws.backHome')}>
          {t('ws.backHome')}
        </button>
      </div>

      {!snap && <p className="page-note" data-testid="ai.no-project">{t('ai.noProject')}</p>}

      <div className="ai-modes" role="radiogroup" aria-label={t('ai.modes')} data-testid="ai.modes">
        {MODES.map((m) => (
          <button
            key={m.id}
            role="radio"
            aria-checked={mode === m.id}
            className={`ai-mode-card ${mode === m.id ? 'active' : ''}`}
            data-testid={`ai.mode.${m.id}`}
            onClick={() => {
              setMode(m.id)
              // Reset on every switch (review follow-up P1): the provider
              // must be valid for the NEW mode — a stale demo selector
              // never submits into a real AI job.
              setProvider(providersForMode(m.id)[0] ?? '')
            }}
          >
            {m.id === 'auto' && <Sparkles size={14} />}
            {m.id === 'chatgpt' && <Sparkles size={14} />}
            {m.id === 'byo' && <KeyRound size={14} />}
            {m.id === 'local' && <Monitor size={14} />}
            {m.id === 'noapi' && <MessagesSquare size={14} />}
            <span>{t('uiLocale') === 'ru' ? m.labelRu : m.label}</span>
          </button>
        ))}
      </div>

      {error && (
        <div className="inline-warning text-destructive" role="alert" data-testid="ai.error">
          <CircleAlert size={15} /> {error}
        </div>
      )}

      {mode === 'noapi' && (
        <div data-testid="ai.noapi">
          <p className="page-note">{t('ai.noapiNote')}</p>
          <a className="btn-primary" href="#/chatbatch" data-testid="ai.open-chatbatch">
            {t('ai.openChatbatch')}
          </a>
        </div>
      )}

      {mode === 'chatgpt' && (
        <div data-testid="ai.chatgpt">
          <p className="page-note">{t('ai.chatgptNote')}</p>
          <button className="btn-primary" data-testid="ai.chatgpt-signin" disabled>
            {t('ai.chatgptSignin')}
          </button>
          <p style={{ fontSize: 10, color: 'var(--muted-foreground)' }}>{t('ai.chatgptPlanned')}</p>
        </div>
      )}

      {(mode === 'auto' || mode === 'byo' || mode === 'local') && (
        <div className="ai-form" data-testid="ai.form">
          {mode === 'byo' ? (
            <label className="field">
              <span>{t('ai.provider')}</span>
              <select
                value={instanceId}
                onChange={(e) => setInstanceId(e.target.value)}
                data-testid="ai.instance"
              >
                <option value="">{t('ai.byo.pick')}</option>
                {instances.map((i) => (
                  <option key={i.id} value={i.id}>
                    {i.label} · {i.preset} · {i.model}
                    {i.has_key ? '' : ` · ${t('ai.byo.noKey')}`}
                  </option>
                ))}
              </select>
              {instances.length === 0 && (
                <span style={{ fontSize: 10, color: 'var(--muted-foreground)' }} data-testid="ai.byo-empty">
                  {t('ai.byo.noInstances')}
                </span>
              )}
            </label>
          ) : (
            <>
              <label className="field">
                <span>{t('ai.provider')}</span>
                <select value={provider} onChange={(e) => setProvider(e.target.value)} data-testid="ai.provider">
                  {providersForMode(mode).map((p: string) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>{t('ai.model')}</span>
                <input
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  placeholder={t('ai.modelAuto')}
                  data-testid="ai.model"
                />
              </label>
            </>
          )}
          <label className="field">
            <span>{t('ai.locale')}</span>
            <select value={locale} onChange={(e) => setLocale(e.target.value)} data-testid="ai.locale">
              <option value="Russian">Русский</option>
              <option value="Ukrainian">Українська</option>
              <option value="German">Deutsch</option>
            </select>
          </label>
          <p style={{ fontSize: 10, color: 'var(--muted-foreground)' }}>{t('ai.privacyNotice')}</p>
          <button className="btn-primary" data-testid="ai.run" disabled={!canRun || busy} onClick={run}>
            <Play size={13} /> {t('ai.start')}
          </button>
        </div>
      )}

      {job && (
        <div className="ai-job" data-testid="ai.job">
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <strong data-testid="ai.job-state">{t(`ai.state.${job.state}`)}</strong>
            {job.state === 'running' && (
              <button className="icon-btn" data-testid="ai.cancel" onClick={cancel}>
                {t('ai.cancel')}
              </button>
            )}
            {job.state === 'preview_ready' && (
              <>
                {selected.size > 0 && (
                  <button
                    className="btn-primary"
                    data-testid="ai.accept-selected"
                    disabled={busy}
                    onClick={acceptSelected}
                  >
                    <Check size={13} /> {t('ai.acceptSelected', { count: selected.size })}
                  </button>
                )}
                {job.rows_truncated && !acknowledgeFull ? (
                  <button data-testid="ai.ack-full" disabled={busy} onClick={() => setAcknowledgeFull(true)}>
                    {t('ai.ackFull', { shown: job.rows.length, total: job.translated })}
                  </button>
                ) : (
                  <button className="btn-primary" data-testid="ai.accept" disabled={busy} onClick={accept}>
                    <Check size={13} /> {t('ai.accept')}
                  </button>
                )}
              </>
            )}
          </div>
          <div style={{ fontSize: 10, color: 'var(--muted-foreground)' }} data-testid="ai.counters">
            {t('ai.counters', {
              translated: job.translated,
              failed: job.failed,
              excluded: job.excluded_notranslate,
              total: job.total
            })}
          </div>
          {job.error && (
            <div className="inline-warning text-destructive" role="alert" data-testid="ai.job-error">
              {job.error}
            </div>
          )}
          {job.rows.length > 0 && (
            <div style={{ maxHeight: 300, overflowY: 'auto', marginTop: 6 }} data-testid="ai.rows">
              {job.rows.map((r) => (
                <div key={r.identity} data-testid="ai.row" style={{ borderBottom: '1px solid var(--border)', padding: '3px 0' }}>
                  <div style={{ display: 'flex', gap: 6, alignItems: 'baseline' }}>
                    {r.status === 'translated' && job.state === 'preview_ready' ? (
                      <input
                        type="checkbox"
                        data-testid={`ai.select.${r.identity}`}
                        checked={selected.has(r.identity)}
                        onChange={() => toggleSelected(r.identity)}
                      />
                    ) : null}
                    <code style={{ fontSize: 10 }}>{r.identity}</code>
                    <span className="status-label" style={{ fontSize: 10 }}>{r.status}</span>
                  </div>
                  <div style={{ fontSize: 11 }}>
                    {r.source} → {r.target ?? '—'}
                  </div>
                </div>
              ))}
              {job.rows_truncated && (
                <div className="inline-warning" role="status" data-testid="ai.truncated">
                  {t('ai.truncated', { shown: job.rows.length, total: job.translated })}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
