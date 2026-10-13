// Test-environment shims for the store unit lane. Node ≥22 ships an
// experimental `localStorage` that is DISABLED unless --localstorage-file is
// provided — its presence shadows happy-dom's implementation, so the store
// under test would see `window.localStorage === undefined`. An explicit
// in-memory polyfill keeps the lane deterministic and hermetic.
if (typeof window !== 'undefined' && !window.localStorage) {
  const store = new Map<string, string>()
  Object.defineProperty(window, 'localStorage', {
    value: {
      getItem: (key: string): string | null => (store.has(key) ? store.get(key)! : null),
      setItem: (key: string, value: string): void => {
        store.set(key, String(value))
      },
      removeItem: (key: string): void => {
        store.delete(key)
      },
      clear: (): void => {
        store.clear()
      },
      key: (index: number): string | null => [...store.keys()][index] ?? null,
      get length(): number {
        return store.size
      },
    },
  })
}

export {}
