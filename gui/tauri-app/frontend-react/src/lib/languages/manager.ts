// Language Manager (§32): CRUD user-defined languages from the registry.
// Builtin languages are read-only; user languages persist to localStorage.
// Generic-fallback invariant preserved: unknown locales never reject.
import { useState, useCallback, useMemo } from 'react'
import { BUILTIN_LANGUAGES, genericDisplayName, isValidLocaleId } from './registry'

export interface UserLanguage {
  localeId: string
  displayName: string
  nativeName: string
  rimworldFolder: string
}

const LM_KEY = 'rimloc.languages.user.v1'

export function loadUserLanguages(): UserLanguage[] {
  try {
    const raw = localStorage.getItem(LM_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw) as UserLanguage[]
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

export function saveUserLanguages(list: UserLanguage[]): void {
  localStorage.setItem(LM_KEY, JSON.stringify(list))
}

/** Refusal codes returned by addUser — the manager never carries user-facing
 *  text (EN literals rendered raw in the RU UI was an audit finding); the
 *  component maps a code through t() (`lm.err.<code>`). */
export type LMAddError = 'empty' | 'invalid' | 'duplicate-builtin' | 'duplicate-user'

export function useLanguageManager() {
  const [user, setUser] = useState<UserLanguage[]>(loadUserLanguages)

  const addUser = useCallback((localeId: string, displayName: string, nativeName: string): LMAddError | null => {
    const trimmed = localeId.trim()
    if (!trimmed) return 'empty'
    // Acceptance MUST-FIX: «1пробел плохой код!» принимался — валидатор формы
    // в React-лейне не вызывался (порт из Svelte registry.ts:246).
    if (!isValidLocaleId(trimmed)) return 'invalid'
    if (BUILTIN_LANGUAGES.some((l) => l.localeId === trimmed)) return 'duplicate-builtin'
    if (user.some((l) => l.localeId === trimmed)) return 'duplicate-user'
    const entry: UserLanguage = {
      localeId: trimmed,
      displayName: displayName.trim() || genericDisplayName(trimmed),
      nativeName: nativeName.trim() || genericDisplayName(trimmed),
      rimworldFolder: trimmed,
    }
    const next = [...user, entry]
    setUser(next)
    saveUserLanguages(next)
    return null
  }, [user])

  const removeUser = useCallback((localeId: string): void => {
    const next = user.filter((l) => l.localeId !== localeId)
    setUser(next)
    saveUserLanguages(next)
  }, [user])

  /** U in CRUD (audit v2): rename a user language's display/native names.
   *  localeId and rimworldFolder are identity — immutable here. An empty
   *  input keeps the previous value (never blanks a stored name). */
  const updateUser = useCallback((localeId: string, displayName: string, nativeName: string): void => {
    const next = user.map((l) =>
      l.localeId === localeId
        ? {
            ...l,
            displayName: displayName.trim() || l.displayName,
            nativeName: nativeName.trim() || l.nativeName,
          }
        : l,
    )
    setUser(next)
    saveUserLanguages(next)
  }, [user])

  const allLanguages = useMemo(
    () => [
      ...BUILTIN_LANGUAGES.map((l) => ({
        localeId: l.localeId,
        displayName: l.displayName,
        nativeName: l.nativeName,
        builtin: true,
      })),
      ...user.map((l) => ({
        localeId: l.localeId,
        displayName: l.displayName,
        nativeName: l.nativeName,
        builtin: false,
      })),
    ],
    [user],
  )

  return { user, allLanguages, addUser, updateUser, removeUser }
}
