// Language Manager screen (§32): CRUD user-defined languages.
// Builtin languages read-only; user languages persist to localStorage.
// Audit v2: search over the registry (MUST-FIX) and inline rename (U in
// CRUD) for user languages. WDIO contract kept: lm.table / lm.row.<id>
// testids, the DELETE button stays the FIRST button of a user row
// (lm-acceptance/lm-restart click exactly it), builtin rows carry no
// <button> at all.
import { useState } from 'react'
import { Pencil, Plus, Save, Search, Trash2, X } from 'lucide-react'
import { t } from '../lib/i18n'
import { useLanguageManager, type LMAddError } from '../lib/languages/manager'

export function LanguageManager() {
  const { allLanguages, addUser, updateUser, removeUser } = useLanguageManager()
  const [newId, setNewId] = useState('')
  const [newDisplay, setNewDisplay] = useState('')
  const [newNative, setNewNative] = useState('')
  // Хранится КОД отказа (не готовый текст): manager не несёт user-facing
  // строк (EN-литералы в RU-UI — находка аудита), текст строится через t().
  const [error, setError] = useState<LMAddError | null>(null)
  // Поиск над реестром (audit v2 #1): localeId/displayName/nativeName.
  const [query, setQuery] = useState('')
  // Inline-редактирование (audit v2 #2): только пользовательские языки.
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editDisplay, setEditDisplay] = useState('')
  const [editNative, setEditNative] = useState('')

  const q = query.trim().toLowerCase()
  const visible = q
    ? allLanguages.filter((l) =>
        `${l.localeId} ${l.displayName} ${l.nativeName}`.toLowerCase().includes(q),
      )
    : allLanguages

  const startEdit = (localeId: string, displayName: string, nativeName: string): void => {
    setEditingId(localeId)
    setEditDisplay(displayName)
    setEditNative(nativeName)
  }
  const cancelEdit = (): void => setEditingId(null)
  const saveEdit = (): void => {
    if (!editingId) return
    updateUser(editingId, editDisplay, editNative)
    setEditingId(null)
  }

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
          <span>{t(`lm.err.${error}`)}</span>
        </div>
      )}

      <div className="search-field">
        <Search size={16} />
        <input
          placeholder={t('lm.searchPlaceholder')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label={t('lm.searchPlaceholder')}
          data-testid="lm.search"
        />
      </div>

      <div className="glossary-table" data-testid="lm.table">
        <div className="glossary-head">
          <span>{t('lm.localeId')}</span>
          <span>{t('lm.displayName')}</span>
          <span>{t('lm.origin')}</span>
          <span />
        </div>
        {visible.map((l) =>
          editingId === l.localeId ? (
            // Режим правки: id неизменен (identity), правятся только имена.
            <div key={l.localeId} data-testid={`lm.row.${l.localeId}`}>
              <code>{l.localeId}</code>
              <input
                value={editDisplay}
                onChange={(e) => setEditDisplay(e.target.value)}
                aria-label={t('lm.displayName')}
                data-testid="lm.edit-display"
              />
              <input
                value={editNative}
                onChange={(e) => setEditNative(e.target.value)}
                aria-label={t('lm.placeholderNative')}
                data-testid="lm.edit-native"
              />
              <span className="lm-row-actions">
                <button
                  className="icon-btn"
                  aria-label={t('lm.save')}
                  onClick={saveEdit}
                  data-testid="lm.edit-save"
                >
                  <Save size={14} />
                </button>
                <button
                  className="icon-btn"
                  aria-label={t('lm.cancel')}
                  onClick={cancelEdit}
                  data-testid="lm.edit-cancel"
                >
                  <X size={14} />
                </button>
              </span>
            </div>
          ) : (
            <div key={l.localeId} data-testid={`lm.row.${l.localeId}`}>
              <code>{l.localeId}</code>
              <span>{l.displayName}</span>
              <span className="locale">{l.builtin ? t('lm.builtin') : t('lm.user')}</span>
              {!l.builtin && (
                // Порядок кнопок ЗАФИКСИРОВАН контрактом WDIO: delete — первая
                // кнопка строки (спека кликает row.querySelector('button')).
                <span className="lm-row-actions">
                  <button
                    className="icon-btn"
                    aria-label={`${t('gl.delete')} ${l.localeId}`}
                    onClick={() => removeUser(l.localeId)}
                  >
                    <Trash2 size={14} />
                  </button>
                  <button
                    className="icon-btn"
                    aria-label={`${t('lm.edit')} ${l.localeId}`}
                    onClick={() => startEdit(l.localeId, l.displayName, l.nativeName)}
                    data-testid="lm.edit-start"
                  >
                    <Pencil size={14} />
                  </button>
                </span>
              )}
            </div>
          ),
        )}
        {visible.length === 0 && (
          <p className="page-note">{t('lm.noMatch')}</p>
        )}
      </div>

      <form
        className="add-term"
        onSubmit={(e) => {
          e.preventDefault()
          const err = addUser(newId, newDisplay, newNative)
          if (err) setError(err)
          // Успех гасит предыдущую ошибку (аудит: stale error после успеха).
          else { setError(null); setNewId(''); setNewDisplay(''); setNewNative('') }
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
