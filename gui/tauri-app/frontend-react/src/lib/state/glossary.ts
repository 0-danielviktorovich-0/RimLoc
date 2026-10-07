// Glossary cache store (§13 Terms-таб): список терминов проекта через
// project_glossary, загружается ONCE на проект и перечитывается только когда
// сдвинулась ревизия проекта (мутации глоссария — persist-before-ack и
// поднимают ревизию; экран Глоссария после них пере-усыновляет снапшот).
// Framework-neutral, как project.ts: биндинг —
// useSyncExternalStore(glossaryStore.subscribe, glossaryStore.getState).
import type { GlossaryTermDto } from '../client/types'
import { clientInstance } from '../client/instance'

export interface GlossaryCacheState {
  /** Проект, на которого закэширован список; null = ещё не грузили. */
  projectId: string | null
  /** Ревизия проекта на момент загрузки списка. */
  revision: number | null
  terms: GlossaryTermDto[]
  busy: boolean
  error: string | null
}

let state: GlossaryCacheState = {
  projectId: null,
  revision: null,
  terms: [],
  busy: false,
  error: null,
}

const listeners = new Set<() => void>()

function set(patch: Partial<GlossaryCacheState>): void {
  state = { ...state, ...patch }
  listeners.forEach((l) => l())
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function getState(): GlossaryCacheState {
  return state
}

/** Сериальный токен: ответ устаравшей загрузки не затирает свежую. */
let loadToken = 0

export const glossaryStore = {
  /** Идемпотентная загрузка: no-op, когда тот же проект+ревизия уже в кэше
   *  или грузится; иначе один fetch. Ошибка НЕ ретраится автоматически —
   *  следующий вызов (новая ревизия / вернувшийся воркспейс) повторит. */
  ensureLoaded(projectId: string, sessionEpoch: number, revision: number): void {
    if (state.busy) return
    if (state.projectId === projectId && state.revision === revision && !state.error) return
    const token = ++loadToken
    set({ busy: true, error: null })
    clientInstance
      .getClient()
      .glossaryList(projectId, sessionEpoch)
      .then((terms) => {
        if (token !== loadToken) return
        set({ projectId, revision, terms, busy: false, error: null })
      })
      .catch((e: unknown) => {
        if (token !== loadToken) return
        set({ busy: false, error: e instanceof Error ? e.message : String(e) })
      })
  },

  /** Смена/закрытие проекта: кэш чужого проекта не показываем. */
  reset(): void {
    loadToken++
    set({ projectId: null, revision: null, terms: [], busy: false, error: null })
  },
}
