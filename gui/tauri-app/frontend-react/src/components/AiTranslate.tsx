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
import type { AiJobStatusDto } from '../lib/client/types'

type Mode = 'auto' | 'chatgpt' | 'byo' | 'local' | 'noapi'

const MODES: { id: Mode; label: string; labelRu: string }[] = [
  { id: 'auto', label: 'Translate automatically', labelRu: 'Перевести автоматически' },
  { id: 'chatgpt', label: 'ChatGPT plan (no API key)', labelRu: 'Подписка ChatGPT (без ключа)' },
  { id: 'byo', label: 'Bring your API key', labelRu: 'Свой API-ключ' },
  { id: 'local', label: 'Local models', labelRu: 'Локальные модели' },
  { id: 'noapi', label: 'AI chat without API', labelRu: 'Чат без API' },
]

const PROVIDERS_BY_MODE: Record<Mode, string[]> = {
  auto: ['mock', 'openai', 'anthropic', 'zai'],
  chatgpt: [],
  byo: ['openai', 'anthropic', 'zai'],
  local: ['ollama'],
  noapi: [],
}

export function AiTranslate({ onBack }: { onBack: () => void }) {
  const st = useProjectState()
  const [mode, setMode] = useState<Mode>('auto')
  const [provider, setProvider] = useState('mock')
  const [model, setModel] = useState('')
  const [locale, setLocale] = useState('Russian')
  const [job, setJob] = useState<AiJobStatusDto | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const snap = st.snapshot
  const epoch = snap?.session_epoch
  const canRun = Boolean(snap && epoch !== undefined && !busy)

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
    setBusy(true)
    setError(null)
    clientInstance.getClient().aiJobStart({
      project_id: snap.project_id,
      session_epoch: epoch,
      locale,
      provider_id: provider,
      model: model.trim() === '' ? undefined : model.trim(),
      batch_char_budget: 6000
    })
      .then(setJob)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false))
  }

  const accept = (): void => {
    if (!job) return
    setBusy(true)
    clientInstance.getClient().aiJobAccept(job.job_id, epoch ?? 0)
      .then((res) => { setJob(res); setBusy(false) })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e))
        setBusy(false)
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
            onClick={() => setMode(m.id)}
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
          <label className="field">
            <span>{t('ai.provider')}</span>
            <select value={provider} onChange={(e) => setProvider(e.target.value)} data-testid="ai.provider">
              {PROVIDERS_BY_MODE[mode].map((p) => (
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
              <button className="btn-primary" data-testid="ai.accept" disabled={busy} onClick={accept}>
                <Check size={13} /> {t('ai.accept')}
              </button>
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
                    <code style={{ fontSize: 9 }}>{r.identity}</code>
                    <span className="status-label" style={{ fontSize: 9 }}>{r.status}</span>
                  </div>
                  <div style={{ fontSize: 11 }}>
                    {r.source} → {r.target ?? '—'}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
