// Language Manager screen (§32): CRUD user-defined languages.
// Builtin languages read-only; user languages persist to localStorage.
import { useState } from 'react'
import { Plus, Trash2 } from 'lucide-react'
import { t } from '../lib/i18n'
import { useLanguageManager } from '../lib/languages/manager'

export function LanguageManager() {
  const { allLanguages, addUser, removeUser } = useLanguageManager()
  const [newId, setNewId] = useState('')
  const [newDisplay, setNewDisplay] = useState('')
  const [newNative, setNewNative] = useState('')
  const [error, setError] = useState<string | null>(null)

  return (
    <div className="page-content narrow-page">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{t('lm.eyebrow')}</span>
          <h2>{t('lm.title')}</h2>
          <p>{t('lm.subtitle')}</p>
        </div>
      </div>

      {error && (
        <div className="inline-warning" role="alert">
          <span>{error}</span>
        </div>
      )}

      <div className="glossary-table" data-testid="lm.table">
        <div className="glossary-head">
          <span>{t('lm.localeId')}</span>
          <span>{t('lm.displayName')}</span>
          <span>{t('lm.origin')}</span>
          <span />
        </div>
        {allLanguages.map((l) => (
          <div key={l.localeId} data-testid={`lm.row.${l.localeId}`}>
            <code>{l.localeId}</code>
            <span>{l.displayName}</span>
            <span className="locale">{l.builtin ? t('lm.builtin') : t('lm.user')}</span>
            {!l.builtin && (
              <button
                className="icon-btn"
                aria-label={`${t('gl.delete')} ${l.localeId}`}
                onClick={() => removeUser(l.localeId)}
              >
                <Trash2 size={14} />
              </button>
            )}
          </div>
        ))}
      </div>

      <form
        className="add-term"
        onSubmit={(e) => {
          e.preventDefault()
          const err = addUser(newId, newDisplay, newNative)
          if (err) setError(err)
          else { setNewId(''); setNewDisplay(''); setNewNative('') }
        }}
      >
        <input
          required
          placeholder={t('lm.placeholderId')}
          value={newId}
          onChange={(e) => setNewId(e.target.value)}
          aria-label={t('lm.placeholderId')}
          data-testid="lm.add-id"
        />
        <input
          required
          placeholder={t('lm.placeholderDisplay')}
          value={newDisplay}
          onChange={(e) => setNewDisplay(e.target.value)}
          aria-label={t('lm.placeholderDisplay')}
          data-testid="lm.add-display"
        />
        <input
          required
          placeholder={t('lm.placeholderNative')}
          value={newNative}
          onChange={(e) => setNewNative(e.target.value)}
          aria-label={t('lm.placeholderNative')}
          data-testid="lm.add-native"
        />
        <button type="submit" className="btn-primary" data-testid="lm.add-submit">
          <Plus /> {t('gl.add')}
        </button>
      </form>
    </div>
  )
}
